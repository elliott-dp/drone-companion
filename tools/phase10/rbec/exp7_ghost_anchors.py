"""Experiment 7: ghost anchors — the error class the C.4 gates must catch.

exp5's real-data run recorded an honest caveat: the anchor picker has no
quality gates and "identical-range anchor clusters look like ring artifacts".
That is not a small-angle error. A sidelobe/ring ghost of a strong scatterer
carries the phase of its PARENT: its true sensitivity vector is u_parent while
the solve assigns it u_ghost, wrong by TENS of degrees. Every error term in
exp1-exp4 is a fraction of a degree (T3) or a leakage amplitude (T8); this is
a wrong-equation error, unmodelled anywhere in the validation stack.

Three questions:
  1. How much does one ghost cost the LOS prediction, vs one T3-perturbed
     anchor at the same count?
  2. Do the C.4 gates actually catch it? The gates are amplitude-dispersion
     (D_A), residual-based exclusion (RAIM), and co-range clustering. A ghost
     is phase-COHERENT with its parent, so D_A does NOT flag it -- it is a
     stable scatterer by every amplitude test. The discriminator has to be
     the LS residual or the co-range/co-Doppler signature.
  3. Does the standard robust estimator (IRLS / RANSAC-style exclusion)
     recover the budget, and how many ghosts can it absorb?

Usage:
    python3 -m tools.phase10.rbec.exp7_ghost_anchors
"""

from __future__ import annotations

import numpy as np

from .core import (CARDIAC_BAND, CARDIAC_RESIDUAL_BUDGET_RAD, K_DISP,
                   band_rms, los_from_azel, shaped_noise)

FS = 20.0
DWELL_S = 30.0


def scene(n_anchors: int = 9, h: float = 10.0, seed: int = 5):
    """Hover scene: anchors spread in az/el, casualty at el -30 deg."""
    rng = np.random.default_rng(seed)
    az = np.deg2rad(np.linspace(-50, 50, n_anchors))
    el = np.deg2rad(rng.uniform(-45, -15, n_anchors))
    U = np.array([los_from_azel(a, e) for a, e in zip(az, el)])
    return U, los_from_azel(0.0, np.deg2rad(-30.0))


