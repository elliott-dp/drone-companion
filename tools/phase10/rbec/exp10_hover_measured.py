"""Experiment 10 (thesis P5): the RBEC budget on MEASURED hover motion.

Replaces the last synthetic input of the end-to-end chain — the shaped-noise
sway model — with real PX4 hover trajectories (``hover_ingest.py`` npz,
replayed by ``endtoend.platform_motion``; EKF-band caveat applies, rotor
band still synthetic, per HOVER_CAPTURE.md). Re-derives the four
decision-relevant slices of exp3/exp4 per hover segment, each with its
synthetic-model twin so the substitution's effect is the measurement:

  A. T3 anchor-angle coupling (the pacing-term claim),
  B. the IMU per-gap seam sweep (cliff position under real sway),
  C. seam mitigations at the 200 um cliff onset,
  D. the combined worst-realistic budget verdict (register claim 1).

Data provenance: one public hover-endurance ULog supplied by the author
(review.px4.io-style download, filename UUID cfeabd37-..., sha256 prefix
8e89c1b20b380b39db6ee550f9f668bd; NOT the project's own airframe — a
representative multirotor stand-in, so P5 claims are tiered
"measured hover, representative platform" until the flight card flies).
Ingest (2026-08-27): four Loiter segments, 162/845/596/1175 s at 10 Hz
(attitude 20 Hz); vital-band (0.5-3 Hz) sway ~2 mm RMS/axis on segments
1-3; segment 0 carries station-keeping excursions (40-60 cm total RMS)
and serves as the sloppy-hover stressor; segment 3 carries a slow
endurance z-descent (71 cm total, detrended by the replay's mean removal
per 30 s window).

Usage:
    python3 -m tools.phase10.rbec.exp10_hover_measured [--check|--self-test]
"""

from __future__ import annotations

import json
import os
import sys

import numpy as np

from .core import CARDIAC_RESIDUAL_BUDGET_RAD
from .endtoend import SimConfig, run

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
RESULTS_DIR = os.path.join(ROOT, "docs", "phase10", "results")
NPZ = os.path.join(RESULTS_DIR, "hover_p5.npz")
JSON_PATH = os.path.join(RESULTS_DIR, "exp10.json")
FIG_PATH = os.path.join(RESULTS_DIR, "fig_rbec_exp10.png")

SEEDS = list(range(1, 9))
SEGMENTS = [0, 1, 2, 3]
SOURCE = {"ulg": "cfeabd37-0d00-459d-8340-f2bfa2f6cbf8.ulg",
          "sha256_prefix": "8e89c1b20b380b39db6ee550f9f668bd",
          "platform": "representative multirotor (public log), "
                      "not the project airframe"}


def cases(motion: dict) -> list[tuple[str, str, SimConfig]]:
    """The decision-relevant grid; ``motion`` supplies motion_npz/segment
    (empty dict = the synthetic twin at exp3's defaults)."""
    out = []
    for a in [0.0, 0.1, 0.3, 0.675]:
        out.append(("T3", f"angle {a:.3f} deg",
                    SimConfig(anchor_angle_err_deg=a, **motion)))
    for s in [50e-6, 200e-6, 300e-6, 450e-6, 500e-6]:
        out.append(("IMU", f"imu {s*1e6:.0f} um",
                    SimConfig(imu_gap_sigma_m=s, **motion)))
    for name, kw in [("bare", {}), ("raim", {"raim_seam": True}),
                     ("prior+raim+repair",
                      {"chest_prior": True, "raim_seam": True,
                       "slip_repair": True})]:
        out.append(("MITIG200", name,
                    SimConfig(imu_gap_sigma_m=200e-6, **kw, **motion)))
    worst = dict(anchor_angle_err_deg=0.3, leak_amp_ratio=0.0316,
                 apll_step_rad=0.2, chip_step_rad=0.02)
    # the synthetic twin of D keeps exp3's 5 cm sway; measured arms
    # replace exactly that input
    wsyn = {} if motion else {"sway_rms_m": 0.05}
    out.append(("WORST", "0.3deg -30dB 200mrad",
                SimConfig(**worst, **wsyn, **motion)))
    out.append(("WORST", "same + corner",
                SimConfig(**worst, corner_anchor=True, **wsyn, **motion)))
    return out


