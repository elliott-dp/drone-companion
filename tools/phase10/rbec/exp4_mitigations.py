"""Experiment 4: the mitigations and the leakage topologies (follow-ups 1-2).

(a) The unwrap cliff re-measured with the chest-velocity prior and the
cycle-slip repair active — does the ~100 um IMU requirement relax?
(b) The two leakage topologies the review flagged: a single contaminated
anchor (worst T8 topology — the leak biases one equation instead of being
partially common-mode) and anchor->target reverse leakage (anchors are
stronger than the chest echo).
"""

from __future__ import annotations

import numpy as np

from .core import CARDIAC_RESIDUAL_BUDGET_RAD
from .endtoend import SimConfig, run

SEEDS = list(range(1, 9))


def sweep(label: str, cfgs: list[tuple[str, SimConfig]]) -> None:
    print(f"\n== {label} ==")
    print(f"{'case':<44s} {'resp err':>9s} {'card err':>9s} {'card err':>9s} "
          f"{'int-fail':>12s} {'repairs':>8s}")
    print(f"{'':<44s} {'[um]':>9s} {'[um]':>9s} {'[rad]':>9s} "
          f"{'t+a/seams':>12s}")
    for name, cfg in cfgs:
        rs = [run(cfg, s) for s in SEEDS]
        resp = np.array([r.resp_err_rms_m for r in rs]) * 1e6
        card = np.array([r.cardiac_err_rms_m for r in rs]) * 1e6
        cardr = np.array([r.cardiac_err_rms_rad for r in rs])
        f_t = sum(r.integer_fail_target for r in rs)
        f_a = sum(r.integer_fail_anchor for r in rs)
        seams = sum(r.n_seams for r in rs)
        reps = sum(r.extras.get("n_repairs", 0) for r in rs)
        flag = " <-- BUDGET FAIL" \
            if cardr.mean() > CARDIAC_RESIDUAL_BUDGET_RAD else ""
        print(f"{name:<44s} {resp.mean():>8.1f} {card.mean():>8.1f} "
              f"{cardr.mean():>9.4f} {f_t:>4d}t+{f_a:<5d}/{seams:<6d} "
              f"{reps:>7d}{flag}")


def main() -> None:
    print("RBEC exp4: mitigations and leakage topologies "
          f"({len(SEEDS)} seeds; budget {CARDIAC_RESIDUAL_BUDGET_RAD:.3f} rad)")

    # (a) the cliff, with and without mitigations
    for imu in [50e-6, 200e-6, 500e-6]:
        sweep(f"unwrap mitigations at IMU {imu*1e6:.0f} um/gap", [
            ("bare", SimConfig(imu_gap_sigma_m=imu)),
            ("chest prior", SimConfig(imu_gap_sigma_m=imu, chest_prior=True)),
            ("RAIM seam", SimConfig(imu_gap_sigma_m=imu, raim_seam=True)),
            ("prior + RAIM", SimConfig(imu_gap_sigma_m=imu, chest_prior=True,
                                       raim_seam=True)),
            ("prior + RAIM + repair",
             SimConfig(imu_gap_sigma_m=imu, chest_prior=True, raim_seam=True,
                       slip_repair=True)),
        ])

    # (b) leakage topologies at -20 dB (amplitude 0.1) and -10 dB (0.316)
    sweep("leakage topology (all anchors vs single anchor)", [
        ("all anchors, -20 dB", SimConfig(leak_amp_ratio=0.1)),
        ("single anchor, -20 dB",
         SimConfig(leak_amp_ratio=0.1, leak_topology="single")),
        ("all anchors, -10 dB", SimConfig(leak_amp_ratio=0.316)),
        ("single anchor, -10 dB",
         SimConfig(leak_amp_ratio=0.316, leak_topology="single")),
    ])

    sweep("anchor->target reverse leakage (anchor 10 dB stronger)", [
        ("none", SimConfig()),
        ("-30 dB sidelobe -> ratio 0.1",
         SimConfig(target_leak_ratio=0.1)),
        ("-20 dB sidelobe -> ratio 0.316",
         SimConfig(target_leak_ratio=0.316)),
    ])


if __name__ == "__main__":
    main()