def run_dwell(n_ghost: int = 0, ghost_offset_deg: float = 25.0,
              n_anchors: int = 9, sigma_az_deg: float = 0.1,
              sigma_el_deg: float = 0.1, sway_rms_m: float = 0.02,
              sigma_phi: float = 0.0115, seed: int = 0,
              robust: str = "none") -> dict:
    """One dwell with ``n_ghost`` of the anchors replaced by ghosts.

    A ghost at index k: the solve believes its LOS is U_est[k], but the phase
    it actually reports is that of its parent — a strong scatterer
    ``ghost_offset_deg`` away in azimuth. (Amplitude is irrelevant here: the
    solve consumes phases, and a ghost above the noise floor reports its
    parent's phase at full weight.)

    robust: 'none'  -> plain LS over all anchors
            'irls'  -> Huber IRLS on the per-frame residuals
            'raim'  -> iterative max-normalised-residual exclusion
    """
    rng = np.random.default_rng(seed + 4200)
    n = int(DWELL_S * FS)
    U_true, u_t = scene(n_anchors)
    N = U_true.shape[0]

    # what the solve BELIEVES each anchor's LOS is (T3 angle error)
    az = np.arctan2(U_true[:, 1], U_true[:, 0])
    el = np.arcsin(np.clip(U_true[:, 2], -1, 1))
    U_est = np.array([
        los_from_azel(a + np.deg2rad(sigma_az_deg) * rng.standard_normal(),
                      e + np.deg2rad(sigma_el_deg) * rng.standard_normal())
        for a, e in zip(az, el)])

    # which LOS each anchor's PHASE actually follows
    U_phase = U_true.copy()
    ghost_idx = list(range(N - n_ghost, N))          # ghosts at the tail
    for k in ghost_idx:
        parent_az = az[k] + np.deg2rad(ghost_offset_deg)
        U_phase[k] = los_from_azel(parent_az, el[k])

    d = np.stack([shaped_noise(n, FS, 0.3, sway_rms_m, rng) for _ in range(3)],
                 axis=1)
    phi = K_DISP * (d @ U_phase.T).T + sigma_phi * rng.standard_normal((N, n))
    los_true = d @ u_t

    A = K_DISP * U_est

    if robust == "traim":
        # TEMPORAL RAIM (the design answer this experiment produces).
        # A ghost is not a per-frame outlier: within one frame it biases the
        # whole solve, inflating EVERY residual, so a per-frame MAD threshold
        # never trips (measured: the ghost is rank 1 by dwell-accumulated
        # residual but the per-frame test excludes nothing). The correct
        # statistic is the per-anchor residual RMS accumulated over a window,
        # studentised by leverage -- a ghost's wrong sensitivity vector makes
        # its residual grow with platform excursion, persistently, while a
        # T3-perturbed good anchor stays near the noise floor.
        # Threshold against the KNOWN good-anchor residual scale, not against a
        # MAD of the residuals themselves. Two facts set the design:
        #  (a) With >=2 ghosts a MAD threshold MASKS -- the outliers inflate the
        #      very scale they are compared to and nothing is excluded.
        #  (b) The floor is not sigma_phi alone: a GOOD anchor with T3 angle
        #      error sigma_theta also grows residual with platform excursion,
        #      K*sigma_theta*|d_perp|, which at 0.1 deg and 2 cm/axis sway is
        #      ~0.2 rad -- 3x sigma_phi. Thresholding on sigma_phi alone
        #      therefore discards ~45% of good anchors (measured).
        # Both terms are known a priori (SNR, chirp count, DoA error budget,
        # and the platform excursion the solve itself reports), so the test is
        # absolute -- the same variance model the angle-aware WLS weights use.
        keep = np.ones(N, dtype=bool)
        d_rms = float(np.sqrt(np.mean(np.sum(
            (np.linalg.lstsq(A, phi, rcond=None)[0]) ** 2, axis=0))))
        t3_scale = K_DISP * np.deg2rad(max(sigma_az_deg, sigma_el_deg)) * d_rms
        thresh = max(6.0 * np.sqrt(sigma_phi ** 2 + t3_scale ** 2), 1e-9)
        for _ in range(N - 4):
            Ak = A[keep]
            x = np.linalg.lstsq(Ak, phi[keep], rcond=None)[0]
            r = phi[keep] - Ak @ x
            H = Ak @ np.linalg.pinv(Ak)
            lev = np.clip(np.diag(H), 0.0, 0.99)
            stud = r.std(axis=1) / np.sqrt(1.0 - lev)
            j = int(np.argmax(stud))
            if stud[j] < thresh or keep.sum() <= 4:
                break
            keep[np.flatnonzero(keep)[j]] = False
        x = np.linalg.lstsq(A[keep], phi[keep], rcond=None)[0]
        err = K_DISP * (u_t @ x - los_true)
        gi = [k for k in ghost_idx]
        good = [k for k in range(N) if k not in ghost_idx]
        return {
            "err": err,
            "cardiac_rad": band_rms(err, FS, CARDIAC_BAND),
            "n_excluded": float((~keep).sum()),
            "ghost_retention": float(keep[gi].mean()) if gi else float("nan"),
            "good_retention": float(keep[good].mean()),
        }

    if robust == "consensus":
        # Subset consensus (RANSAC-style), which defeats masking because it
        # never fits the contaminated set: draw minimal 4-anchor subsets, fit,
        # and score by how many anchors agree within the known noise+T3 scale.
        # Greedy exclusion masks at >=2 ghosts because each fit is polluted by
        # the ghosts still in the set; a subset fit that happens to be
        # ghost-free is clean by construction, and the consensus count finds
        # it. This is the estimator the C.4 gates need behind them.
        d_rms = float(np.sqrt(np.mean(np.sum(
            (np.linalg.lstsq(A, phi, rcond=None)[0]) ** 2, axis=0))))
        t3_scale = K_DISP * np.deg2rad(max(sigma_az_deg, sigma_el_deg)) * d_rms
        tol = 4.0 * np.sqrt(sigma_phi ** 2 + t3_scale ** 2)
        rs = np.random.default_rng(seed + 77)
        best_keep, best_score = None, -1
        # subsample frames for scoring speed; the statistic is a dwell RMS
        cols = np.linspace(0, n - 1, min(n, 120)).astype(int)
        for _ in range(200):
            idx = rs.choice(N, size=4, replace=False)
            if np.linalg.matrix_rank(U_est[idx], tol=1e-9) < 3:
                continue
            x = np.linalg.lstsq(A[idx], phi[np.ix_(idx, cols)], rcond=None)[0]
            r = phi[:, cols] - A @ x
            agree = r.std(axis=1) < tol
            if agree.sum() > best_score:
                best_score, best_keep = int(agree.sum()), agree.copy()
        keep = best_keep if best_keep is not None and best_keep.sum() >= 4 \
            else np.ones(N, dtype=bool)
        if keep.sum() >= 4 and np.linalg.matrix_rank(U_est[keep], tol=1e-9) >= 3:
            x = np.linalg.lstsq(A[keep], phi[keep], rcond=None)[0]
        else:
            keep = np.ones(N, dtype=bool)
            x = np.linalg.lstsq(A, phi, rcond=None)[0]
        err = K_DISP * (u_t @ x - los_true)
        good = [k for k in range(N) if k not in ghost_idx]
        return {
            "err": err,
            "cardiac_rad": band_rms(err, FS, CARDIAC_BAND),
            "n_excluded": float((~keep).sum()),
            "ghost_retention": (float(keep[ghost_idx].mean()) if ghost_idx
                                else float("nan")),
            "good_retention": float(keep[good].mean()),
        }

    d_hat = np.empty((3, n))
    n_excluded = np.zeros(n)
    kept_mask = np.ones((N, n), dtype=bool)
    for i in range(n):
        y = phi[:, i]
        if robust == "none":
            d_hat[:, i] = np.linalg.lstsq(A, y, rcond=None)[0]
            continue
        keep = np.ones(N, dtype=bool)
        x = np.linalg.lstsq(A[keep], y[keep], rcond=None)[0]
        if robust == "irls":
            w = np.ones(N)
            for _ in range(6):
                r = y - A @ x
                s = 1.4826 * np.median(np.abs(r - np.median(r))) + 1e-12
                c = 1.345 * s
                w = np.where(np.abs(r) <= c, 1.0, c / np.abs(r))
                Aw = A * w[:, None]
                x = np.linalg.lstsq(Aw, y * w, rcond=None)[0]
            kept_mask[:, i] = w > 0.5
        elif robust == "raim":
            for _ in range(N - 4):
                r = y - A @ x
                r[~keep] = 0.0
                s = 1.4826 * np.median(np.abs(r[keep])) + 1e-12
                j = int(np.argmax(np.abs(r)))
                if np.abs(r[j]) < 5.0 * s or keep.sum() <= 4:
                    break
                keep[j] = False
                x = np.linalg.lstsq(A[keep], y[keep], rcond=None)[0]
            kept_mask[:, i] = keep
            n_excluded[i] = (~keep).sum()
        d_hat[:, i] = x

    err = K_DISP * (u_t @ d_hat - los_true)
    ghost_kept = (kept_mask[ghost_idx].mean() if ghost_idx else float("nan"))
    good_kept = kept_mask[[k for k in range(N) if k not in ghost_idx]].mean()
    return {
        "err": err,
        "cardiac_rad": band_rms(err, FS, CARDIAC_BAND),
        "n_excluded": n_excluded.mean(),
        "ghost_retention": ghost_kept,
        "good_retention": good_kept,
    }


