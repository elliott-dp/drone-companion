"""Experiment 3: end-to-end deterministic RBEC solve on synthetic hover data.

Sweeps the terms the paper names as pacing (anchor angle error T3, leakage
T8, IMU seam quality, APLL step size T6) and reports in-band residual error
against the budgets: cardiac residual < 0.110 rad (= cardiac/3), respiration
residual << 4 mm. Every row is a mean over seeds with the worst seed shown.
"""

from __future__ import annotations

import numpy as np

from .core import CARDIAC_RESIDUAL_BUDGET_RAD, K_DISP
from .endtoend import SimConfig, run

SEEDS = list(range(1, 9))


def sweep(label: str, cfgs: list[tuple[str, SimConfig]],
          steps_only: bool = False) -> None:
    print(f"\n== {label} ==")
    print(f"{'case':<38s} {'resp err':>10s} {'card err':>10s} "
          f"{'card err':>10s} {'int-fail':>9s} {'CMRR':>7s}")
    print(f"{'':<38s} {'[um]':>10s} {'[um]':>10s} {'[rad]':>10s} "
          f"{'/seams':>9s} {'[dB]':>7s}")
    for name, cfg in cfgs:
        rs = [run(cfg, s, steps_only=steps_only) for s in SEEDS]
        resp = np.array([r.resp_err_rms_m for r in rs]) * 1e6
        card = np.array([r.cardiac_err_rms_m for r in rs]) * 1e6
        cardr = np.array([r.cardiac_err_rms_rad for r in rs])
        f_t = sum(r.integer_fail_target for r in rs)
        f_a = sum(r.integer_fail_anchor for r in rs)
        seams = sum(r.n_seams for r in rs)
        cmrr = [r.step_cmrr_db for r in rs if r.step_cmrr_db is not None]
        cm = f"{np.mean(cmrr):7.1f}" if cmrr else "      -"
        flag = " <-- BUDGET FAIL" if cardr.mean() > CARDIAC_RESIDUAL_BUDGET_RAD \
            else ""
        print(f"{name:<38s} {resp.mean():>7.1f}±{resp.std():<4.0f}"
              f"{card.mean():>7.1f}±{card.std():<4.0f}"
              f"{cardr.mean():>10.4f} {f_t:>3d}t+{f_a:<4d}a/{seams:<6d} "
              f"{cm}{flag}")


def main() -> None:
    base = SimConfig()
    print("RBEC exp3: end-to-end deterministic solve, 30 s dwell, 20 Hz frames,"
          f" {base.n_anchors} anchors, {len(SEEDS)} seeds")
    print(f"budgets: cardiac residual < {CARDIAC_RESIDUAL_BUDGET_RAD:.3f} rad "
          f"(= {CARDIAC_RESIDUAL_BUDGET_RAD/K_DISP*1e6:.0f} um in band)")

    # T3: anchor angle error is claimed the pacing term
    sweep("T3: anchor angle error (deg, 1-sigma; sway 2 cm RMS/axis)", [
        (f"angle err {a:.3f} deg",
         SimConfig(anchor_angle_err_deg=a)) for a in
        [0.0, 0.02, 0.05, 0.1, 0.3, 0.675]
    ])

    # plain LS vs angle-aware WLS at the pacing point; WLS exists for
    # heterogeneous anchors, so also test with a corner reflector present
    sweep("estimator: plain LS vs angle-aware WLS (angle err 0.3 deg)", [
        ("plain LS", SimConfig(anchor_angle_err_deg=0.3,
                               use_angle_aware_wls=False)),
        ("angle-aware WLS", SimConfig(anchor_angle_err_deg=0.3,
                                      use_angle_aware_wls=True)),
        ("corner + plain LS",
         SimConfig(anchor_angle_err_deg=0.3, corner_anchor=True,
                   use_angle_aware_wls=False)),
        ("corner + angle-aware WLS",
         SimConfig(anchor_angle_err_deg=0.3, corner_anchor=True,
                   use_angle_aware_wls=True)),
    ])

    # sway scaling (T3 couples with excursion)
    sweep("sway RMS per axis (angle err 0.1 deg)", [
        (f"sway {s*100:.0f} cm RMS",
         SimConfig(sway_rms_m=s, anchor_angle_err_deg=0.1)) for s in
        [0.005, 0.02, 0.05]
    ])

    # T8: leakage, including the paper's fatal weak-anchor case (-10 dB)
    sweep("T8: casualty->anchor leakage (amplitude ratio)", [
        ("leak 0 (off)", SimConfig(leak_amp_ratio=0.0)),
        ("leak -40 dB", SimConfig(leak_amp_ratio=0.01)),
        ("leak -30 dB", SimConfig(leak_amp_ratio=0.0316)),
        ("leak -20 dB", SimConfig(leak_amp_ratio=0.1)),
        ("leak -10 dB (weak anchor)", SimConfig(leak_amp_ratio=0.316)),
    ])

    # IMU seam quality (integer fixing)
    sweep("IMU per-gap displacement error (T-unwrap)", [
        (f"imu {s*1e6:.0f} um/gap", SimConfig(imu_gap_sigma_m=s)) for s in
        [20e-6, 50e-6, 200e-6, 500e-6, 950e-6]
    ])

    # anchor count
    sweep("anchor count (angle err 0.1 deg)", [
        (f"N={n}", SimConfig(n_anchors=n, anchor_angle_err_deg=0.1))
        for n in [4, 6, 9, 16]
    ])

    # T6: APLL steps, common-mode rejection. Review fix: run at very high
    # SNR with vibration lines off so the CMRR is not noise-floored — the
    # residual ceiling is then the GEOMETRIC leak of the common step through
    # the solve (|1 - u_t.g|, exp2's CM-leak), which is the number to report.
    sweep("T6: APLL common step rejection (steps only, high SNR)", [
        (f"step {s*1e3:.0f} mrad, mism {m:.2f}",
         SimConfig(apll_step_rad=s, beam_chip_mismatch=m, sway_rms_m=0.0,
                   anchor_snr_db=80.0, target_snr_db=80.0, vib_lines=()))
        for s, m in [(0.05, 0.1), (0.2, 0.1), (1.0, 0.1), (0.2, 0.3)]
    ], steps_only=True)

    # T6 in context: steps + motion together vs no steps
    sweep("T6 in context (full sim, angle err 0.1 deg)", [
        ("no steps", SimConfig(apll_step_rad=0.0, chip_step_rad=0.0,
                               anchor_angle_err_deg=0.1)),
        ("steps 50 mrad/5 mrad", SimConfig(anchor_angle_err_deg=0.1)),
        ("steps 200 mrad/20 mrad",
         SimConfig(apll_step_rad=0.2, chip_step_rad=0.02,
                   anchor_angle_err_deg=0.1)),
    ])

    # combined worst-realistic case: every stressor at its plausible worst
    sweep("combined worst-realistic", [
        ("0.3deg + 5cm sway + -30dB + 200mrad",
         SimConfig(anchor_angle_err_deg=0.3, sway_rms_m=0.05,
                   leak_amp_ratio=0.0316, apll_step_rad=0.2,
                   chip_step_rad=0.02)),
        ("same + corner anchor",
         SimConfig(anchor_angle_err_deg=0.3, sway_rms_m=0.05,
                   leak_amp_ratio=0.0316, apll_step_rad=0.2,
                   chip_step_rad=0.02, corner_anchor=True)),
    ])


if __name__ == "__main__":
    main()
