#!/usr/bin/env python3
# ============================================================================
# sitl_phase2_check.py — Phase 2 SITL verification harness (dev plan Phase 2
# exit criteria; assertion design in docs/phase2/phase2_px4_telemetry.md §A.4).
#
# Runs PX4 SITL **headless** with the built-in SIH simulator (airframe
# 10040_sihsim_quadx — no Gazebo/jMAVSim needed) and drives it through the
# interactive pxh shell over stdin/stdout, asserting:
#
#   1. all six cc_telemetry_* topics publish at AI_UART profile rates ±20%
#      (rates measured from the samples' own timestamps, not wall clock)
#   2. per-stream sequence strictly monotonic; px4_boot_id nonzero and
#      constant across the whole run
#   3. live parameter changes: CC_TEL_IMU_RATE 50→10→50 re-measured;
#      CC_TEL_PROFILE→MINIMAL freezes publish counts; →AI_UART restores
#   4. plausible values while flying (SIH): estimator_valid, |q|≈1, climb,
#      4 finite motor outputs, sane battery voltage, GPS 3D fix
#   5. print_status() reports all six streams with advancing counts
#   6. work_queue status shows the module on wq:lp_default (recorded)
#
# I/O DESIGN (hard-won; see docs Part B). Four PX4 behaviors shape this:
#   * `listener <topic> -n N` (multi-sample mode) is BROKEN on SITL: it
#     waits on plain poll() which never wakes for uORB handles, and its
#     stdin "abort key" check is `read(0,&c,1); if (ret) return;` — ANY
#     byte exits. Net effect: -n mode hangs until any input arrives. The
#     harness therefore uses only SINGLE-SHOT `listener <topic>` (a
#     synchronous print of the latest sample, no poll loop) and derives
#     rates as Δsequence/Δtimestamp between two samples — same sim-time
#     base on both axes, and it proves sequence==publish-count directly.
#   * Commands must be sent strictly one at a time (stdin bytes reach the
#     running command, not pxh).
#   * PX4 log lines (PX4_INFO/status) are unbuffered stderr, but listener
#     sample output is raw stdout — FULLY BUFFERED on a pipe, invisible to
#     a pipe-driven harness. The shell must run on a PSEUDO-TERMINAL.
#     The px4 daemon+client path fails the same way; pty+pxh is reliable.
#   * The SITL rootfs must be on a SPACE-FREE path (PX4 startup breaks on
#     this project's "UAV Project" directory), hence a system-temp workdir.
#
# Python stdlib only. Exit code 0 = all checks green.
# Usage: ./sitl_phase2_check.py [--px4 <PX4_DIR>] [--keep-rootfs]
# ============================================================================

import argparse
import math
import re
import shutil
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "common"))
from pxh import Pxh, parse_samples, sample, status_counts  # noqa: E402  (shared plumbing — see tools/common/pxh.py)

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_PX4 = (SCRIPT_DIR / "../../../../PX4-Autopilot-CCFC").resolve()

AIRFRAME = "10040"  # sihsim_quadx
TICK_HZ = 50        # AI_UART base tick (spec §4.2)
RATE_TOL = 0.20     # ±20% per the dev plan

results = []
log_lines = []


def log(msg):
    line = f"[{time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    log_lines.append(line)


def check(name, ok, detail=""):
    results.append((name, bool(ok), detail))
    log(f"{'PASS' if ok else 'FAIL'}  {name}{(' — ' + detail) if detail else ''}")
    return ok