def amplitude_dispersion_of_ghost(n_draws: int = 400, seed: int = 9) -> dict:
    """Does the PS-InSAR D_A gate catch a ghost? No -- and here is why.

    A sidelobe/ring ghost is a deterministic linear function of its parent's
    complex echo, so its amplitude is as stable as the parent's: D_A is
    essentially identical. Only an INCOHERENT clutter cell has high D_A.
    """
    rng = np.random.default_rng(seed)
    n = 200
    parent = 10.0 * np.ones(n) + 0.3 * rng.standard_normal(n)
    ghost = 0.05 * parent                       # -26 dB coherent sidelobe
    ghost = ghost + 0.002 * rng.standard_normal(n)
    clutter = np.abs(0.5 * (rng.standard_normal(n) + 1j * rng.standard_normal(n)))
    da = lambda a: float(a.std() / a.mean())
    return {"D_A parent": da(parent), "D_A ghost": da(ghost),
            "D_A incoherent clutter": da(clutter), "gate": 0.25}


def _self_test() -> str:
    # with no ghosts and no angle error, LS is exact
    r = run_dwell(n_ghost=0, sigma_az_deg=0.0, sigma_el_deg=0.0,
                  sigma_phi=0.0, seed=1)
    assert r["cardiac_rad"] < 1e-9, r["cardiac_rad"]
    # a ghost with zero angular offset from its parent is not a ghost
    r0 = run_dwell(n_ghost=1, ghost_offset_deg=0.0, sigma_az_deg=0.0,
                   sigma_el_deg=0.0, sigma_phi=0.0, seed=1)
    assert r0["cardiac_rad"] < 1e-9, r0["cardiac_rad"]
    # RAIM keeps everything when there is nothing to exclude
    rr = run_dwell(n_ghost=0, robust="raim", seed=2)
    assert rr["good_retention"] > 0.99, rr["good_retention"]
    # D_A cannot separate a coherent ghost from its parent
    d = amplitude_dispersion_of_ghost()
    assert d["D_A ghost"] < d["gate"] and d["D_A parent"] < d["gate"]
    assert d["D_A incoherent clutter"] > d["gate"]
    return "exp7 self-tests pass"


