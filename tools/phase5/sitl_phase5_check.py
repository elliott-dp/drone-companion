#!/usr/bin/env python3
# Phase 5 SITL verification: companiond writes a crash-safe mission dataset,
# log-inspect verifies it. Boots headless SIH SITL and the release companiond;
# then:
#   * clean mission  : short run, clean shutdown -> log-inspect CLEAN (exit 0)
#   * crash drill     : kill -9 companiond mid-mission -> log-inspect DIRTY
#                       (exit 1) with segment_00 sealed parts still readable;
#                       restart (resume) -> clean shutdown -> CLEAN, two
#                       segments linked under the SAME mission_id (spec §7)
#   * disk-full drill : shed thresholds forced above free space -> shed ladder
#                       trips, WARN set, imu/actuator drop, state never shed
#   --soak N          : one N-second mission, clean shutdown, log-inspect CLEAN
#                       with per-stream row counts within tolerance (exit gate)
#
# Usage: ./sitl_phase5_check.py [--px4 DIR] [--soak SECONDS]
#                               [--skip-crash] [--skip-diskfull] [--keep]

import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(REPO / "tools/common"))
from pxh import Pxh  # noqa: E402

COMPANIOND = REPO / "target/release/companiond"
LOG_INSPECT = REPO / "target/release/log-inspect"
CCFC_UDP_PORT = 24040
# large-free-space scratch on the FRAGMENT volume (mission_root must live on a
# volume with room above the 5 GiB default floor)
SCRATCH = Path(os.environ.get("CCFC_SCRATCH", "/Volumes/FRAGMENT/cctmp"))

EXPECT_HZ = {"state": 25, "imu": 50, "power": 10, "gps": 5, "estimator": 10, "actuator": 50 / 3}

results = []


def check(name, ok, detail=""):
    results.append((name, ok))
    mark = "PASS" if ok else "FAIL"
    ts = time.strftime("%H:%M:%S")
    print(f"[{ts}] {mark}  {name}" + (f" — {detail}" if detail else ""), flush=True)
    return ok


class Companiond:
    """Runs companiond --status-json; parses the status line history."""

    def __init__(self, mission_root, env=None, disk_floor=None):
        self.mission_root = Path(mission_root)
        self.env = env
        self.disk_floor = disk_floor
        self.proc = None
        self.statuses = []
        self._lock = threading.Lock()

    def start(self):
        args = [str(COMPANIOND), "--status-json",
                "--udp-bind", f"0.0.0.0:{CCFC_UDP_PORT}",
                "--mission-root", str(self.mission_root)]
        if self.disk_floor is not None:
            args += ["--disk-floor", str(self.disk_floor)]
        env = dict(os.environ)
        if self.env:
            env.update(self.env)
        self.proc = subprocess.Popen(args, stdout=subprocess.PIPE,
                                     stderr=subprocess.PIPE, text=True, bufsize=1, env=env)
        threading.Thread(target=self._reader, daemon=True).start()

    def _reader(self):
        for line in self.proc.stdout:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                st = json.loads(line)
            except json.JSONDecodeError:
                continue
            with self._lock:
                self.statuses.append(st)

    def latest(self):
        with self._lock:
            return self.statuses[-1] if self.statuses else None

    def wait_for(self, predicate, timeout_s, poll=0.25):
        deadline = time.time() + timeout_s
        while time.time() < deadline:
            st = self.latest()
            if st is not None and predicate(st):
                return st
            if self.proc.poll() is not None:
                return None
            time.sleep(poll)
        return None

    def sigint(self, timeout=15):
        """Clean shutdown — companiond seals + marks the mission complete."""
        if self.proc and self.proc.poll() is None:
            self.proc.send_signal(signal.SIGINT)
            try:
                self.proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    def kill9(self):
        """Crash — SIGKILL, no finalize (models a power loss / hard crash)."""
        if self.proc and self.proc.poll() is None:
            self.proc.send_signal(signal.SIGKILL)
            self.proc.wait(timeout=5)


def mission_dirs(root):
    return sorted(p for p in Path(root).glob("mission_*") if p.is_dir())


def inspect(mission_dir):
    r = subprocess.run([str(LOG_INSPECT), str(mission_dir), "--json"],
                       capture_output=True, text=True)
    try:
        report = json.loads(r.stdout)
    except json.JSONDecodeError:
        report = None
    return r.returncode, report


def free_bytes(path):
    s = os.statvfs(path)
    return s.f_bavail * s.f_frsize


# --------------------------------------------------------------------------


