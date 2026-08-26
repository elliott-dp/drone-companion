"""exp9 (thesis P3 + exp5b D.9): joint anchor-redundancy availability.

Three mechanisms spend the same anchor redundancy, and until now each was
validated alone: dwell-level subset CONSENSUS (exp7: ghost exclusion),
per-seam SEAM-RAIM (exp4: the anchors' shared-IMU-error solve that removed
the unwrap cliff), and — new here, mandated by exp5b Part G's finding that
integer misses are ABSORBING (one miss injects lambda/2 into every later
seed on that track) — D.9 per-seam integer-chain RAIM: robust consensus
over the seam innovations that (a) excludes ghosts and slipped anchors
from the shared-error solve and (b) snaps a detected 2*pi*m slip back,
de-absorbing the chain.

The question (thesis P3): the combined anchor-count requirement, "unknown,
may exceed 9". Answered as a measured availability surface over
  N anchors x ghost count x per-gap IMU sigma x {plain seam-RAIM, D.9},
100 seeded dwells per cell, hover design geometry (30 s at 21 Hz = the
47 ms inter-burst gap), exp7's scene/noise/T3 conventions. A dwell is
AVAILABLE when the target integer chain closes error-free, the dwell-level
consensus retains a rank-3 set of >= 4 anchors, and the cardiac in-band
residual meets the 0.110 rad budget.

Model boundaries (deliberate, stated):
  * Frame-level abstraction: within-burst unwrap is assumed clean (exp3/4
    showed the seams are the failure locus); one phase sample per frame.
  * Ghosts are exp7's mis-attribution (phase follows a parent LOS 25 deg
    away); their innovations are continuous outliers, not 2*pi slips —
    D.9 must EXCLUDE them, while genuinely slipped good anchors must be
    CORRECTED. Both behaviors are asserted in the self-test.
  * The IMU per-gap error is shared across anchors (that is what
    seam-RAIM solves for); sigma per axis on the exp4 ladder.
  * No APLL steps / leakage here — exp3/exp4 carried those; this
    experiment isolates the redundancy economics.

Deterministic (seeded); bundle ``docs/phase10/results/exp9.json``
regenerated exactly by ``--check``.

Usage:
    python3 -m tools.phase10.rbec.exp9_availability            # bundle+fig
    python3 -m tools.phase10.rbec.exp9_availability --check
    python3 -m tools.phase10.rbec.exp9_availability --self-test
"""

from __future__ import annotations

import json
import os
import sys

import numpy as np

from .core import (CARDIAC_BAND, CARDIAC_RESIDUAL_BUDGET_RAD, K_DISP,
                   band_rms, los_from_azel, shaped_noise)
from .exp5b_upgrade import _compare

RESULTS_DIR = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "..", "docs", "phase10", "results"))

FRAME_HZ = 21.0                 # 47 ms inter-burst gap (design point)
DWELL_S = 30.0
SIGMA_PHI = 0.0115              # per-frame anchor phase noise (exp7)
SIGMA_THETA_DEG = 0.1           # T3 believed-LOS error (exp7 convention)
GHOST_OFFSET_DEG = 25.0         # exp7's ghost parent offset
SWAY_RMS_M = 0.02               # per axis, knee 0.3 Hz (exp4 hover)
D9_TOL_MULT = 6.0               # innovation gate, in sigma of the model
D9_DRAWS = 60                   # minimal 3-subsets per seam

N_GRID = [5, 6, 7, 9, 12, 15]
GHOSTS = [0, 1, 2]
IMU_SIGMA_UM = [100, 300, 450]
N_DWELLS = 100


def scene(n_anchors: int, n_ghost: int, seed: int):
    """exp7's hover scene split into believed / phase-generating LOS."""
    rng = np.random.default_rng(seed)
    az = np.deg2rad(np.linspace(-50, 50, n_anchors))
    el = np.deg2rad(rng.uniform(-45, -15, n_anchors))
    U_true = np.array([los_from_azel(a, e) for a, e in zip(az, el)])
    perr = np.deg2rad(SIGMA_THETA_DEG) * rng.standard_normal((n_anchors, 2))
    U_est = np.array([los_from_azel(a + da, e + de) for (a, e), (da, de)
                      in zip(zip(az, el), perr)])
    U_phase = U_true.copy()
    for g in range(n_ghost):                 # ghosts at the tail (exp7)
        k = n_anchors - 1 - g
        U_phase[k] = los_from_azel(az[k] + np.deg2rad(GHOST_OFFSET_DEG),
                                   el[k])
    u_t = los_from_azel(0.0, np.deg2rad(-30.0))
    return U_est, U_phase, u_t, rng


