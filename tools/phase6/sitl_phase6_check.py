#!/usr/bin/env python3
# Phase 6 SITL scenario suite: the companion safety loop end to end.
#
# Drives the FC cc_safety_monitor with scripted CC_HEALTH_REPORTs from the
# release companiond (cc-health-tx --health-scenario) and asserts the monitor's
# response, read from companiond's --status-json "safety" object
# (state/action/reject/ack_seq — the monitor's CC_SAFETY_STATUS echoed back).
#
# One SITL boot. A single companiond runs a comprehensive OK -> CRITICAL ->
# recover timeline (no monitor reset / no restart), then short follow-up runs
# in the same SITL cover stale, garbage-immunity, and the disabled path.
#
# Coverage (the report-loop): OK->monitor-OK/None, escalation to CRITICAL with
# BLOCK_OFFBOARD (never Land on the ground), OK_COUNT-gated recovery, report
# ACK, staleness on a report gap, receiver-gauntlet immunity to bad/flooded
# reports, and the CC_MON_EN=0 disabled path. The *airborne* action decisions
# (CRITICAL/STALE while flying -> Land/Hold/RTL) are exhaustively covered by the
# 40 host unit tests (fork cc_policy_table_test.cpp); the SITL
# arm+takeoff+assert-one-Land scenario is a documented follow-up (the headless
# SIH config used across phases 2-6 is telemetry-focused, not flight-ready).
#
# Usage: ./sitl_phase6_check.py [--px4 DIR] [--keep]

import argparse
import importlib.util
import json
import os
import re
import shutil
import signal
import socket
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
CCFC_PORT = 24040
CCFC_PX4_LOCAL = 24540  # PX4 CCFC instance local port (where we inject garbage)
SCRATCH = Path(os.environ.get("CCFC_SCRATCH", "/Volumes/FRAGMENT/cctmp"))

ST_UNKNOWN, ST_OK, ST_WARN, ST_CRITICAL, ST_STALE = 0, 1, 2, 3, 4
ACT_NONE, ACT_BLOCK = 0, 2
REJECT_MONITOR_DISABLED = 6

results = []


def check(name, ok, detail=""):
    results.append((name, ok))
    print(f"[{time.strftime('%H:%M:%S')}] {'PASS' if ok else 'FAIL'}  {name}"
          + (f" — {detail}" if detail else ""), flush=True)
    return ok


class Companiond:
    def __init__(self, mission_root, scenario=None, env=None):
        self.mission_root = Path(mission_root)
        self.scenario = scenario
        self.env = env
        self.proc = None
        self.statuses = []
        self._lock = threading.Lock()

    def start(self):
        args = [str(COMPANIOND), "--status-json", "--udp-bind", f"0.0.0.0:{CCFC_PORT}",
                "--mission-root", str(self.mission_root)]
        if self.scenario:
            args += ["--health-scenario", str(self.scenario)]
        env = dict(os.environ)
        if self.env:
            env.update(self.env)
        self.proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                     text=True, bufsize=1, env=env)
        threading.Thread(target=self._reader, daemon=True).start()

    def _reader(self):
        for line in self.proc.stdout:
            line = line.strip()
            if line.startswith("{"):
                try:
                    st = json.loads(line)
                except json.JSONDecodeError:
                    continue
                with self._lock:
                    self.statuses.append(st)

    def latest_safety(self):
        with self._lock:
            for st in reversed(self.statuses):
                if "safety" in st and st["safety"]["seen"]:
                    return st["safety"]
        return None

    def wait_safety(self, predicate, timeout_s, poll=0.25):
        deadline = time.time() + timeout_s
        while time.time() < deadline:
            sf = self.latest_safety()
            if sf is not None and predicate(sf):
                return sf
            if self.proc.poll() is not None:
                return None
            time.sleep(poll)
        return None

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.send_signal(signal.SIGINT)
            try:
                self.proc.wait(timeout=6)
            except subprocess.TimeoutExpired:
                self.proc.kill()


def scenario_file(name, body):
    p = SCRATCH / f"p6-{name}-{os.getpid()}.toml"
    p.write_text(body)
    return p


def monitor_state(pxh):
    o = pxh.run("listener cc_safety_status 1", timeout=8)
    g = lambda k: int(re.search(rf"{k}:\s*(\d+)", o).group(1)) if re.search(rf"{k}:\s*(\d+)", o) else None
    return {"state": g("companion_state"), "action": g("action_taken"),
            "reject": g("reject_reason"), "ack": g("last_report_sequence")}