def clean_mission_check(run_secs):
    root = SCRATCH / f"p5-clean-{os.getpid()}"
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    comp = Companiond(root)
    comp.start()
    st = comp.wait_for(lambda s: s["link"] == "UP" and s["mission_id"] != 0, 30)
    check("clean: link UP + mission opened", bool(st),
          f"mission {st['mission_id']}" if st else "no status")
    comp.wait_for(lambda s: s["timesync"]["q"] == "LOCKED", 15)
    time.sleep(run_secs)
    comp.sigint()

    dirs = mission_dirs(root)
    if not check("clean: mission directory created", len(dirs) == 1):
        return root
    code, rep = inspect(dirs[0])
    check("clean: log-inspect CLEAN", rep and rep["verdict"] == "CLEAN" and code == 0,
          f"verdict {rep['verdict'] if rep else '?'} exit {code}")
    if rep:
        check("clean: complete + dialect/schema match",
              rep["complete"] and rep["dialect_hash_ok"] and rep["schema_version_ok"])
        check("clean: all six streams have rows",
              all(any(st["name"] == n and st["rows"] > 0 for seg in rep["segments"] for st in seg["streams"])
                  for n in EXPECT_HZ),
              f"total rows {rep['total_rows']}")
        check("clean: zero drops", rep["total_drops"] == 0)
        check("clean: raw capture present",
              any(seg["raw_present"] and seg["raw_frames"] > 0 for seg in rep["segments"]))
    return root


def crash_drill():
    root = SCRATCH / f"p5-crash-{os.getpid()}"
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    # Seal parts every 2 s so a kill a few seconds in leaves sealed parts on
    # disk (the whole point of the drill); the bounded loss is the <=2 s tail.
    env = {"CC_MISSION_LOG__FLUSH_SECS": "2"}

    # run A, then kill -9 while it is actively logging
    a = Companiond(root, env=env)
    a.start()
    # wait until the log task has sealed at least one part (log.parts > 0)
    st = a.wait_for(lambda s: s["mission_id"] != 0 and s["log"]["parts"] > 0, 30)
    mission_id = st["mission_id"] if st else 0
    check("crash: parts sealed before kill", bool(st),
          f"mission {mission_id}, {st['log']['parts'] if st else 0} parts")
    time.sleep(2)
    a.kill9()

    dirs = mission_dirs(root)
    code, rep = inspect(dirs[0]) if dirs else (3, None)
    check("crash: log-inspect DIRTY after kill -9",
          rep and rep["verdict"] == "DIRTY" and code == 1,
          f"verdict {rep['verdict'] if rep else '?'} exit {code}")
    if rep:
        check("crash: incomplete (not finalized)", not rep["complete"])
        check("crash: sealed parts still readable",
              rep["total_rows"] > 0 and rep["dialect_hash_ok"],
              f"{rep['total_rows']} rows survived")

    # run B: resume the SAME mission, new segment, clean shutdown
    b = Companiond(root, env=env)
    b.start()
    b.wait_for(lambda s: s["mission_id"] != 0 and s["streams"]["imu"]["n"] > 100, 30)
    time.sleep(3)
    b.sigint()

    code, rep = inspect(dirs[0])
    check("crash: resumed mission is CLEAN after restart",
          rep and rep["verdict"] == "CLEAN" and code == 0,
          f"verdict {rep['verdict'] if rep else '?'} exit {code}")
    if rep:
        check("crash: same mission_id resumed (spec §7)", rep["mission_id"] == mission_id,
              f"{mission_id} -> {rep['mission_id']}")
        check("crash: two segments linked in one manifest", len(rep["segments"]) == 2,
              f"{len(rep['segments'])} segments")


def diskfull_drill():
    root = SCRATCH / f"p5-diskfull-{os.getpid()}"
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    # Force the shed ladder above the real free space so it trips immediately,
    # without needing a small volume: set every shed threshold above free F,
    # keeping crit < bf < raw and resume > shed. Floor stays at 2 GiB (<= F).
    f = free_bytes(root)
    gib = 1024 ** 3
    env = {
        "CC_DISK__CRIT_LOW_BYTES": str(f + 100 * gib),
        "CC_DISK__CRIT_RESUME_BYTES": str(f + 110 * gib),
        "CC_DISK__BF_SHED_LOW_BYTES": str(f + 200 * gib),
        "CC_DISK__BF_RESUME_BYTES": str(f + 210 * gib),
        "CC_DISK__RAW_SHED_LOW_BYTES": str(f + 300 * gib),
        "CC_DISK__RAW_RESUME_BYTES": str(f + 310 * gib),
    }
    comp = Companiond(root, env=env, disk_floor=2 * gib)
    comp.start()
    st = comp.wait_for(lambda s: s["mission_id"] != 0, 30)
    check("diskfull: mission opened above 2 GiB floor", bool(st))
    # ladder polls on the 500 ms log tick; give it a few ticks
    st = comp.wait_for(lambda s: s["log"]["shed_stage"] == "SHED_CRIT", 10)
    check("diskfull: shed ladder reached SHED_CRIT", bool(st),
          st["log"]["shed_stage"] if st else "no CRIT")
    if st:
        check("diskfull: WARN flag set", st["log"]["warn"] is True)
    # let it run so imu/actuator accrue drops while state keeps landing
    time.sleep(6)
    st = comp.latest()
    if st:
        state_n = st["streams"]["state"]["n"]
        check("diskfull: state still written under pressure (never shed)", state_n > 0,
              f"state n={state_n}")
        check("diskfull: drops recorded", st["log"]["drops"] > 0 or st["log"]["raw_drops"] > 0,
              f"drops={st['log']['drops']} raw={st['log']['raw_drops']}")
    comp.sigint()
    dirs = mission_dirs(root)
    if dirs:
        code, rep = inspect(dirs[0])
        # drops make it DIRTY (recoverable, with a queryable ledger)
        check("diskfull: log-inspect DIRTY with drop ledger",
              rep and rep["verdict"] == "DIRTY" and rep["total_drops"] > 0,
              f"verdict {rep['verdict'] if rep else '?'} drops {rep['total_drops'] if rep else '?'}")