def run_dwell(n_anchors: int = 9, n_ghost: int = 0,
              imu_sigma_m: float = 300e-6, d9: bool = True,
              seed: int = 0) -> dict:
    """One dwell of the seam chain + dwell-level solve. Returns the
    availability verdict and its components."""
    U_est, U_phase, u_t, rng = scene(n_anchors, n_ghost, seed)
    N = n_anchors
    F = int(DWELL_S * FRAME_HZ)

    d = np.stack([shaped_noise(F, FRAME_HZ, 0.3, SWAY_RMS_M, rng)
                  for _ in range(3)], axis=1)          # (F, 3)
    chest = 0.1e-3 * np.sin(2 * np.pi * 1.17 *
                            np.arange(F) / FRAME_HZ)   # cardiac only
    phi_true = K_DISP * (U_phase @ d.T)                # (N, F)
    phi = phi_true + SIGMA_PHI * rng.standard_normal((N, F))
    phi_t = K_DISP * (u_t @ d.T + chest) \
        + SIGMA_PHI * rng.standard_normal(F)

    wrap = lambda x: (x + np.pi) % (2 * np.pi) - np.pi
    imu_err = imu_sigma_m * rng.standard_normal((F, 3))

    # D.9 subset pool: drawn once per dwell, inverses precomputed
    rs = np.random.default_rng(seed * 100003 + 1)
    subsets = np.empty((0, 3), dtype=int)
    if d9:
        cand = np.array([rs.choice(N, size=3, replace=False)
                         for _ in range(D9_DRAWS)])
        dets = np.abs(np.linalg.det(U_est[cand]))
        cand = cand[dets > 1e-3]
        subsets = cand
        sub_inv = np.linalg.inv(U_est[subsets])

    # seam chain: anchor tracks then target, per frame
    out_a = np.empty((N, F))
    out_a[:, 0] = phi[:, 0]
    out_t = np.empty(F)
    out_t[0] = phi_t[0]
    A3 = U_est                                          # (N, 3)
    n_target_fail = 0
    n_snap = 0
    excl_frac = np.zeros(N)
    err_hat_err = []                        # |err_hat - true imu error|

    for f in range(1, F):
        delta_true = d[f] - d[f - 1]
        delta_imu = delta_true + imu_err[f]
        # a-priori innovation gate: pair noise + the T3 term at THIS
        # seam's increment scale (exp7's variance model, per seam)
        tol_innov = D9_TOL_MULT * (
            np.sqrt(2.0) * SIGMA_PHI
            + K_DISP * np.deg2rad(SIGMA_THETA_DEG)
            * float(np.linalg.norm(delta_imu)))
        meas = wrap(phi[:, f])
        pred = out_a[:, f - 1] + K_DISP * (A3 @ delta_imu)
        n_cyc = np.round((pred - meas) / (2 * np.pi))
        fixed = meas + 2 * np.pi * n_cyc
        innov = fixed - pred                            # (N,)

        keep = np.ones(N, dtype=bool)
        if d9:
            # robust consensus over minimal 3-subsets of the innovations:
            # innov_k ~ -K (err . u_k); inliers agree with the common
            # err. Subsets are drawn once per dwell and their inverses
            # precomputed (batched); the RAW residual is deliberately
            # unwrapped — a slipped anchor sits at r ~ 2*pi*m, which a
            # wrapped statistic would alias back into the inlier set and
            # feed its raw -2*pi-offset innovation to the LS.
            errs = -np.einsum("dij,dj->di", sub_inv,
                              innov[subsets]) / K_DISP     # (D, 3)
            r_all = innov[None, :] \
                + K_DISP * (errs @ A3.T)                   # (D, N)
            scores = (np.abs(r_all) < tol_innov).sum(axis=1)
            bi = int(np.argmax(scores))
            best_in = np.abs(r_all[bi]) < tol_innov
            if best_in.sum() >= 3:
                err0, *_ = np.linalg.lstsq(A3[best_in],
                                           -innov[best_in] / K_DISP,
                                           rcond=None)
                r = innov + K_DISP * (A3 @ err0)
                m = np.round(r / (2 * np.pi))
                # slipped good anchor: residual is ~2*pi*m from consensus
                # -> snap the chain back (de-absorb); continuous outlier
                # (ghost): exclude from the shared-error solve
                snap = (~best_in) & (m != 0) \
                    & (np.abs(r - 2 * np.pi * m) < tol_innov)
                n_cyc[snap] -= m[snap]
                fixed = meas + 2 * np.pi * n_cyc
                innov = fixed - pred
                n_snap += int(snap.sum())
                keep = best_in | snap
        excl_frac += ~keep

        # seam-RAIM: shared-error estimate from the kept anchors
        if keep.sum() >= 3:
            err_hat, *_ = np.linalg.lstsq(A3[keep], -innov[keep] / K_DISP,
                                          rcond=None)
        else:
            err_hat = np.zeros(3)
        err_hat_err.append(float(np.linalg.norm(err_hat - imu_err[f])))
        delta_for_target = delta_imu - err_hat

        out_a[:, f] = fixed
        meas_t = wrap(phi_t[f])
        pred_t = out_t[f - 1] + K_DISP * (delta_for_target @ u_t)
        n_t = np.round((pred_t - meas_t) / (2 * np.pi))
        true_t = np.round(((out_t[f - 1]
                            + K_DISP * (delta_true @ u_t)
                            + K_DISP * (chest[f] - chest[f - 1]))
                           - meas_t) / (2 * np.pi))
        if n_t != true_t:
            n_target_fail += 1
        out_t[f] = meas_t + 2 * np.pi * n_t

    # dwell-level consensus (exp7, 3-D) on the unwrapped anchor tracks
    Y = out_a / K_DISP
    x_all = np.linalg.lstsq(A3, Y, rcond=None)[0]
    d_rms = float(np.sqrt(np.mean(np.sum(x_all ** 2, axis=0))))
    t3 = K_DISP * np.deg2rad(SIGMA_THETA_DEG) * d_rms
    tol = 4.0 * np.sqrt(SIGMA_PHI ** 2 + t3 ** 2) / K_DISP
    rs = np.random.default_rng(seed + 77)
    cols = np.linspace(0, F - 1, min(F, 120)).astype(int)
    best_keep, best_score = None, -1
    for _ in range(200):
        idx = rs.choice(N, size=4, replace=False)
        if np.linalg.matrix_rank(A3[idx], tol=1e-9) < 3:
            continue
        x = np.linalg.lstsq(A3[idx], Y[np.ix_(idx, cols)], rcond=None)[0]
        r = Y[:, cols] - A3 @ x
        agree = np.sqrt((r ** 2).mean(axis=1)) < tol
        if agree.sum() > best_score:
            best_score, best_keep = int(agree.sum()), agree.copy()
    ck = best_keep if best_keep is not None and best_keep.sum() >= 4 \
        else np.ones(N, dtype=bool)
    rank_ok = np.linalg.matrix_rank(A3[ck], tol=1e-9) >= 3
    if rank_ok and ck.sum() >= 4:
        x = np.linalg.lstsq(A3[ck], Y[ck], rcond=None)[0]
    else:
        x = x_all
    err_series = K_DISP * (u_t @ x - u_t @ d.T)
    cardiac = band_rms(err_series, FRAME_HZ, CARDIAC_BAND)

    ok = (n_target_fail == 0) and rank_ok and ck.sum() >= 4 \
        and cardiac < CARDIAC_RESIDUAL_BUDGET_RAD
    return {"available": bool(ok),
            "target_fail": int(n_target_fail),
            "cardiac_rad": float(cardiac),
            "consensus_set": int(ck.sum()),
            "snaps": int(n_snap),
            "seam_excl_mean": float(excl_frac.mean() / (F - 1)),
            "err_hat_err_um": float(np.mean(err_hat_err) * 1e6)}