def run_arm(motion: dict) -> list[dict]:
    rows = []
    for block, name, cfg in cases(motion):
        rs = [run(cfg, s) for s in SEEDS]
        cardr = np.array([r.cardiac_err_rms_rad for r in rs])
        rows.append({
            "block": block, "case": name,
            "cardiac_rad_mean": float(cardr.mean()),
            "cardiac_rad_std": float(cardr.std()),
            "cardiac_rad_worst": float(cardr.max()),
            "cardiac_um_mean": float(np.mean(
                [r.cardiac_err_rms_m for r in rs]) * 1e6),
            "resp_um_mean": float(np.mean(
                [r.resp_err_rms_m for r in rs]) * 1e6),
            "int_fail_target": int(sum(r.integer_fail_target for r in rs)),
            "int_fail_anchor": int(sum(r.integer_fail_anchor for r in rs)),
            "n_seams": int(sum(r.n_seams for r in rs)),
            "budget_ok": bool(cardr.mean() <= CARDIAC_RESIDUAL_BUDGET_RAD),
        })
    return rows


def build() -> dict:
    z = np.load(NPZ, allow_pickle=False)
    segs = {}
    for i in SEGMENTS:
        xyz = z[f"seg{i}_xyz"]
        segs[str(i)] = {
            "dur_s": float(z[f"seg{i}_t"][-1]),
            "rms_cm": [float(x) for x in xyz.std(axis=0) * 100.0],
        }
    out = {"budget_rad": CARDIAC_RESIDUAL_BUDGET_RAD, "seeds": SEEDS,
           "source": SOURCE, "segments": segs,
           "synthetic": run_arm({}),
           "measured": {str(i): run_arm(
               {"motion_npz": NPZ, "motion_segment": i}) for i in SEGMENTS}}
    return out


def make_figure(data: dict, path: str) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 4.5))
    angles = [0.0, 0.1, 0.3, 0.675]
    syn = [r["cardiac_rad_mean"] for r in data["synthetic"]
           if r["block"] == "T3"]
    ax1.plot(angles, syn, "k--o", label="synthetic sway (2 cm model)")
    for i, rows in sorted(data["measured"].items()):
        t3 = [r["cardiac_rad_mean"] for r in rows if r["block"] == "T3"]
        ax1.plot(angles, t3, "-s", label=f"measured seg{i}")
    ax1.axhline(data["budget_rad"], color="r", ls=":",
                label=f"budget {data['budget_rad']:.3f}")
    ax1.set_xlabel("anchor angle error [deg, 1-sigma]")
    ax1.set_ylabel("cardiac residual [rad]")
    ax1.set_title("T3 coupling: measured hover vs synthetic")
    ax1.legend(fontsize=8)
    sig = [50, 200, 300, 450, 500]
    fs = [r["int_fail_target"] + r["int_fail_anchor"]
          for r in data["synthetic"] if r["block"] == "IMU"]
    ns = [r["n_seams"] for r in data["synthetic"] if r["block"] == "IMU"]
    ax2.semilogy(sig, [max(f / n, 3e-6) for f, n in zip(fs, ns)], "k--o",
                 label="synthetic")
    for i, rows in sorted(data["measured"].items()):
        fm = [r["int_fail_target"] + r["int_fail_anchor"]
              for r in rows if r["block"] == "IMU"]
        nm = [r["n_seams"] for r in rows if r["block"] == "IMU"]
        ax2.semilogy(sig, [max(f / n, 3e-6) for f, n in zip(fm, nm)],
                     "-s", label=f"measured seg{i}")
    ax2.set_xlabel("IMU per-gap sigma [um]")
    ax2.set_ylabel("integer failure rate per seam (floor 3e-6)")
    ax2.set_title("Seam cliff under measured sway")
    ax2.legend(fontsize=8)
    fig.suptitle("exp10 (P5): RBEC budget on measured hover motion")
    fig.tight_layout()
    fig.savefig(path, dpi=130)