def main() -> None:
    print(_self_test(), "\n")
    print(f"cardiac budget {CARDIAC_RESIDUAL_BUDGET_RAD:.4f} rad\n")

    d = amplitude_dispersion_of_ghost()
    print("--- does the D_A stability gate catch a ghost? ---")
    for k, v in d.items():
        print(f"  {k:26s} {v:.4f}")
    print("  -> a coherent ghost passes the D_A gate as easily as its parent;"
          "\n     only INCOHERENT clutter is rejected. D_A is not the "
          "discriminator.\n")

    print("--- cost of ghosts, and whether robust estimation recovers it ---")
    print(f"{'n_ghost':>7s} {'estimator':>10s} {'cardiac_rad':>12s} "
          f"{'vs budget':>10s} {'ghost kept':>11s} {'good kept':>10s}")
    seeds = range(6)
    for ng in (0, 1, 2, 3, 4):
        for rob in ("none", "irls", "raim", "traim", "consensus"):
            vals, gk, gd = [], [], []
            for s in seeds:
                r = run_dwell(n_ghost=ng, robust=rob, seed=s)
                vals.append(r["cardiac_rad"])
                gk.append(r["ghost_retention"])
                gd.append(r["good_retention"])
            m = float(np.mean(vals))
            flag = "PASS" if m < CARDIAC_RESIDUAL_BUDGET_RAD else "FAIL"
            gkm = float(np.nanmean(gk)) if ng else float("nan")
            print(f"{ng:7d} {rob:>10s} {m:12.4f} {flag:>10s} "
                  f"{gkm:11.3f} {float(np.mean(gd)):10.3f}")

    print("\n--- consensus breakdown point (N=12 anchors) ---")
    print(f"{'n_ghost':>7s} {'frac':>6s} {'cardiac_rad':>12s} {'verdict':>8s} "
          f"{'ghost kept':>11s} {'good kept':>10s}")
    for ng in range(0, 8):
        rs = [run_dwell(n_ghost=ng, n_anchors=12, robust="consensus", seed=s)
              for s in seeds]
        m = float(np.mean([r["cardiac_rad"] for r in rs]))
        gk = float(np.nanmean([r["ghost_retention"] for r in rs]))
        gd = float(np.mean([r["good_retention"] for r in rs]))
        flag = "PASS" if m < CARDIAC_RESIDUAL_BUDGET_RAD else "FAIL"
        print(f"{ng:7d} {ng/12:6.2f} {m:12.4f} {flag:>8s} {gk:11.3f} "
              f"{gd:10.3f}")

    print("\n--- how far away does a ghost have to be to matter? ---")
    print(f"{'offset_deg':>10s} {'plain LS':>10s} {'per-frame':>10s} "
          f"{'temporal':>10s} {'ghost kept':>11s}")
    for off in (0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 45.0):
        a = np.mean([run_dwell(1, off, robust="none", seed=s)["cardiac_rad"]
                     for s in seeds])
        b = np.mean([run_dwell(1, off, robust="raim", seed=s)["cardiac_rad"]
                     for s in seeds])
        cs = [run_dwell(1, off, robust="traim", seed=s) for s in seeds]
        c = np.mean([r["cardiac_rad"] for r in cs])
        gk = np.mean([r["ghost_retention"] for r in cs])
        print(f"{off:10.1f} {a:10.4f} {b:10.4f} {c:10.4f} {gk:11.3f}")


if __name__ == "__main__":
    main()
