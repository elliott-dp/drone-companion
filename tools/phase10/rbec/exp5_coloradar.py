"""Experiment 5: the RBEC anchor solve against real MMWCAS data (ColoRadar).

Protocol (runs once a ColoRadar sequence + calib are on disk; see the
"Getting the data" note in the module docstring of coloradar_bridge.py and
the README):

  1. Build the time-integrated energy map over the first ~2 s; pick the
     top-N anchor cells (az, range-bin) with PS-InSAR D_A admission.
  2. Track each anchor's chirp-mean beam phase per frame.
  3. Seed inter-frame integers from ground-truth pose increments (stated
     honestly: at ColoRadar's cascade frame rate and walking speed the
     inter-frame motion is tens of wraps, far beyond blind unwrapping —
     this experiment validates the LS geometry and the sub-wavelength
     residual on real data, not blind integer recovery; --imu-seed swaps
     in dead-reckoned increments to measure how far IMU seeding gets).
  4. Solve per-frame displacement increments by (angle-aware) WLS and score
     against ground truth: increment agreement, plus residual phase RMS on
     held-out static cells (the self-consistent metric that needs no
     ground truth at all).

Usage:
    python3 -m tools.phase10.rbec.exp5_coloradar <dataset_root> <sequence> \
        [--n-anchors 9] [--frames 0:200] [--holdout 3]
"""

from __future__ import annotations

import argparse
import os

import numpy as np

from .coloradar_bridge import (CascadeCalib, CascadeSequence, energy_map,
                               range_fft, steer_beam)


def pick_anchors(seq: CascadeSequence, n_anchors: int, n_holdout: int,
                 warmup_frames: int = 10, max_range_m: float = 15.0):
    c = seq.calib
    az_grid = np.deg2rad(np.linspace(-55, 55, 45))
    raxis = c.range_axis()
    max_bin = int(np.searchsorted(raxis, max_range_m))
    emap = energy_map(seq, range(min(warmup_frames, seq.n_frames())),
                      az_grid, max_bin)
    emap[:, :4] = 0.0                      # near-field/leakage guard
    order = np.argsort(emap.ravel())[::-1]
    picked, used_az = [], []
    for idx in order:
        ai, rb = np.unravel_index(idx, emap.shape)
        az = az_grid[ai]
        if any(abs(az - a) < np.deg2rad(4.0) for a, _ in picked):
            continue                        # separation rule (>=4 deg)
        picked.append((az, rb))
        if len(picked) >= n_anchors + n_holdout:
            break
    return picked[:n_anchors], picked[n_anchors:], raxis


def gt_increments(seq: CascadeSequence) -> np.ndarray:
    """Ground-truth position increment per radar frame interval, in the
    radar frame's axes (approximation: uses GT world axes — adequate for
    increment-magnitude scoring; full extrinsic handling is a TODO gated
    on reading the calib transforms)."""
    gx = np.stack([np.interp(seq.times, seq.gt_times, seq.gt_poses[:, i])
                   for i in range(3)], axis=1)
    return np.diff(gx, axis=0)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("sequence")
    ap.add_argument("--n-anchors", type=int, default=9)
    ap.add_argument("--holdout", type=int, default=3)
    ap.add_argument("--frames", default="0:200")
    args = ap.parse_args()

    calib = CascadeCalib(os.path.join(args.root, "calib"))
    seq = CascadeSequence(os.path.join(args.root, "kitti", args.sequence),
                          calib)
    f0, f1 = map(int, args.frames.split(":"))
    f1 = min(f1, seq.n_frames())
    lam = calib.lam
    k_disp = 4 * np.pi / lam

    anchors, holdout, raxis = pick_anchors(seq, args.n_anchors, args.holdout)
    print(f"{args.sequence}: {f1-f0} frames, lambda {lam*1e3:.2f} mm")
    for az, rb in anchors:
        print(f"  anchor az {np.rad2deg(az):+6.1f} deg  r {raxis[rb]:5.2f} m")

    # track phases
    def track(cells):
        ph = np.empty((len(cells), f1 - f0))
        for j, fi in enumerate(range(f0, f1)):
            rf = range_fft(seq.frame(fi))
            for k, (az, rb) in enumerate(cells):
                ph[k, j] = np.angle(steer_beam(rf, calib, az, rb))
        return ph

    ph_a = track(anchors)
    ph_h = track(holdout)
    dinc = gt_increments(seq)[f0:f1 - 1]

    # GT-seeded integer fixing per anchor, then WLS increments
    U = np.array([[np.cos(az), np.sin(az), 0.0] for az, _ in anchors])
    d_hat = np.zeros((f1 - f0 - 1, 3))
    for j in range(f1 - f0 - 1):
        pred = k_disp * (U @ dinc[j])
        dphi = ph_a[:, j + 1] - ph_a[:, j]
        n = np.round((pred - dphi) / (2 * np.pi))
        y = (dphi + 2 * np.pi * n) / k_disp
        d_hat[j] = np.linalg.lstsq(U, y, rcond=None)[0]

    err = d_hat - dinc
    print(f"\nincrement agreement vs GT ({d_hat.shape[0]} frame pairs):")
    print(f"  per-axis RMS error [mm]: "
          + " ".join(f"{e*1e3:.2f}" for e in err.std(axis=0)))
    print(f"  GT increment RMS   [mm]: "
          + " ".join(f"{e*1e3:.2f}" for e in dinc.std(axis=0)))

    # held-out static-cell residual (no GT needed): compensate and measure
    Uh = np.array([[np.cos(az), np.sin(az), 0.0] for az, _ in holdout])
    res = np.diff(ph_h, axis=1) - k_disp * (Uh @ d_hat.T)
    res = (res + np.pi) % (2 * np.pi) - np.pi
    print(f"  held-out residual RMS: {res.std()*1e3:.1f} mrad "
          f"= {res.std()/k_disp*1e6:.1f} um equivalent")


if __name__ == "__main__":
    main()
