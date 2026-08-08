"""Experiment 1: does per-chip correlation break the sigma^2/K sidelobe floor?

Post-review version. Compares anchor-beam leakage for i.i.d. vs
chip-correlated errors at MATCHED total per-element variance
(sigma_iid = sqrt(sigma_tot^2 - 2 sigma_chip^2)), with a FIXED chip map per
run, mean-of-power statistics, the taper-aware analytic floor
sigma^2 sum(w^2)/(sum w)^2, an off-boresight steered case, and — the
observable the first version lacked — a fine sin-space spur scan of the
error-excess power, where a fixed map's block structure produces discrete
spurs the offset table cannot see.
"""

from __future__ import annotations

import numpy as np

from .array_model import N_AZ, leakage_stats, spur_scan

OFFSETS = np.array([2.0, 3.0, 5.0, 10.0, 20.0])
N_MC = 800


def sig_iid_for(total_deg: float, chip_deg: float) -> float:
    return float(np.sqrt(total_deg ** 2 - 2 * chip_deg ** 2))


def main() -> None:
    print("RBEC exp1 (post-review): anchor-beam leakage under calibration errors")
    print(f"array: {N_AZ}-element lambda/2 ULA; MC draws: {N_MC}; "
          "variance-matched cases; fixed chip map per run\n")

    cases = [
        # label, taper, sigma_tot_deg, sigma_chip_deg, permute, steer_deg
        ("uniform, iid tot=2.0",           "uniform",  2.0, 0.0, False, 0.0),
        ("kaiser30, iid tot=2.0",          "kaiser30", 2.0, 0.0, False, 0.0),
        ("kaiser30, chip 0.8, tot=2.0",    "kaiser30", 2.0, 0.8, False, 0.0),
        ("kaiser30, chip 0.8 permuted",    "kaiser30", 2.0, 0.8, True, 0.0),
        ("kaiser30, iid tot=3.0",          "kaiser30", 3.0, 0.0, False, 0.0),
        ("kaiser30, chip 1.2, tot=3.0",    "kaiser30", 3.0, 1.2, False, 0.0),
        ("kaiser40, iid tot=2.0",          "kaiser40", 2.0, 0.0, False, 0.0),
        ("kaiser30, iid tot=2.0, steer40", "kaiser30", 2.0, 0.0, False, 40.0),
    ]
    hdr = "offset[deg]:" + "".join(f"{o:>9.0f}" for o in OFFSETS)
    for label, tap, s_tot, s_chip, perm, steer in cases:
        rng = np.random.default_rng(20260808)
        s_iid = sig_iid_for(s_tot, s_chip)
        st = leakage_stats(rng, s_iid, s_chip, tap, OFFSETS,
                           n_mc=N_MC, permute_map=perm, steer_deg=steer)
        print(f"-- {label}  (sigma_tot {s_tot:.2f} deg; analytic iid floor "
              f"{st['analytic_iid_floor_db']:.1f} dB)")
        print(hdr)
        print("mean  [dB]:" + "".join(f"{v:>9.1f}" for v in st["leak_mean_db"]))
        print("p90   [dB]:" + "".join(f"{v:>9.1f}" for v in st["leak_p90_db"]))
        print(f"far floor (mean power): {st['floor_mean_db']:.1f} dB\n")

    print("== spur scan: error-excess power, fixed default map vs fixed "
          "permuted map (chip-only errors expose the block structure) ==")
    for label, s_iid, s_chip, perm in [
            ("default map, chip-only 0.8 deg", 0.0, 0.8, False),
            ("permuted map, chip-only 0.8 deg", 0.0, 0.8, True),
            ("default map, mixed tot=2.0/chip=0.8",
             sig_iid_for(2.0, 0.8), 0.8, False)]:
        rng = np.random.default_rng(20260808)
        sc = spur_scan(rng, s_iid, s_chip, "kaiser30", n_mc=400,
                       permute_map=perm)
        spurs = ", ".join(f"{d:.1f} dB @ {a:.2f} deg (sin={s:.3f})"
                          for d, a, s in zip(sc["spur_db"], sc["spur_deg"],
                                             sc["spur_sin"]))
        print(f"{label}:\n  top spurs: {spurs}\n  median excess: "
              f"{sc['median_excess_db']:.1f} dB")


if __name__ == "__main__":
    main()