def cell(n: int, g: int, sig_um: int, d9: bool, seeds=range(N_DWELLS)):
    rs = [run_dwell(n, g, sig_um * 1e-6, d9, seed=s) for s in seeds]
    return {
        "availability": float(np.mean([r["available"] for r in rs])),
        "p_int_clean": float(np.mean([r["target_fail"] == 0
                                      for r in rs])),
        "cardiac_med": float(np.median([r["cardiac_rad"] for r in rs])),
        "set_med": float(np.median([r["consensus_set"] for r in rs])),
        "snaps_mean": float(np.mean([r["snaps"] for r in rs])),
    }


def build() -> dict:
    out = {"budget": CARDIAC_RESIDUAL_BUDGET_RAD, "n_grid": N_GRID,
           "ghosts": GHOSTS, "imu_sigma_um": IMU_SIGMA_UM,
           "n_dwells": N_DWELLS, "frame_hz": FRAME_HZ}
    surf = {}
    for arm in ("d9", "plain"):
        for g in GHOSTS:
            for sig in IMU_SIGMA_UM:
                key = f"{arm}_g{g}_s{sig}"
                surf[key] = {str(n): cell(n, g, sig, arm == "d9")
                             for n in N_GRID}
                print(f"cell {key}: " + " ".join(
                    f"N{n}={surf[key][str(n)]['availability']:.2f}"
                    for n in N_GRID))
    out["surface"] = surf
    # the doctrine table: min N for >=0.99 availability
    min_n = {}
    for key, row in surf.items():
        m = next((n for n in N_GRID
                  if row[str(n)]["availability"] >= 0.99), None)
        min_n[key] = -1 if m is None else int(m)
    out["min_n_99"] = min_n
    return out


