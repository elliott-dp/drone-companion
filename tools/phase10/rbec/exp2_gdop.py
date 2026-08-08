"""Experiment 2: GDOP maps for realistic anchor geometries.

Scenes: (a) ground-ring anchors from hover height h (flat ground — the
suspected near-degenerate case), (b) the same plus a structure (elevation-
diverse anchors), (c) the same plus one corner reflector at chosen offsets,
(d) uniform sector spread as the reference case. Reports per-axis DOPs,
the target-LOS DOP (what the prediction error actually scales with), the
condition number, and the common-mode rejection factor of solve+differencing
(paper §C.3 / §E: bracketing anchors should implicitly cancel a common
phase offset).
"""

from __future__ import annotations

import numpy as np

from .core import los_from_azel
from .geometry import (axis_dops, common_mode_rejection, condition_number,
                       scene_ground_ring, scene_sector, target_dop)

H = 10.0  # hover height [m]


def report(name: str, U: np.ndarray, u_t: np.ndarray) -> None:
    try:
        dops = axis_dops(U)
        tdop = target_dop(U, u_t)
        cmr = common_mode_rejection(U, u_t)
        cond = condition_number(U)
        print(f"{name:<46s} N={U.shape[0]:>2d}  "
              f"DOP(x,y,z)=({dops[0]:5.2f},{dops[1]:5.2f},{dops[2]:5.2f})  "
              f"DOP(u_t)={tdop:5.2f}  CM-leak={cmr:6.3f}  cond={cond:9.1f}")
    except np.linalg.LinAlgError:
        print(f"{name:<46s} N={U.shape[0]:>2d}  SINGULAR "
              "(LOS directions rank-deficient)")


def main() -> None:
    rng = np.random.default_rng(20260808)
    u_t = los_from_azel(0.0, np.deg2rad(-30.0))   # casualty at 30 deg depression
    print(f"RBEC exp2: anchor-geometry DOPs (hover h={H} m; target el=-30deg)")
    print("DOP(u_t): prediction-error amplification along the target LOS;")
    print("CM-leak: fraction of a common anchor-phase offset surviving at the target\n")

    az7 = np.deg2rad(np.linspace(-50, 50, 7))

    # (a) flat ground, single range ring (all same depression) vs spread rings
    ring1 = scene_ground_ring(H, np.array([17.3]), az7)          # el = -30 deg
    rings = scene_ground_ring(H, np.array([6.0, 12.0, 25.0]), az7[::2])
    report("(a1) ground ring, one range (el=-30)", ring1, u_t)
    report("(a2) ground rings 6/12/25 m", rings, u_t)

    # (b) rings + a vertical structure (elevation diversity above horizon-ish)
    struct = np.array([los_from_azel(np.deg2rad(a), np.deg2rad(e))
                       for a, e in [(35, -5), (35, -12), (-40, -8)]])
    report("(b) rings + 3 structure anchors", np.vstack([rings, struct]), u_t)

    # (c) rings + one corner reflector at increasing angular separation
    for az_cr, el_cr in [(0, -30), (30, -20), (-45, -10)]:
        cr = los_from_azel(np.deg2rad(az_cr), np.deg2rad(el_cr))[None, :]
        report(f"(c) rings + corner at az={az_cr:+d} el={el_cr:+d}",
               np.vstack([rings, cr]), u_t)

    # (d) uniform sector reference cases
    for n, el_lo, el_hi, tag in [(12, -60, -5, "el -60..-5"),
                                 (12, -35, -25, "el -35..-25 (narrow)"),
                                 (24, -60, -5, "el -60..-5, N=24")]:
        U = scene_sector(n, np.deg2rad(120), np.deg2rad(el_lo),
                         np.deg2rad(el_hi), rng)
        report(f"(d) sector {tag}", U, u_t)

    # (e) target-anchor separation sweep: target inside vs outside the cluster
    print("\n(e) CM-leak & DOP(u_t) vs target offset from anchor-cluster centre:")
    U = scene_sector(12, np.deg2rad(90), np.deg2rad(-45), np.deg2rad(-15),
                     np.random.default_rng(7))
    for off in [0, 10, 20, 30, 45, 60]:
        ut2 = los_from_azel(np.deg2rad(off), np.deg2rad(-30))
        print(f"   target az offset {off:>2d} deg:  DOP(u_t)="
              f"{target_dop(U, ut2):5.2f}  CM-leak="
              f"{common_mode_rejection(U, ut2):6.3f}")


if __name__ == "__main__":
    main()
