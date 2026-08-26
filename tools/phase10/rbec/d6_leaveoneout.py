#!/usr/bin/env python3
"""D.6 mechanism probe: is the bin-4 leakage anchor load-bearing?

Holdout-matched leave-one-out on the guard-4 upgraded anchor set: same
windows, same holdout cells, drop one anchor at a time and re-solve.
If dropping the bin-4 cell moves the held-out residual far more than
dropping any real anchor, the leakage anchor is load-bearing in the
solve itself (not a pool-composition artifact).

Measured (2026-08, ColoRadar v1 aspen still windows, deltas vs all-9):
dropping the leakage anchor moves holdout by -8.6 (run0), +15.0 (run1),
+8.3 (run2), +37.3 (run3) um while every real-anchor drop stays within
-6.7..+9.7 um — the leakage cell (always az ~ +50 deg, the array's own
coupling direction) acts as an unintentional zero-motion regularizer
with run-dependent net effect.

Usage: python3 -m tools.phase10.rbec.d6_leaveoneout <dataset_root>
"""
import sys
import numpy as np
from .coloradar_bridge import CascadeCalib, CascadeSequence
from .exp5b_upgrade import (DA_GATE, pick_anchors_v2, track_phases,
                            integer_fixed_increments, solve_plain,
                            gt_positions, holdout_residual_um)

ROOT = sys.argv[1] if len(sys.argv) > 1 else "/Volumes/FRAGMENT/coloradar"
WINDOWS = [("2_24_2021_aspen_run0", 1, 42), ("2_24_2021_aspen_run1", 408, 458),
           ("2_24_2021_aspen_run2", 411, 471),
           ("2_24_2021_aspen_run3", 533, 584)]
calib = CascadeCalib(ROOT + "/calib")
k = 4 * np.pi / calib.lam
for seq_name, f0, f1 in WINDOWS:
    seq = CascadeSequence(f"{ROOT}/kitti/{seq_name}", calib)
    pick = pick_anchors_v2(seq, 9, 3, f0, f_end=f1, corange_tol_bins=1,
                           da_gate=DA_GATE, guard_bins=4)
    anchors, holdout = pick["anchors"], pick["holdout"]
    az = np.array([a for a, _ in anchors])
    rb = np.array([r for _, r in anchors])
    U2 = np.stack([np.cos(az), np.sin(az)], axis=1)
    Uh2 = np.array([[np.cos(a), np.sin(a)] for a, _ in holdout]).reshape(-1, 2)
    ph_a = track_phases(seq, anchors, f0, f1)
    ph_h = track_phases(seq, holdout, f0, f1)
    gpos = gt_positions(seq)
    dgt = np.diff(gpos, axis=0)[f0:f1 - 1, :2]
    Y = integer_fixed_increments(ph_a, U2, dgt, k)
    d_all = solve_plain(U2, Y)
    h_all = holdout_residual_um(ph_h, Uh2, d_all, k)
    name = seq_name.replace("2_24_2021_", "")
    print(f"=== {name} [{f0}:{f1})  anchors rbin {list(rb)}  "
          f"holdout rbin {[r for _, r in holdout]}  all-9 holdout "
          f"{h_all:.1f} um  |d|rms {d_all.std(axis=0)[0]*1e6:.0f}/"
          f"{d_all.std(axis=0)[1]*1e6:.0f} um")
    for i in range(len(anchors)):
        keep = [j for j in range(len(anchors)) if j != i]
        d_s = solve_plain(U2[keep], Y[keep])
        h_s = holdout_residual_um(ph_h, Uh2, d_s, k)
        tagc = "LEAKAGE" if rb[i] < 8 else "real"
        print(f"   drop rbin {rb[i]:3d} ({tagc:7s} az {np.rad2deg(az[i]):+6.1f}"
              f") -> holdout {h_s:7.1f} um  (delta {h_s - h_all:+7.1f})  "
              f"|d|rms {d_s.std(axis=0)[0]*1e6:5.0f}/"
              f"{d_s.std(axis=0)[1]*1e6:5.0f} um")
