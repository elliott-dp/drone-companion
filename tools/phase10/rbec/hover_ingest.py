"""Ingest PX4 hover ULogs into the RBEC simulation (validation follow-up 3).

Turns real hover flights into (a) a measured position/attitude PSD report and
(b) an .npz of detrended hover trajectories that ``endtoend.SimConfig
(motion_npz=...)`` replays in place of the synthetic shaped-noise sway —
replacing the one input the V1 stack fakes.

Usage (from the repo root):

    python3 -m tools.phase10.rbec.hover_ingest LOG1.ulg [LOG2.ulg ...] \
        [--out tools/phase10/rbec/hover_data.npz] \
        [--segment FILEIDX:START:END]   # seconds, manual override
        [--min-hover 40]

Requires ``pyulog`` (pip install pyulog) — import-guarded so the rest of the
stack stays NumPy-only. Capture protocol: HOVER_CAPTURE.md next to this file.
"""

from __future__ import annotations

import argparse
import sys

import numpy as np

try:
    from pyulog import ULog
except ImportError:  # pragma: no cover
    ULog = None

# PX4 nav_state values that count as hover-capable modes
HOVER_NAV_STATES = {2, 4, 17}          # POSCTL, AUTO_LOITER, ORBIT
BANDS = ((0.05, 0.5), (0.5, 3.0))      # report bands [Hz]


def welch_psd(x: np.ndarray, fs: float, nseg: int = 8):
    """Segment-averaged periodogram (Hann, 50 % overlap), NumPy-only.
    Returns (freqs, psd) with psd in x-units^2/Hz, single-sided."""
    n = x.size
    seg = max(256, int(2 ** np.floor(np.log2(2 * n / (nseg + 1)))))
    seg = min(seg, n)
    step = seg // 2
    w = np.hanning(seg)
    scale = 1.0 / (fs * (w ** 2).sum())
    acc, count = 0.0, 0
    for s in range(0, n - seg + 1, step):
        d = x[s:s + seg] - x[s:s + seg].mean()
        acc = acc + (np.abs(np.fft.rfft(d * w)) ** 2) * scale
        count += 1
    psd = acc / max(count, 1)
    psd[1:-1] *= 2.0
    return np.fft.rfftfreq(seg, 1.0 / fs), psd


def band_rms_from_psd(f: np.ndarray, psd: np.ndarray,
                      band: tuple[float, float]) -> float:
    sel = (f >= band[0]) & (f <= band[1])
    if not sel.any():
        return float("nan")
    return float(np.sqrt(np.trapezoid(psd[sel], f[sel])))


def quat_to_euler(q: np.ndarray) -> np.ndarray:
    """(N,4) w,x,y,z -> (N,3) roll,pitch,yaw [rad]."""
    w, x, y, z = q.T
    roll = np.arctan2(2 * (w * x + y * z), 1 - 2 * (x * x + y * y))
    pitch = np.arcsin(np.clip(2 * (w * y - z * x), -1, 1))
    yaw = np.arctan2(2 * (w * z + x * y), 1 - 2 * (y * y + z * z))
    return np.stack([roll, pitch, yaw], axis=1)


def load_log(path: str):
    ulog = ULog(path, message_name_filter_list=[
        "vehicle_local_position", "vehicle_attitude", "vehicle_status"])
    out = {}
    for name in ("vehicle_local_position", "vehicle_attitude",
                 "vehicle_status"):
        try:
            out[name] = ulog.get_dataset(name).data
        except (KeyError, IndexError):
            out[name] = None
    return out


def detect_hover_segments(data: dict, min_dur: float) -> list[tuple[float, float]]:
    lp = data["vehicle_local_position"]
    if lp is None:
        return []
    t = lp["timestamp"] * 1e-6
    vh = np.hypot(lp["vx"], lp["vy"])
    vz = np.abs(lp["vz"])
    ok = (vh < 1.0) & (vz < 0.5)
    st = data["vehicle_status"]
    if st is not None:
        ts = st["timestamp"] * 1e-6
        nav = st["nav_state"]
        nav_i = nav[np.clip(np.searchsorted(ts, t) - 1, 0, nav.size - 1)]
        ok &= np.isin(nav_i, list(HOVER_NAV_STATES))
        arm = st.get("arming_state")
        if arm is not None:
            arm_i = arm[np.clip(np.searchsorted(ts, t) - 1, 0, arm.size - 1)]
            ok &= (arm_i == 2)             # ARMED
    segs, start = [], None
    for i, o in enumerate(ok):
        if o and start is None:
            start = t[i]
        elif not o and start is not None:
            if t[i - 1] - start >= min_dur:
                segs.append((start, t[i - 1]))
            start = None
    if start is not None and t[-1] - start >= min_dur:
        segs.append((start, t[-1]))
    return segs


