#!/usr/bin/env python3
# ============================================================================
# sitl_phase3_check.py — Phase 3 SITL verification harness (dev plan Phase 3
# exit criteria; design in docs/phase3/phase3_mavlink_link.md §A.6).
#
# Proves the MAVLink link both ways against a headless SIH SITL:
#   FC → CC : all six CC_TELEMETRY_* streams decoded EXTERNALLY over UDP
#             with pymavlink bindings generated from the pinned dialect
#             (rates from Δsequence/Δfc_timestamp_us, schema, boot id,
#             gap ratio, value plausibility)
#   CC → FC : the full validation gauntlet (spec §4.4) — valid reports
#             publish uORB; wrong-source / bad-schema / out-of-range /
#             duplicate-sequence / flood messages each increment exactly
#             their drop counter and publish NOTHING; CC_MISSION_CONTEXT
#             handshake accepted (and its mission_id echoes back on the
#             CC_TELEMETRY_STATE stream) or refused on wrong vehicle_id.
#
# Preflight guards: fork-vendored XML must byte-match the companion copy,
# and the generated Python bindings must decode the Phase 1 golden vectors
# 16/16 — so C, Rust, and Python agree on the wire before PX4 is involved.
#
# Runs under the pinned mavgen venv (re-execs itself if pymavlink is
# missing). Exit 0 = all checks green.
# Usage: ./sitl_phase3_check.py [--px4 <PX4_DIR>] [--keep-rootfs]
# ============================================================================

import argparse
import hashlib
import importlib.util
import math
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DEFAULT_PX4 = (REPO_ROOT.parent.parent / "PX4-Autopilot-CCFC").resolve()
VENV_PY = REPO_ROOT / "cc-dialect/.venv-mavgen/bin/python"

# --- self-bootstrap: the generated bindings import pymavlink internals ----
try:
    import pymavlink  # noqa: F401
except ImportError:
    if VENV_PY.exists() and os.environ.get("CCFC_REEXEC") != "1":
        os.environ["CCFC_REEXEC"] = "1"
        os.execv(str(VENV_PY), [str(VENV_PY), *sys.argv])
    print("error: pymavlink unavailable and mavgen venv missing "
          f"({VENV_PY}) — run cc-dialect/gen_c.sh once to create it", file=sys.stderr)
    sys.exit(2)

sys.path.insert(0, str(SCRIPT_DIR.parent / "common"))
from pxh import Pxh, sample  # noqa: E402

CCFC_UDP_PORT = 24040          # px4-rc.mavlink: -o $((24040+instance))
RATE_TOL = 0.20
GOLDEN_BIN = REPO_ROOT / "cc-dialect/golden/golden_frames.bin"
COMPANION_XML = REPO_ROOT / "cc-dialect/cc_dialect.xml"

# (msgid name, expected wire rate Hz — the uORB-decimation ceilings)
TELEMETRY_STREAMS = {
    "CC_TELEMETRY_STATE": 25.0,
    "CC_TELEMETRY_IMU": 50.0,
    "CC_TELEMETRY_POWER": 10.0,
    "CC_TELEMETRY_GPS": 5.0,
    "CC_TELEMETRY_ESTIMATOR": 10.0,
    "CC_TELEMETRY_ACTUATOR": 50.0 / 3.0,   # 20 requested -> divider 3 (doc D3)
}

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


def load_dialect():
    gen = SCRIPT_DIR / "generated/cc_dialect.py"
    if not gen.exists():
        log("generated bindings missing — running gen_python.sh")
        subprocess.run([str(SCRIPT_DIR / "gen_python.sh")], check=True,
                       capture_output=True)
    spec = importlib.util.spec_from_file_location("cc_dialect", gen)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# --- preflight -------------------------------------------------------------