def inject_garbage(count=80):
    """Blast invalid CC_HEALTH_REPORTs at the PX4 CCFC port: bad source
    (wrong compid), out-of-range fields, and a flood — all must be dropped by
    the receiver gauntlet, never reaching the monitor."""
    spec = importlib.util.spec_from_file_location(
        "cc_dialect", REPO / "tools/phase3/generated/cc_dialect.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect(("127.0.0.1", CCFC_PX4_LOCAL))

    class Out:
        def write(self, b):
            s.send(bytes(b))

    bad_src = mod.MAVLink(Out(), srcSystem=1, srcComponent=200)   # wrong component
    good_src = mod.MAVLink(Out(), srcSystem=1, srcComponent=191)  # right source, bad fields
    for i in range(count):
        bad_src.cc_health_report_send(i, 9000 + i, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 100, 1)
        good_src.cc_health_report_send(i, 9500 + i, 0, 0, 0, 0, 0, 0, 0, 0, 99, 4, 200, 1)
        time.sleep(0.004)
    s.close()


def run_suite(pxh):
    # --- one companiond over a comprehensive OK -> CRITICAL -> recover run ----
    sc = scenario_file("main",
                       '[[event]]\nt_s=0\nseverity="ok"\n'
                       '[[event]]\nt_s=5\nseverity="critical"\naction="land"\nflags=["battery"]\nconfidence=95\n'
                       '[[event]]\nt_s=9\nseverity="ok"\n')
    c = Companiond(SCRATCH / f"p6m-main-{os.getpid()}", scenario=sc)
    c.start()

    sf = c.wait_safety(lambda s: s["state"] == ST_OK, 35)
    check("nominal: monitor reaches OK on OK reports", bool(sf), f"state {sf['state'] if sf else '?'}")
    if sf:
        check("nominal: action NONE at OK", sf["action"] == ACT_NONE, f"action {sf['action']}")
        check("nominal: report ACK advances", sf["ack_seq"] > 0, f"ack {sf['ack_seq']}")

    sf = c.wait_safety(lambda s: s["state"] == ST_CRITICAL, 15)
    check("critical: monitor escalates to CRITICAL", bool(sf), f"state {sf['state'] if sf else '?'}")
    if sf:
        check("critical: BLOCK_OFFBOARD on the ground (never auto-Land)",
              sf["action"] == ACT_BLOCK, f"action {sf['action']}")
        ack_at_crit = sf["ack_seq"]
        time.sleep(2)
        sf2 = c.latest_safety()
        check("critical: ACK keeps advancing (monitor acknowledging the 5 Hz repeat)",
              sf2 and sf2["ack_seq"] > ack_at_crit, f"{ack_at_crit} -> {sf2['ack_seq'] if sf2 else '?'}")

    sf = c.wait_safety(lambda s: s["state"] == ST_OK, 20)
    check("recovery: OK x CC_MON_OK_COUNT returns to OK", bool(sf), f"state {sf['state'] if sf else '?'}")

    # --- garbage immunity (same companiond still sending OK) ------------------
    inject_garbage()
    time.sleep(2)
    sf = c.latest_safety()
    check("garbage: monitor unmoved by bad/flooded reports (stays OK)",
          sf and sf["state"] == ST_OK, f"state {sf['state'] if sf else '?'}")
    c.stop()

    # --- stale: reports stop -> STALE after CC_MON_TMOUT_MS -------------------
    time.sleep(5)  # default 3000 ms + margin
    st = monitor_state(pxh)
    check("stale: monitor goes STALE after the report gap", st["state"] == ST_STALE, f"state {st['state']}")

    # resume OK -> recover out of STALE
    c2 = Companiond(SCRATCH / f"p6m-stale2-{os.getpid()}", scenario=scenario_file("ok", '[[event]]\nt_s=0\nseverity="ok"\n'))
    c2.start()
    sf = c2.wait_safety(lambda s: s["state"] == ST_OK, 25)
    check("stale: resumed OK reports recover to OK", bool(sf), f"state {sf['state'] if sf else '?'}")
    c2.stop()

    # --- monitor disabled ----------------------------------------------------
    pxh.run("param set CC_MON_EN 0", timeout=8)
    c3 = Companiond(SCRATCH / f"p6m-dis-{os.getpid()}",
                    scenario=scenario_file("crit", '[[event]]\nt_s=0\nseverity="critical"\naction="land"\n'))
    c3.start()
    sf = c3.wait_safety(lambda s: s["reject"] == REJECT_MONITOR_DISABLED, 20)
    check("disabled: reject_reason MONITOR_DISABLED when CC_MON_EN=0", bool(sf),
          f"reject {sf['reject'] if sf else '?'}")
    if sf:
        check("disabled: no action while disabled", sf["action"] == ACT_NONE, f"action {sf['action']}")
    c3.stop()
    pxh.run("param set CC_MON_EN 1", timeout=8)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--px4", type=Path, default=(REPO.parent.parent / "PX4-Autopilot-CCFC").resolve())
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    if not COMPANIOND.exists():
        print("build first: cargo build --release -p companiond")
        return 2

    SCRATCH.mkdir(parents=True, exist_ok=True)
    subprocess.run(["pkill", "-9", "-f", "bin/px4"], capture_output=True)
    for p in Path("/tmp").glob("px4*"):
        try:
            p.unlink()
        except OSError:
            pass
    time.sleep(1.5)

    rootfs = Path(tempfile.mkdtemp(prefix="ccfc-p6-", dir=str(SCRATCH)))
    pxh = Pxh(args.px4, rootfs)
    pxh.start()
    print(f"[{time.strftime('%H:%M:%S')}] SITL up", flush=True)
    time.sleep(2)

    try:
        run_suite(pxh)
    finally:
        pxh.stop()
        if not args.keep:
            shutil.rmtree(rootfs, ignore_errors=True)
            for p in SCRATCH.glob(f"p6-*-{os.getpid()}.toml"):
                p.unlink(missing_ok=True)
            for d in SCRATCH.glob(f"p6m-*-{os.getpid()}"):
                shutil.rmtree(d, ignore_errors=True)

    passed = sum(1 for _, ok in results if ok)
    print(f"\n===== Phase 6 SITL scenario suite: {passed}/{len(results)} passed =====")
    for name, ok in results:
        if not ok:
            print(f"  FAIL  {name}")
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