def soak(secs):
    root = SCRATCH / f"p5-soak-{os.getpid()}"
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    comp = Companiond(root)
    comp.start()
    st = comp.wait_for(lambda s: s["link"] == "UP" and s["mission_id"] != 0, 30)
    check("soak: mission opened", bool(st))
    comp.wait_for(lambda s: s["timesync"]["q"] == "LOCKED", 15)
    print(f"[{time.strftime('%H:%M:%S')}] soak: {secs}s unattended", flush=True)
    t_end = time.time() + secs
    while time.time() < t_end:
        time.sleep(30)
        st = comp.latest()
        if st is None or comp.proc.poll() is not None:
            break
    check("soak: companiond alive at end", comp.proc.poll() is None)
    comp.sigint(timeout=30)

    dirs = mission_dirs(root)
    code, rep = inspect(dirs[0]) if dirs else (3, None)
    check("soak: log-inspect CLEAN", rep and rep["verdict"] == "CLEAN" and code == 0,
          f"verdict {rep['verdict'] if rep else '?'} exit {code}")
    if rep:
        check("soak: complete + zero drops", rep["complete"] and rep["total_drops"] == 0)
        check("soak: segments rotated on the 30 min cap", len(rep["segments"]) >= 2,
              f"{len(rep['segments'])} segments over {secs}s")
        # per-stream row counts SUMMED across segments, within ±25% of Hz*secs
        # (the mission rotates every seg_cap_secs, so a stream's rows are split
        # across segments; sim jitter + boot warmup explain the tolerance).
        totals = {}
        for seg in rep["segments"]:
            for st in seg["streams"]:
                totals[st["name"]] = totals.get(st["name"], 0) + st["rows"]
        for name, hz in EXPECT_HZ.items():
            expect = hz * secs
            got = totals.get(name, 0)
            check(f"soak: {name} total rows in range",
                  expect * 0.5 <= got <= expect * 1.25,
                  f"{got} vs ~{expect:.0f}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--px4", type=Path,
                    default=(REPO.parent.parent / "PX4-Autopilot-CCFC").resolve())
    ap.add_argument("--soak", type=int, default=0)
    ap.add_argument("--skip-crash", action="store_true")
    ap.add_argument("--skip-diskfull", action="store_true")
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    if not COMPANIOND.exists() or not LOG_INSPECT.exists():
        print("build first: cargo build --release -p companiond -p log-inspect")
        return 2

    SCRATCH.mkdir(parents=True, exist_ok=True)
    subprocess.run(["pkill", "-9", "-f", "bin/px4"], capture_output=True)
    for p in Path("/tmp").glob("px4*"):
        try:
            p.unlink()
        except OSError:
            pass
    time.sleep(1)

    rootfs = Path(tempfile.mkdtemp(prefix="ccfc-p5-", dir=str(SCRATCH)))
    pxh = Pxh(args.px4, rootfs)
    pxh.start()
    print(f"[{time.strftime('%H:%M:%S')}] SITL up", flush=True)
    time.sleep(2)

    made = []
    try:
        made.append(clean_mission_check(run_secs=12))
        if not args.skip_crash:
            crash_drill()
        if not args.skip_diskfull:
            diskfull_drill()
        if args.soak:
            soak(args.soak)
    finally:
        pxh.stop()
        if not args.keep:
            shutil.rmtree(rootfs, ignore_errors=True)
            for root in SCRATCH.glob(f"p5-*-{os.getpid()}"):
                shutil.rmtree(root, ignore_errors=True)

    passed = sum(1 for _, ok in results if ok)
    total = len(results)
    print(f"\n===== Phase 5 SITL check: {passed}/{total} passed =====")
    for name, ok in results:
        if not ok:
            print(f"  FAIL  {name}")
    return 0 if passed == total else 1


if __name__ == "__main__":
    sys.exit(main())
