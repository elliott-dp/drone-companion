#!/usr/bin/env python3
"""D.7 seed-budget probe: conventions, bias, and per-pair bridging error.

For each window, reports (a) the mean world-frame accel after the
base_to_imu + GT-attitude rotation (must be ~[0, 0, +9.81]: the sign and
frame proof), (b) the open-loop dead-reckoned seed error per frame pair,
and (c) the per-pair velocity-re-anchored (imu-rot-track) seed error —
the number that decides integer seeding against the lambda/4 = 958 um
margin.

Measured on ColoRadar v1 aspen (2026-08, ZUPT bias-calibrated,
Lord 3DM-GX5-25 at ~494 Hz, vicon attitude at 98 Hz):
  conventions: mean a_world z = +9.78..+9.80 across all windows (the
    naive world-aligned reader saw -9.79 — base_to_imu is a ~180 deg
    flip about (1,1,0));
  open-loop over a 10-12 s dwell: 5-123 mm median per pair even after
    ZUPT — the ~4e-3 m/s^2 post-calibration residual double-integrates
    past lambda/4 within ~1 s (INS physics, not implementation);
  per-pair re-anchored bridge: 248 um median on a still window
    (seeding-grade), 1.5-10 mm median on sway windows — the residual
    scales with motion and is attributed to ~0.2-0.6 deg effective
    attitude error leaking gravity (~0.05 m/s^2); an IMU-vicon clock
    offset was tested by accel cross-correlation and refuted
    (corr ~0.06, inconsistent lags across runs).
  The same leakage over the flight design's 47 ms inter-burst gap
    scales (~Delta t^2 for the bias-like term) to ~55 um — inside the
    F-series <=100 um budget; ColoRadar's 0.2 s frame gap is the
    limiter, not the mechanism.

Usage: python3 -m tools.phase10.rbec.d7_seed_budget <dataset_root>
"""
import sys

import numpy as np

from .coloradar_bridge import CascadeCalib, CascadeSequence, quat_mats
from .exp5b_upgrade import (cascade_frame_increments, cascade_track,
                            gt_attitude, imu_bias_body, imu_increments_rotated,
                            imu_pair_bridge, read_imu)

ROOT = sys.argv[1] if len(sys.argv) > 1 else "/Volumes/FRAGMENT/coloradar"
WINDOWS = [("2_24_2021_aspen_run2", 411, 471, "still"),
           ("2_24_2021_aspen_run2", 5, 55, "sway"),
           ("2_24_2021_aspen_run9", 350, 400, "sway"),
           ("2_24_2021_aspen_run0", 365, 415, "sway")]
LAM4 = 957.7e-6


def main() -> None:
    calib = CascadeCalib(ROOT + "/calib")
    _, R_bi = calib.transforms["base_to_imu"]
    for seq_name, f0, f1, kind in WINDOWS:
        seq = CascadeSequence(f"{ROOT}/kitti/{seq_name}", calib)
        imu_t, a_body = read_imu(f"{ROOT}/kitti/{seq_name}")
        t0 = seq.times[f0]
        t1 = seq.times[min(f1, seq.n_frames() - 1)]
        sel = (imu_t >= t0) & (imu_t <= t1)
        R_wb = quat_mats(gt_attitude(seq, imu_t[sel]))
        a_w = np.einsum("nij,nj->ni", R_wb, a_body[sel] @ R_bi.T)
        p_ref, R_wc = cascade_track(seq)
        dgt = cascade_frame_increments(p_ref, R_wc)
        print(f"=== {seq_name} [{f0}:{f1}) {kind}: mean a_world "
              f"[{a_w[:, 0].mean():+.3f} {a_w[:, 1].mean():+.3f} "
              f"{a_w[:, 2].mean():+.3f}] (want ~ [0 0 +9.81]); ZUPT bias "
              f"{np.linalg.norm(imu_bias_body(imu_t, a_body, seq)):.3f} "
              f"m/s^2")
        for tag, fn in (("open-loop",
                         lambda: imu_increments_rotated(
                             imu_t, a_body, seq, p_ref, R_wc, f0)),
                        ("pair-bridge",
                         lambda: imu_pair_bridge(
                             imu_t, a_body, seq, p_ref, R_wc))):
            inc = fn()
            if inc is None:
                print(f"    {tag}: no IMU coverage")
                continue
            err = np.linalg.norm((inc - dgt)[f0:f1 - 1, :2], axis=1)
            print(f"    {tag:11s} seed err um/pair: med "
                  f"{np.median(err) * 1e6:8.1f} p95 "
                  f"{np.percentile(err, 95) * 1e6:8.1f} | pairs<lam/4 "
                  f"{int(np.sum(err < LAM4))}/{err.size}")


if __name__ == "__main__":
    main()