def extract_segment(data: dict, t0: float, t1: float) -> dict | None:
    lp = data["vehicle_local_position"]
    t = lp["timestamp"] * 1e-6
    sel = (t >= t0) & (t <= t1)
    if sel.sum() < 100:
        return None
    ts = t[sel]
    fs = 1.0 / np.median(np.diff(ts))
    tu = np.arange(ts[0], ts[-1], 1.0 / fs)
    xyz = np.stack([np.interp(tu, ts, lp[k][sel]) for k in ("x", "y", "z")],
                   axis=1)
    # linear detrend per axis: the sim wants sway about the hover point;
    # slow GNSS drift beyond the 0.05 Hz band edge is not sway
    tt = tu - tu[0]
    for i in range(3):
        c = np.polyfit(tt, xyz[:, i], 1)
        xyz[:, i] -= np.polyval(c, tt)
    seg = {"t": tu - tu[0], "fs": fs, "xyz": xyz}
    att = data["vehicle_attitude"]
    if att is not None:
        ta = att["timestamp"] * 1e-6
        sa = (ta >= t0) & (ta <= t1)
        if sa.sum() > 100:
            q = np.stack([att[f"q[{i}]"][sa] for i in range(4)], axis=1)
            eul = quat_to_euler(q)
            eul = np.unwrap(eul, axis=0)
            fse = 1.0 / np.median(np.diff(ta[sa]))
            tue = np.arange(ta[sa][0], ta[sa][-1], 1.0 / fse)
            seg["euler"] = np.stack(
                [np.interp(tue, ta[sa], eul[:, i]) for i in range(3)], axis=1)
            seg["fs_att"] = fse
    return seg


def report(seg: dict, label: str) -> None:
    print(f"\n-- {label}: {seg['t'][-1]:.0f} s at {seg['fs']:.0f} Hz "
          f"(attitude {seg.get('fs_att', 0):.0f} Hz)")
    hdr = "  axis  RMS[cm]" + "".join(f"   {a}-{b} Hz[cm]" for a, b in BANDS)
    print(hdr)
    for i, ax in enumerate("xyz"):
        f, p = welch_psd(seg["xyz"][:, i], seg["fs"])
        cells = "".join(f"{band_rms_from_psd(f, p, b)*100:>13.2f}"
                        for b in BANDS)
        print(f"  {ax}  {seg['xyz'][:, i].std()*100:>8.2f}{cells}")
    if "euler" in seg:
        print("  attitude RMS [deg]: " + "  ".join(
            f"{n}={np.rad2deg(seg['euler'][:, i].std()):.3f}"
            for i, n in enumerate(("roll", "pitch", "yaw"))))


def main(argv=None) -> int:
    if ULog is None:
        print("pyulog is required: python3 -m pip install --user pyulog")
        return 1
    ap = argparse.ArgumentParser()
    ap.add_argument("logs", nargs="+")
    ap.add_argument("--out", default="tools/phase10/rbec/hover_data.npz")
    ap.add_argument("--segment", action="append", default=[],
                    help="FILEIDX:START:END seconds, manual override")
    ap.add_argument("--min-hover", type=float, default=40.0)
    args = ap.parse_args(argv)

    segments = []
    for fi, path in enumerate(args.logs):
        data = load_log(path)
        manual = [tuple(map(float, s.split(":")[1:])) for s in args.segment
                  if int(s.split(":")[0]) == fi]
        wins = manual or detect_hover_segments(data, args.min_hover)
        if not wins:
            print(f"{path}: no hover segments >= {args.min_hover}s found "
                  "(use --segment FILEIDX:START:END)")
            continue
        for (t0, t1) in wins:
            seg = extract_segment(data, t0, t1)
            if seg is None:
                continue
            label = f"{path} [{t0:.0f}-{t1:.0f}s]"
            report(seg, label)
            seg["label"] = label
            segments.append(seg)

    if not segments:
        print("nothing extracted")
        return 1
    out = {}
    for i, s in enumerate(segments):
        for k in ("t", "xyz", "fs"):
            out[f"seg{i}_{k}"] = s[k]
        if "euler" in s:
            out[f"seg{i}_euler"] = s["euler"]
            out[f"seg{i}_fs_att"] = s["fs_att"]
        out[f"seg{i}_label"] = s["label"]
    out["n_segments"] = len(segments)
    np.savez(args.out, **out)
    print(f"\nwrote {args.out} ({len(segments)} segment(s)); replay with "
          "SimConfig(motion_npz='<path>', motion_segment=<i>)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