def preflight(mod, px4_dir):
    fork_xml = px4_dir / "ccfc_dialect/cc_dialect.xml"
    sha_fork = hashlib.sha256(fork_xml.read_bytes()).hexdigest()
    sha_comp = hashlib.sha256(COMPANION_XML.read_bytes()).hexdigest()
    check("preflight: fork XML == companion XML", sha_fork == sha_comp,
          f"{sha_fork[:8]} vs {sha_comp[:8]}")

    mav = mod.MAVLink(None)
    decoded = []
    for b in GOLDEN_BIN.read_bytes():
        m = mav.parse_char(bytes([b]))
        if m is not None:
            decoded.append(m)
    check("preflight: python bindings decode golden vectors", len(decoded) == 16,
          f"{len(decoded)}/16")
    return int(sha_comp[:8], 16)  # CC_DIALECT_HASH (first 4 bytes, big-endian)


# --- UDP link --------------------------------------------------------------


class CcfcLink:
    """udpin endpoint on the CCFC mavlink instance's remote port."""

    def __init__(self, mod):
        self.mod = mod
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind(("127.0.0.1", CCFC_UDP_PORT))
        self.sock.settimeout(0.5)
        self.peer = None
        self.rx = mod.MAVLink(None)
        self.rx.robust_parsing = True

    def recv_msgs(self, duration):
        """Collect decoded messages for `duration` wall seconds."""
        out = []
        deadline = time.time() + duration
        while time.time() < deadline:
            try:
                data, addr = self.sock.recvfrom(65535)
            except socket.timeout:
                continue
            self.peer = addr
            msgs = self.rx.parse_buffer(data)
            if msgs:
                out.extend(msgs)
        return out

    def wait_link(self, timeout=60):
        """Wait for the first HEARTBEAT from the CCFC instance."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            for m in self.recv_msgs(1.0):
                if m.get_type() == "HEARTBEAT":
                    return True
        return False

    def sender(self, src_component):
        """A MAVLink encoder writing to the socket as (sysid 1, comp X)."""
        link = self

        class _Out:
            def write(self, buf):
                if link.peer:
                    link.sock.sendto(bytes(buf), link.peer)

        return self.mod.MAVLink(_Out(), srcSystem=1, srcComponent=src_component)


# --- counter parsing (from `mavlink status` via pxh) ------------------------

ACC_RE = re.compile(r"CCFC rx accepted: reports (\d+) context (\d+) diagnostic (\d+)")
DROP_RE = re.compile(r"CCFC rx dropped: bad_source (\d+) bad_schema (\d+) bad_range (\d+)"
                     r" dup_seq (\d+) flood (\d+) missed_reports (\d+)")


def ccfc_counters(pxh):
    """Sum the CCFC gauntlet counters across all mavlink instances (only the
    companion-link instance ever receives CC_* traffic, the rest stay 0)."""
    txt = pxh.run("mavlink status", timeout=20)
    keys = ("reports", "context", "diagnostic",
            "bad_source", "bad_schema", "bad_range", "dup_seq", "flood", "missed")
    tot = dict.fromkeys(keys, 0)
    for m in ACC_RE.finditer(txt):
        for k, v in zip(keys[:3], m.groups()):
            tot[k] += int(v)
    for m in DROP_RE.finditer(txt):
        for k, v in zip(keys[3:], m.groups()):
            tot[k] += int(v)
    return tot


def delta(before, after):
    return {k: after[k] - before[k] for k in before}


def expect_delta(name, got, **want):
    """Assert exactly the named counter movements; everything else zero."""
    exp = dict.fromkeys(got, 0)
    exp.update(want)
    check(name, got == exp,
          f"delta {dict((k, v) for k, v in got.items() if v or exp[k])} want "
          f"{dict((k, v) for k, v in exp.items() if v)}")


# --- checks ------------------------------------------------------------------


def check_streams(link):
    log("collecting FC->CC stream traffic (12 s wall)...")
    msgs = link.recv_msgs(12.0)
    by_type = {}
    for m in msgs:
        by_type.setdefault(m.get_type(), []).append(m)
    log("seen: " + ", ".join(f"{k}×{len(v)}" for k, v in sorted(by_type.items())
                             if k.startswith("CC_")))

    boot_ids = set()
    for name, expect_hz in TELEMETRY_STREAMS.items():
        got = by_type.get(name, [])
        if not check(f"stream {name} present", len(got) >= 5, f"{len(got)} msgs"):
            continue
        first, last = got[0], got[-1]
        dseq = last.sequence - first.sequence
        dt = (last.fc_timestamp_us - first.fc_timestamp_us) / 1e6
        rate = dseq / dt if dt > 0 else 0.0
        check(f"stream {name} rate", expect_hz * (1 - RATE_TOL) <= rate <= expect_hz * (1 + RATE_TOL),
              f"{rate:.1f} Hz vs {expect_hz:.1f} ±20%")
        received = len(got)
        span = dseq + 1
        gap_ratio = 1.0 - received / span if span > 0 else 1.0
        check(f"stream {name} continuity", 0 <= gap_ratio < 0.05,
              f"{received}/{span} frames on wire ({gap_ratio:.1%} lost)")
        check(f"stream {name} schema", all(m.schema_version == 1 for m in got))
        if name == "CC_TELEMETRY_STATE":
            boot_ids = {m.px4_boot_id for m in got}

    check("STATE px4_boot_id constant+nonzero", len(boot_ids) == 1 and 0 not in boot_ids,
          f"{boot_ids}")

    st = by_type.get("CC_TELEMETRY_STATE", [])
    gps = by_type.get("CC_TELEMETRY_GPS", [])
    pwr = by_type.get("CC_TELEMETRY_POWER", [])
    if st:
        q = [st[-1].q[i] for i in range(4)]
        qn = math.sqrt(sum(x * x for x in q))
        check("plausibility: quaternion normalized", abs(qn - 1.0) < 0.05, f"|q|={qn:.3f}")
    if gps:
        check("plausibility: GPS fix + position", gps[-1].fix_type >= 3 and gps[-1].lat != 0,
              f"fix={gps[-1].fix_type} lat={gps[-1].lat}")
    if pwr:
        check("plausibility: battery voltage band", 6.0 < pwr[-1].voltage < 60.0,
              f"{pwr[-1].voltage:.2f} V")
    return by_type


def make_report(tx, seq, severity=0, action=0, confidence=90, schema=1):
    tx.cc_health_report_send(
        companion_timestamp_us=123456789, sequence=seq, mission_id=0,
        companion_boot_id=0xCC0CC001, health_flags=0, detail_code=0,
        link_rtt_ms=1, telemetry_age_ms=2, companion_loop_ms=3,
        dropped_rx_count=0, severity=severity, recommended_action=action,
        confidence_percent=confidence, schema_version=schema)


def check_gauntlet(link, pxh, dialect_hash):
    tx = link.sender(191)        # legitimate companion identity
    imposter = link.sender(42)   # wrong component id

    # -- valid reports ------------------------------------------------------
    c0 = ccfc_counters(pxh)
    for s in (1, 2, 3):
        make_report(tx, seq=s, severity=1, action=1)
    time.sleep(1.0)
    expect_delta("gauntlet: 3 valid reports accepted", delta(c0, ccfc_counters(pxh)), reports=3)
    rep = sample(pxh, "cc_health_report")
    check("gauntlet: uORB cc_health_report published", rep.get("sequence") == 3,
          f"listener shows seq {rep.get('sequence')}")

    # -- each invalid class: exactly one counter, nothing published ----------
    c = ccfc_counters(pxh)
    make_report(imposter, seq=4)
    time.sleep(0.6)
    expect_delta("gauntlet: wrong component -> bad_source", delta(c, ccfc_counters(pxh)), bad_source=1)

    c = ccfc_counters(pxh)
    make_report(tx, seq=4, schema=99)
    time.sleep(0.6)
    expect_delta("gauntlet: bad schema -> bad_schema", delta(c, ccfc_counters(pxh)), bad_schema=1)

    c = ccfc_counters(pxh)
    make_report(tx, seq=4, severity=7)
    time.sleep(0.6)
    expect_delta("gauntlet: severity 7 -> bad_range", delta(c, ccfc_counters(pxh)), bad_range=1)

    c = ccfc_counters(pxh)
    make_report(tx, seq=3)  # duplicate of the last accepted
    time.sleep(0.6)
    expect_delta("gauntlet: duplicate sequence -> dup_seq", delta(c, ccfc_counters(pxh)), dup_seq=1)

    c = ccfc_counters(pxh)
    make_report(tx, seq=10)  # gap of 6 after seq 3
    time.sleep(0.6)
    expect_delta("gauntlet: sequence gap counted", delta(c, ccfc_counters(pxh)),
                 reports=1, missed=6)
    rep = sample(pxh, "cc_health_report")
    check("gauntlet: gapped report published", rep.get("sequence") == 10,
          f"listener shows seq {rep.get('sequence')}")

    # -- flood: 100 reports in one burst --------------------------------------
    time.sleep(2.0)  # let the 1 s flood window expire cleanly
    c = ccfc_counters(pxh)
    for s in range(11, 111):
        make_report(tx, seq=s)
    time.sleep(1.5)
    d = delta(c, ccfc_counters(pxh))
    check("gauntlet: flood capped (spec: >20/s dropped)",
          d["reports"] + d["flood"] == 100 and d["reports"] <= 25 and d["flood"] >= 75,
          f"accepted {d['reports']}, flood-dropped {d['flood']}")
    check("gauntlet: flood moved no other counter",
          all(d[k] == 0 for k in d if k not in ("reports", "flood")), f"{d}")

    # -- receiver alive after the flood ---------------------------------------
    time.sleep(2.0)
    c = ccfc_counters(pxh)
    make_report(tx, seq=200)
    time.sleep(0.8)
    d = delta(c, ccfc_counters(pxh))
    check("gauntlet: receiver alive post-flood", d["reports"] == 1 and d["missed"] >= 1,
          f"{d}")

    # -- mission context handshake --------------------------------------------
    c = ccfc_counters(pxh)
    tx.cc_mission_context_send(mission_id=777001, cc_boot_id=0xCC0CC001,
                               vehicle_id=1,  # == CC_VEHICLE_ID default
                               dialect_hash=dialect_hash,
                               sw_version=b"ccfc-phase3-check", schema_version=1)
    time.sleep(1.0)
    expect_delta("handshake: valid CC_MISSION_CONTEXT accepted",
                 delta(c, ccfc_counters(pxh)), context=1)
    ctx = sample(pxh, "cc_mission_context")
    check("handshake: uORB cc_mission_context published",
          ctx.get("mission_id") == 777001 and ctx.get("cc_boot_id") == 0xCC0CC001,
          f"mission_id={ctx.get('mission_id')}")

    # the echo: STATE frames on the wire must now carry the mission id
    got = [m for m in link.recv_msgs(3.0) if m.get_type() == "CC_TELEMETRY_STATE"]
    check("handshake: mission_id echoed on CC_TELEMETRY_STATE",
          bool(got) and got[-1].mission_id == 777001,
          f"wire mission_id={got[-1].mission_id if got else 'no frames'}")

    # wrong vehicle id -> refused, echo unchanged
    c = ccfc_counters(pxh)
    tx.cc_mission_context_send(mission_id=999999, cc_boot_id=1, vehicle_id=999,
                               dialect_hash=dialect_hash,
                               sw_version=b"imposter", schema_version=1)
    time.sleep(1.0)
    expect_delta("handshake: wrong vehicle_id refused",
                 delta(c, ccfc_counters(pxh)), bad_source=1)
    got = [m for m in link.recv_msgs(2.0) if m.get_type() == "CC_TELEMETRY_STATE"]
    check("handshake: mission_id unchanged after refusal",
          bool(got) and got[-1].mission_id == 777001,
          f"wire mission_id={got[-1].mission_id if got else 'no frames'}")

    # wrong dialect hash -> refused as schema mismatch
    c = ccfc_counters(pxh)
    tx.cc_mission_context_send(mission_id=1, cc_boot_id=1, vehicle_id=1,
                               dialect_hash=0xDEADBEEF,
                               sw_version=b"stale-dialect", schema_version=1)
    time.sleep(1.0)
    expect_delta("handshake: wrong dialect_hash refused",
                 delta(c, ccfc_counters(pxh)), bad_schema=1)

    # -- log-only diagnostic ----------------------------------------------------
    c = ccfc_counters(pxh)
    tx.cc_ai_diagnostic_send(companion_timestamp_us=42, sequence=1,
                             value=3.375, limit=2.5, detail_code=0x0BAD,
                             subsystem=1, severity=1, confidence_percent=87,
                             schema_version=1)
    time.sleep(1.0)
    expect_delta("diagnostic: valid CC_AI_DIAGNOSTIC accepted",
                 delta(c, ccfc_counters(pxh)), diagnostic=1)
    diag = sample(pxh, "cc_ai_diagnostic")
    check("diagnostic: uORB cc_ai_diagnostic published",
          diag.get("detail_code") == 0x0BAD, f"detail_code={diag.get('detail_code')}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--px4", type=Path, default=DEFAULT_PX4)
    ap.add_argument("--keep-rootfs", action="store_true")
    args = ap.parse_args()

    if not (args.px4 / "build/px4_sitl_default/bin/px4").exists():
        print("error: px4 binary missing — run tools/phase2/build_px4.sh first", file=sys.stderr)
        return 2

    mod = load_dialect()
    dialect_hash = preflight(mod, args.px4)

    rootfs = Path(tempfile.mkdtemp(prefix="ccfc-sitl3-rootfs-"))
    pxh = Pxh(args.px4, rootfs)
    link = None

    try:
        log("=== Phase 3 SITL verification (SIH headless, UDP companion link) ===")
        link = CcfcLink(mod)   # bind BEFORE px4 starts sending
        pxh.start()
        log("pxh up; waiting for CCFC-instance heartbeat on UDP "
            f"127.0.0.1:{CCFC_UDP_PORT}")
        check("link: HEARTBEAT from CCFC mavlink instance", link.wait_link(60))
        time.sleep(6)  # EKF/GPS settle so plausibility fields are meaningful

        check_streams(link)
        check_gauntlet(link, pxh, dialect_hash)

        # evidence capture
        log("mavlink status (CCFC lines):")
        for ln in pxh.run("mavlink status", timeout=20).splitlines():
            if "CCFC" in ln:
                log("  " + ln.strip())

    except Exception as e:
        check("harness completed without exception", False, repr(e))
    finally:
        pxh.stop(SCRIPT_DIR / "px4_server_last.log")
        if link:
            link.sock.close()
        if args.keep_rootfs:
            log(f"rootfs kept at {rootfs}")
        else:
            shutil.rmtree(rootfs, ignore_errors=True)

    n_ok = sum(1 for _, ok, _ in results if ok)
    print("\n===== Phase 3 SITL check summary =====")
    for name, ok, detail in results:
        print(f"  {'PASS' if ok else 'FAIL'}  {name}" + (f" — {detail}" if detail else ""))
    print(f"===== {n_ok}/{len(results)} checks passed =====")
    (SCRIPT_DIR / "last_run.log").write_text("\n".join(log_lines) + "\n")
    return 0 if n_ok == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