def _compare(a, b, path="") -> list[str]:
    bad = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                bad.append(f"{path}/{k}: missing on one side")
            else:
                bad += _compare(a[k], b[k], f"{path}/{k}")
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            bad.append(f"{path}: length {len(a)} vs {len(b)}")
        else:
            for i, (x, y) in enumerate(zip(a, b)):
                bad += _compare(x, y, f"{path}[{i}]")
    elif isinstance(a, float) and isinstance(b, float):
        if not np.isclose(a, b, rtol=1e-9, atol=0.0, equal_nan=True):
            bad.append(f"{path}: {a!r} != {b!r}")
    elif a != b:
        bad.append(f"{path}: {a!r} != {b!r}")
    return bad


def _self_test() -> None:
    z = np.load(NPZ, allow_pickle=False)
    assert int(z["n_segments"]) >= 4, "expected 4 hover segments"
    r_m = run(SimConfig(motion_npz=NPZ, motion_segment=1), 1)
    r_s = run(SimConfig(), 1)
    assert r_m.n_seams == r_s.n_seams, "seam schedule must not depend on sway"
    assert 0.0 < r_m.cardiac_err_rms_rad < CARDIAC_RESIDUAL_BUDGET_RAD, \
        "measured default must sit inside the budget"
    hard = run(SimConfig(motion_npz=NPZ, motion_segment=1,
                         imu_gap_sigma_m=950e-6), 1)
    assert (hard.integer_fail_target + hard.integer_fail_anchor) > 0, \
        "950 um/gap must fail seams on measured motion too"
    print("exp10 self_test OK: 4 segments, measured default "
          f"{r_m.cardiac_err_rms_rad:.4f} rad inside budget, seam schedule "
          f"stable ({r_m.n_seams}), 950 um cliff reproduces "
          f"({hard.integer_fail_target}t+{hard.integer_fail_anchor}a)")


def main() -> None:
    if "--self-test" in sys.argv:
        _self_test()
        return
    data = build()
    if "--check" in sys.argv:
        with open(JSON_PATH) as fh:
            committed = json.load(fh)
        bad = _compare(data, committed)
        if bad:
            print(f"MISMATCH ({len(bad)} values):")
            for m in bad[:20]:
                print(" ", m)
            sys.exit(1)
        print(f"exp10 --check: regenerated bundle matches "
              f"{os.path.relpath(JSON_PATH, ROOT)} exactly")
        return
    with open(JSON_PATH, "w") as fh:
        json.dump(data, fh, indent=1)
    print(f"wrote {JSON_PATH}")
    try:
        make_figure(data, FIG_PATH)
        print(f"wrote {FIG_PATH}")
    except ImportError:
        print("matplotlib not available -- figure skipped")
    for arm, rows in [("synthetic", data["synthetic"])] + [
            (f"seg{i}", data["measured"][str(i)]) for i in SEGMENTS]:
        w = [r for r in rows if r["block"] == "WORST"][0]
        t = [r for r in rows if r["block"] == "T3"][2]
        print(f"{arm:10s} worst-realistic {w['cardiac_rad_mean']:.4f} rad "
              f"({'OK' if w['budget_ok'] else 'FAIL'} vs "
              f"{CARDIAC_RESIDUAL_BUDGET_RAD:.3f}) | T3@0.3deg "
              f"{t['cardiac_rad_mean']:.4f} rad")


if __name__ == "__main__":
    main()