def make_figure(data: dict, path: str) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, axs = plt.subplots(1, 3, figsize=(16.5, 4.8))
    surf = data["surface"]
    nn = data["n_grid"]

    for ax, sig in zip(axs, data["imu_sigma_um"]):
        for g, ls in zip(data["ghosts"], ["-", "--", ":"]):
            for arm, color in (("d9", "tab:blue"), ("plain", "tab:red")):
                y = [surf[f"{arm}_g{g}_s{sig}"][str(n)]["availability"]
                     for n in nn]
                ax.plot(nn, y, ls, color=color, lw=1.6,
                        label=f"{arm}, {g} ghost(s)" if sig ==
                        data["imu_sigma_um"][0] else None)
        ax.axhline(0.99, ls=":", color="dimgray")
        ax.set_ylim(-0.03, 1.05)
        ax.set_xlabel("anchor count N")
        ax.set_title(f"IMU per-gap sigma {sig} um")
    axs[0].set_ylabel("dwell availability")
    axs[0].legend(fontsize=8, loc="lower right")
    fig.suptitle("Availability: D.9 integer-chain RAIM (blue) vs plain "
                 "seam-RAIM (red); dotted line = 99 %")
    fig.tight_layout()
    fig.savefig(path, dpi=130)
    plt.close(fig)


def _self_test() -> None:
    # (1) clean case: N=9, no ghosts, 100 um -> available with D.9
    r = run_dwell(9, 0, 100e-6, True, seed=0)
    assert r["available"], r
    # (2) mechanism: with 2 ghosts, D.9 excludes them per seam (~2/9 of
    # anchors) and its shared-error estimate is measurably better than
    # the plain arm's ghost-biased one; at 300 um both arms share the
    # lambda/4 wall for target failures, so the discriminator is the
    # estimate quality, not the wall-dominated failure count
    eh_p = np.mean([run_dwell(9, 2, 150e-6, False, s)["err_hat_err_um"]
                    for s in range(6)])
    eh_d = np.mean([run_dwell(9, 2, 150e-6, True, s)["err_hat_err_um"]
                    for s in range(6)])
    assert eh_d < eh_p, (eh_d, eh_p)
    r2 = run_dwell(9, 2, 150e-6, True, seed=0)
    assert 0.12 < r2["seam_excl_mean"] < 0.4, r2["seam_excl_mean"]
    r2p = run_dwell(9, 2, 150e-6, False, seed=0)
    assert r2p["seam_excl_mean"] == 0.0
    # (3) D.9 snaps isolated slips at moderate sigma (its design case);
    # at the 450 um wall anchors wrap COHERENTLY and the shared error's
    # 2*pi branch ambiguity defeats innovation-only RAIM — asserted as
    # the honest boundary (few/no snaps, poor availability, both arms)
    snaps = sum(run_dwell(9, 0, 300e-6, True, seed=s)["snaps"]
                for s in range(6))
    assert snaps > 0, snaps
    rw = run_dwell(9, 0, 450e-6, True, seed=1)
    assert not rw["available"], rw
    # (4) determinism
    a = run_dwell(9, 1, 300e-6, True, seed=3)
    b = run_dwell(9, 1, 300e-6, True, seed=3)
    assert a == b
    print(f"exp9 self_test OK: clean available, ghost err_hat "
          f"plain/d9 {eh_p:.0f}/{eh_d:.0f} um, seam exclusion "
          f"{r2['seam_excl_mean']:.2f}, snaps at 300 um {snaps}, "
          f"wall unavailable as expected, deterministic")


def main() -> None:
    check = "--check" in sys.argv
    if "--self-test" in sys.argv:
        _self_test()
        return
    os.makedirs(RESULTS_DIR, exist_ok=True)
    json_path = os.path.join(RESULTS_DIR, "exp9.json")
    fig_path = os.path.join(RESULTS_DIR, "fig_rbec_exp9.png")
    data = build()
    if check:
        with open(json_path) as fh:
            committed = json.load(fh)
        bad = _compare(data, committed)
        if bad:
            print(f"MISMATCH ({len(bad)} values):")
            for m in bad[:20]:
                print(" ", m)
            sys.exit(1)
        print(f"exp9_availability --check: regenerated surface matches "
              f"{os.path.relpath(json_path)} exactly")
        return
    with open(json_path, "w") as fh:
        json.dump(data, fh, indent=1)
    print(f"wrote {json_path}")
    try:
        make_figure(data, fig_path)
        print(f"wrote {fig_path}")
    except ImportError:
        print("matplotlib not available -- figure skipped, JSON written")


if __name__ == "__main__":
    main()