def eff_rate(requested_hz, tick_hz=TICK_HZ):
    """The module's nearest-actual-rate divider rule (must mirror the C++)."""
    if requested_hz <= 0:
        return 0.0
    d_floor = max(1, tick_hz // requested_hz)
    d_ceil = d_floor + 1
    best = min((d_floor, d_ceil), key=lambda d: abs(tick_hz / d - requested_hz))
    return tick_hz / best


# (topic, requested_hz at AI_UART defaults)
STREAMS = [
    ("cc_telemetry_state", 25),
    ("cc_telemetry_imu", 50),
    ("cc_telemetry_power", 10),
    ("cc_telemetry_gps", 5),
    ("cc_telemetry_estimator", 10),
    ("cc_telemetry_actuator", 20),
]


def stream_rate_check(pxh, topic, requested_hz, label="", span=3.0):
    """Rate = Δsequence / Δtimestamp between two snapshots: sequence
    increments exactly once per publish and timestamp is the publish time,
    so this measures the true stream rate in sim time — and simultaneously
    proves the sequence counter tracks publishes."""
    expect = eff_rate(requested_hz)
    s1 = sample(pxh, topic)
    time.sleep(span)
    s2 = sample(pxh, topic)
    ok_fields = all(isinstance(s.get(k), int) for s in (s1, s2)
                    for k in ("timestamp", "sequence"))
    rate = 0.0
    if ok_fields and s2["timestamp"] > s1["timestamp"]:
        rate = (s2["sequence"] - s1["sequence"]) / ((s2["timestamp"] - s1["timestamp"]) / 1e6)
    ok = expect * (1 - RATE_TOL) <= rate <= expect * (1 + RATE_TOL)
    check(f"rate {topic}{label}", ok,
          f"measured {rate:.1f} Hz, expect {expect:.1f} ±20% (requested {requested_hz})")
    check(f"sequence/timestamp monotonic {topic}{label}",
          ok_fields and s2["sequence"] > s1["sequence"] and s2["timestamp"] > s1["timestamp"],
          f"seq {s1.get('sequence')}→{s2.get('sequence')}, ts {s1.get('timestamp')}→{s2.get('timestamp')}")
    return [s1, s2]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--px4", type=Path, default=DEFAULT_PX4)
    ap.add_argument("--keep-rootfs", action="store_true")
    args = ap.parse_args()

    if not (args.px4 / "build/px4_sitl_default/bin/px4").exists():
        print("error: px4 binary missing — run tools/phase2/build_px4.sh first", file=sys.stderr)
        return 2

    rootfs = Path(tempfile.mkdtemp(prefix="ccfc-sitl-rootfs-"))
    pxh = Pxh(args.px4, rootfs)
    boot_ids = set()

    try:
        log("=== Phase 2 SITL verification (SIH quadx, headless pxh) ===")
        pxh.start()
        time.sleep(8)  # sensors + EKF settle

        # ---- 1+2: rates + sequences at AI_UART defaults --------------------
        for topic, hz in STREAMS:
            for s in stream_rate_check(pxh, topic, hz):
                if "px4_boot_id" in s:
                    boot_ids.add(s["px4_boot_id"])

        check("px4_boot_id nonzero+constant", len(boot_ids) == 1 and 0 not in boot_ids,
              f"seen: {sorted(boot_ids)}")

        # ---- 3a: live IMU rate change ----------------------------------------
        pxh.run("param set CC_TEL_IMU_RATE 10")
        time.sleep(1)
        stream_rate_check(pxh, "cc_telemetry_imu", 10, label=" @10Hz")
        pxh.run("param set CC_TEL_IMU_RATE 50")
        time.sleep(1)
        stream_rate_check(pxh, "cc_telemetry_imu", 50, label=" @50Hz-restored")

        # ---- 3b: MINIMAL freezes counts, AI_UART restores --------------------
        pxh.run("param set CC_TEL_PROFILE 0")
        time.sleep(1)
        c1 = status_counts(pxh.run("cc_telemetry_publisher status"))
        time.sleep(4)
        c2 = status_counts(pxh.run("cc_telemetry_publisher status"))
        frozen = bool(c1) and c1 == c2
        check("MINIMAL profile freezes all streams", frozen, f"{c1} -> {c2}")
        pxh.run("param set CC_TEL_PROFILE 1")
        time.sleep(1)
        stream_rate_check(pxh, "cc_telemetry_state", 25, label=" restored")

        # ---- 4: plausibility while flying --------------------------------------
        armed = False
        for attempt in range(12):
            out = pxh.run("commander arm", timeout=15)
            if "Arming denied" not in out:
                status_out = pxh.run("commander status", timeout=10)
                if re.search(r"armed", status_out, re.IGNORECASE):
                    armed = True
                    break
            time.sleep(5)
        check("commander arm", armed, f"after {attempt + 1} attempt(s)")

        if armed:
            pxh.run("commander takeoff", timeout=15)
            time.sleep(12)

            s = sample(pxh, "cc_telemetry_state")
            if s:
                q = s.get("q", [])
                qn = math.sqrt(sum(x * x for x in q)) if len(q) == 4 else 0.0
                check("flight: estimator_valid", s.get("estimator_valid") == 1)
                check("flight: quaternion normalized", abs(qn - 1.0) < 0.05, f"|q|={qn:.3f}")
                z = s.get("position_ned", [0, 0, 0])[2]
                check("flight: climbed (NED z < -0.5)",
                      isinstance(z, (int, float)) and z < -0.5, f"z={z}")
                check("flight: armed bit in control_mode_flags",
                      (int(s.get("control_mode_flags", 0)) & 0x01) == 1)
            else:
                check("flight: state samples", False, "no samples")

            a = sample(pxh, "cc_telemetry_actuator")
            if a:
                outs = a.get("actuator_output", [])
                finite = [x for x in outs[:4] if isinstance(x, (int, float)) and math.isfinite(x)]
                nans = [x for x in outs[4:] if isinstance(x, float) and math.isnan(x)]
                check("flight: motor_count==4", a.get("motor_count") == 4, f"got {a.get('motor_count')}")
                check("flight: 4 finite outputs in [0..1]",
                      len(finite) == 4 and all(0.0 <= x <= 1.0 for x in finite), f"{outs[:4]}")
                check("flight: unused slots NaN", len(nans) == 4, f"{outs[4:]}")

            p = sample(pxh, "cc_telemetry_power")
            if p:
                v = p.get("voltage", float("nan"))
                check("flight: battery connected + sane voltage",
                      p.get("connected") == 1 and isinstance(v, float) and 6.0 < v < 60.0,
                      f"connected={p.get('connected')} V={v}")

            g = sample(pxh, "cc_telemetry_gps")
            if g:
                check("flight: GPS 3D fix + sats",
                      g.get("fix_type", 0) >= 3 and g.get("satellites_used", 0) >= 6,
                      f"fix={g.get('fix_type')} sats={g.get('satellites_used')}")
                check("flight: GPS position nonzero",
                      abs(g.get("lat", 0)) > 0 and abs(g.get("lon", 0)) > 0)

            e = sample(pxh, "cc_telemetry_estimator")
            if e:
                vr = e.get("velocity_test_ratio", float("nan"))
                ar = e.get("airspeed_test_ratio", 0.0)
                check("flight: velocity_test_ratio finite & sane",
                      isinstance(vr, (int, float)) and math.isfinite(vr) and 0.0 <= vr < 10.0, f"{vr}")
                check("flight: airspeed_test_ratio NaN on quad",
                      isinstance(ar, float) and math.isnan(ar), f"{ar}")

            pxh.run("commander land", timeout=15)
            time.sleep(10)

        # ---- 5: print_status -----------------------------------------------------
        s1_txt = pxh.run("cc_telemetry_publisher status")
        time.sleep(2)
        s2_txt = pxh.run("cc_telemetry_publisher status")
        log("print_status:\n" + s2_txt)
        check("print_status lists all six streams",
              all(n in s2_txt for n in ("state", "imu", "power", "gps", "estimator", "actuator")))
        c1, c2 = status_counts(s1_txt), status_counts(s2_txt)
        check("print_status counts advance", len(c1) == 6 and len(c2) == 6
              and all(c2[k] > c1[k] for k in c1), f"{c1} -> {c2}")
        m = re.search(r"px4_boot_id:\s*(\d+)", s2_txt)
        check("print_status boot id matches stream field",
              m is not None and boot_ids == {int(m.group(1))},
              f"status={m.group(1) if m else '?'} stream={sorted(boot_ids)}")

        # ---- 6: work queue placement ----------------------------------------------
        wq = pxh.run("work_queue status", timeout=15)
        lp = ""
        if "wq:lp_default" in wq:
            lp = wq.split("wq:lp_default", 1)[1].split("wq:")[0]
        check("module on wq:lp_default", "cc_telemetry_publisher" in lp)
        for ln in wq.splitlines():
            if "cc_telemetry_publisher" in ln:
                log("wq: " + ln.rstrip())

    except Exception as e:
        check("harness completed without exception", False, repr(e))
    finally:
        pxh.stop(SCRIPT_DIR / "px4_server_last.log")
        if args.keep_rootfs:
            log(f"rootfs kept at {rootfs}")
        else:
            shutil.rmtree(rootfs, ignore_errors=True)

    n_ok = sum(1 for _, ok, _ in results if ok)
    print("\n===== Phase 2 SITL check summary =====")
    for name, ok, detail in results:
        print(f"  {'PASS' if ok else 'FAIL'}  {name}" + (f" — {detail}" if detail else ""))
    print(f"===== {n_ok}/{len(results)} checks passed =====")
    (SCRIPT_DIR / "last_run.log").write_text("\n".join(log_lines) + "\n")
    return 0 if n_ok == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
