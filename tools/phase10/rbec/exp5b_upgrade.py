"""exp5b: the P1 upgrade pass over the exp5 real-data harness.

Implements every item of thesis_plan.md §4 P1 ("exp5 upgrade run") as pure
additions to the exp5 machinery, per the re-scoping in
radar_rbec_validation.md §F.4 and radar_rbec_validation_exp67.md §B.4/§C.4/§D:

  A. **Per-dwell z-aliasing gain alpha** (exp67 B.4): the exp6 closed form
     computed from the dwell's anchor geometry. Wall/hallway anchors carry
     no measured elevation, and with all-zero elevations the nominal value
     degenerates to a constant — so on azimuth-only data the report carries
     ONLY the seeded Monte-Carlo p95 |alpha| bound over an elevation bound
     (default +-5 deg), and the 0.02 gate is applied to that bound
     (conservative: certification below the gate then requires per-anchor
     elevation knowledge or a tighter bound — the committed fixture's own
     bundle shows the bound FAILING the gate at +-5 deg with 9 anchors,
     while its true drawn geometry passes). A nominal alpha is reported
     only when elevations are supplied. Full uncertainty propagation is P2.
  B. **Co-range structural pre-filter** (exp67 C.4): candidate anchors
     sharing a range bin (within ``corange_tol_bins``) are clustered before
     any solve; only the strongest of a cluster is admitted, the rest are
     flagged as suspected sidelobe/ring ghosts and reported. D_A is computed
     per candidate and kept for its actual job — rejecting incoherent
     clutter — never cited as ghost protection.
  C. **Subset-consensus solve** (exp7, adapted 2-D): RANSAC-style minimal
     3-anchor subsets over the dwell's frame pairs, agreement tolerance
     from the a-priori sigma_phi + T3 variance model (frame-pair phase
     sigma is sqrt(2)*sigma_phi; sigma_theta defaults to the 2.5-deg-grid
     DoA quantization, ~0.72 deg). Two deliberate domain adaptations,
     documented at consensus_solve: full-RMS agreement (differencing turns
     exp7's ramp-shaped ghost signature into a DC offset that mean-removed
     std cannot see) and a wrap-aware tolerance clamp with a per-dwell
     ``consensus_regime_valid`` flag (integer fixing caps pair residuals
     near lambda/4, so beyond ~0.1 m/s the model tolerance would otherwise
     exceed the statistic's ceiling and admit everything — the walking-pace
     regime of ec_hallways_run4 itself, where per-pair discrimination is
     honestly degraded and stride/chirp-rate processing (C.1) is the real
     fix). Consensus set size, the excluded anchors' azimuths, and the
     regime flag are reported per dwell.
  D. **IMU-seeded integers** (toward F.4's "IMU-seeded (not GT-seeded)"):
     a ColoRadar ``imu/`` reader plus a GT-initialized hybrid seed —
     attitude assumed world-aligned and the dwell-start velocity from a GT
     central difference, acceleration double-integrated in between (a full
     INS mechanisation is out of scope, as in endtoend.py's IMU note; the
     v0 finite-difference error is part of what the metric measures).
     Scored as the fraction of anchor-by-pair integer fixes agreeing with
     the GT-seeded fix. When the IMU log is absent, malformed, or does not
     cover the radar window, the run falls back to GT seeding and SAYS SO
     (``int_seed_mode`` stays "gt").
  E. **Full-sequence run**: the sequence is streamed in dwells (default
     30 s); anchors are re-picked per dwell (anchor migration), and the
     per-dwell report carries alpha, the consensus set size, flagged
     ghosts, and the held-out static-cell residual for the plain and
     consensus solves side by side.

Prediction on record (thesis_plan.md P1): the 555 um held-out residual
improves once co-range ghosts are excluded, or the identical-range clusters
were not ghosts — informative either way. Because the pre-filter runs
before BOTH solves, plain-vs-consensus alone cannot isolate it; the
``--baseline`` arm (no pre-filter, no D_A gate — the exp5-equivalent
picker) exists precisely so the real-data rerun can compare
baseline-vs-upgraded holdout residuals and attribute the change.

Without a dataset the module validates end-to-end on a seeded synthetic
fixture in the exact ColoRadar layout (multi-anchor scene, two injected
coherent ghosts, 3-D platform motion, synthetic IMU) and ``--check``
verifies the committed fixture bundle ``docs/phase10/results/exp5b.json``
in the exp67_report sense (rtol 1e-9). With data:

    python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
        --sequence ec_hallways_run4 [--frames 0:2192] [--seed-mode imu]

writes ``exp5b_<sequence>_<seedmode>[...].json`` + figure next to the
fixture bundle; add ``--baseline`` for the exp5-equivalent ablation arm.

IMU file layout note: the fixture and reader use ``imu/imu_data.txt`` rows
``ax ay az gx gy gz`` (m/s^2, rad/s) with ``imu/timestamps.txt`` — the
dev-kit convention as mirrored in azinke/coloradar; re-verify against the
archive when a real sequence lands (the reader degrades to GT seeding with a
warning if the directory is absent or malformed).
"""

from __future__ import annotations

import json
import os
import sys

import numpy as np

from .coloradar_bridge import (CascadeCalib, CascadeSequence, quat_continuous,
                               quat_mats, range_fft, steer_beam,
                               write_fixture)
from .core import los_from_azel

RESULTS_DIR = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "..", "docs", "phase10", "results"))

ALPHA_GATE = 0.02              # exp67 B.4: keep 2-D solve below this
DA_GATE = 0.25                 # PS-InSAR amplitude-dispersion admission
EL_BOUND_DEG = 5.0             # hallway anchor elevation bound for alpha p95
CONSENSUS_DRAWS = 200          # exp7's draw count
CONSENSUS_TOL_MULT = 4.0       # exp7's consensus tolerance multiplier
SIGMA_THETA_DEG = 2.5 / np.sqrt(12.0)   # 2.5-deg DoA grid quantization
COND_MAX = 1e4                 # subset geometry condition guard (2-D)
WRAP_TOL_FRAC = 0.5            # consensus tol clamp, fraction of lambda/4


# --------------------------------------------------------------------------
# A. per-dwell alpha
# --------------------------------------------------------------------------

def alpha_gain(U: np.ndarray, u_t: np.ndarray) -> float:
    """exp6 Eq. (1): LOS prediction error per metre of unmodelled dz for a
    2-D (x, y) solve; U (N,3) anchor LOS unit vectors, u_t (3,) target."""
    g = np.linalg.pinv(U[:, :2]) @ U[:, 2]
    return float(u_t[:2] @ g - u_t[2])


def alpha_report(anchor_az: np.ndarray, target_az: float,
                 anchor_el: np.ndarray | None = None, target_el: float = 0.0,
                 el_bound_deg: float = EL_BOUND_DEG, n_draws: int = 256,
                 seed: int = 0) -> dict:
    """Per-dwell alpha: nominal (given or zero elevations) plus the seeded
    p95/max of |alpha| over elevations drawn uniformly in +-el_bound_deg —
    the honest bound when wall-anchor elevation is not measured."""
    az = np.asarray(anchor_az, dtype=float)
    u_t = los_from_azel(target_az, target_el)
    if anchor_el is not None:
        el0 = np.asarray(anchor_el, dtype=float)
        U0 = np.array([los_from_azel(a, e) for a, e in zip(az, el0)])
        nominal = alpha_gain(U0, u_t)
    else:
        # with all-zero elevations alpha degenerates to -sin(target_el)
        # regardless of the azimuths — a dead constant, not a geometry
        # readout — so no nominal is reported and only the bound stands
        nominal = None
    rng = np.random.default_rng(seed)
    draws = np.empty(n_draws)
    b = np.deg2rad(el_bound_deg)
    for i in range(n_draws):
        el = rng.uniform(-b, b, az.size)
        # anchor elevations only; the target stays at its nominal elevation
        # (target-elevation uncertainty is P2's question, not this report's)
        draws[i] = alpha_gain(
            np.array([los_from_azel(a, e) for a, e in zip(az, el)]), u_t)
    absd = np.abs(draws)
    p95 = float(np.percentile(absd, 95))
    return {"alpha_nominal": nominal,
            "el_known": bool(anchor_el is not None),
            "alpha_abs_p95": p95,
            "alpha_abs_max": float(absd.max()),
            "el_bound_deg": float(el_bound_deg),
            "within_gate": bool(p95 < ALPHA_GATE)}


# --------------------------------------------------------------------------
# B. anchor picking with the structural pre-filter
# --------------------------------------------------------------------------

def energy_map_fast(seq: CascadeSequence, frames, az_grid: np.ndarray,
                    max_bin: int) -> np.ndarray:
    """Vectorized time-integrated |beam|^2 over (az, range-bin): one matmul
    per frame over the elevation-0 azimuth row, matching steer_beam's
    weights (which are applied un-conjugated)."""
    c = seq.calib
    sel = (c.virt_el == c.virt_el.min())
    W = np.exp(-1j * np.pi * c.d_scale * c.virt_az[..., None]
               * np.sin(az_grid)[None, None, :]) * sel[..., None]
    W = W / np.abs(W).sum(axis=(0, 1), keepdims=True)      # (tx, rx, n_az)
    Wf = W.reshape(-1, az_grid.size)
    acc = np.zeros((az_grid.size, max_bin))
    for fi in frames:
        rf = range_fft(seq.frame(fi), calib=seq.calib)
        v = rf[..., :max_bin].mean(axis=2)                  # chirp mean
        acc += np.abs(Wf.T @ v.reshape(-1, max_bin)) ** 2
    return acc


def corange_prefilter(cands: list, tol_bins: int = 1) -> tuple[list, list]:
    """exp67 C.4 structural pre-filter, before any solve: cluster candidates
    whose range bins agree within ``tol_bins``; admit only the strongest of
    each cluster, flag the rest as suspected sidelobe/ring ghosts.
    ``cands`` rows: (az, rbin, power), strongest-first order preserved."""
    admitted, flagged = [], []
    for az, rb, p in cands:
        clash = any(abs(rb - rb2) <= tol_bins for _, rb2, _ in admitted)
        (flagged if clash else admitted).append((az, rb, p))
    return admitted, flagged


def amplitude_dispersion(seq: CascadeSequence, cells: list, frames) -> list:
    """Per-cell D_A = std/mean of |beam| over ``frames`` (PS-InSAR); kept
    for incoherent-clutter rejection only — structurally ghost-blind."""
    fl = list(frames)
    amps = np.zeros((len(cells), len(fl)))
    for j, fi in enumerate(fl):
        rf = range_fft(seq.frame(fi), calib=seq.calib)
        for k, (az, rb) in enumerate(cells):
            amps[k, j] = np.abs(steer_beam(rf, seq.calib, az, rb))
    m = amps.mean(axis=1)
    m[m == 0] = 1.0
    return [float(x) for x in amps.std(axis=1) / m]


def pick_anchors_v2(seq: CascadeSequence, n_anchors: int, n_holdout: int,
                    f_start: int, warmup_frames: int = 10,
                    max_range_m: float = 15.0, min_sep_deg: float = 4.0,
                    corange_tol_bins: int | None = 1,
                    da_gate: float | None = DA_GATE,
                    f_end: int | None = None,
                    guard_bins: int = 8) -> dict:
    """exp5's energy-ranked picker + the co-range pre-filter + D_A.
    Returns admitted anchors, holdout cells, flagged co-range ghosts, and
    the diagnostics the per-dwell report carries."""
    c = seq.calib
    az_grid = np.deg2rad(np.linspace(-55, 55, 45))
    raxis = c.range_axis()
    max_bin = int(np.searchsorted(raxis, max_range_m))
    hi = seq.n_frames() if f_end is None else min(f_end, seq.n_frames())
    wf = range(f_start, min(f_start + warmup_frames, hi))
    emap = energy_map_fast(seq, wf, az_grid, max_bin)
    # near-field/leakage guard; the measured cascade coupling residue on
    # ASPEN still frames is >= +20 dB over the mid-range background at
    # bin 4 and only falls below it from bin 7-8 (0.42-0.47 m). Default 8
    # per the D.6 verdict (guards 8 and 17 bit-identical; the bin-4 cell
    # acted as a zero-motion regularizer that every gate favors); pass 4
    # to reproduce exp5-era behavior
    emap[:, :guard_bins] = 0.0
    order = np.argsort(emap.ravel())[::-1]
    cands = []
    for idx in order:
        ai, rb = np.unravel_index(idx, emap.shape)
        az = az_grid[ai]
        if any(abs(az - a) < np.deg2rad(min_sep_deg) for a, _, _ in cands):
            continue                          # >=4 deg azimuth separation
        cands.append((float(az), int(rb), float(emap[ai, rb])))
        if len(cands) >= 3 * (n_anchors + n_holdout):
            break
    if corange_tol_bins is None:
        admitted, flagged = list(cands), []
    else:
        admitted, flagged = corange_prefilter(cands, corange_tol_bins)
    pool = admitted[:2 * (n_anchors + n_holdout)]
    cells = [(az, rb) for az, rb, _ in pool]
    da = amplitude_dispersion(seq, cells, wf)
    if da_gate is None:
        coherent = list(zip(cells, da))
    else:
        coherent = [(cell, d) for cell, d in zip(cells, da) if d < da_gate]
    n_da_dropped = len(pool) - len(coherent)
    cells = [cell for cell, _ in coherent][:n_anchors + n_holdout]
    da = [d for _, d in coherent][:n_anchors + n_holdout]
    anchors = cells[:n_anchors]
    holdout = cells[n_anchors:n_anchors + n_holdout]
    if len(anchors) < 4 or not holdout:
        raise ValueError(
            f"anchor pool too thin after gates: {len(anchors)} anchors, "
            f"{len(holdout)} holdout (candidates {len(cands)}, co-range "
            f"flagged {len(flagged)}, D_A dropped {n_da_dropped})")
    return {"anchors": anchors, "holdout": holdout,
            "flagged_corange": [(az, rb) for az, rb, _ in flagged],
            "d_a": da[:len(anchors)], "raxis": raxis, "emap": emap,
            "az_grid": az_grid}


# --------------------------------------------------------------------------
# C. the solves
# --------------------------------------------------------------------------

def track_phases(seq: CascadeSequence, cells: list, f0: int,
                 f1: int) -> np.ndarray:
    ph = np.empty((len(cells), f1 - f0))
    for j, fi in enumerate(range(f0, f1)):
        rf = range_fft(seq.frame(fi), calib=seq.calib)
        for k, (az, rb) in enumerate(cells):
            ph[k, j] = np.angle(steer_beam(rf, seq.calib, az, rb))
    return ph


def integer_fixed_increments(ph: np.ndarray, U2: np.ndarray,
                             pred_m: np.ndarray, k_disp: float) -> np.ndarray:
    """Per-pair LOS displacements (N, P) in metres after integer fixing
    seeded by the predicted increments ``pred_m`` (P, 2)."""
    dphi = np.diff(ph, axis=1)
    pred = k_disp * (U2 @ pred_m.T)          # (N, P)
    n = np.round((pred - dphi) / (2 * np.pi))
    return (dphi + 2 * np.pi * n) / k_disp


def solve_plain(U2: np.ndarray, Y: np.ndarray) -> np.ndarray:
    return np.linalg.lstsq(U2, Y, rcond=None)[0].T          # (P, 2)


def consensus_solve(U2: np.ndarray, Y: np.ndarray, sigma_phi: float,
                    k_disp: float, sigma_theta_deg: float = SIGMA_THETA_DEG,
                    seed: int = 0, n_draws: int = CONSENSUS_DRAWS,
                    subset: int = 3, n_score_cols: int = 120) -> dict:
    """exp7's subset-consensus estimator, adapted to the 2-D frame-pair
    solve. Y is (N, P) integer-fixed LOS increments in metres.

    Two deliberate departures from a line-by-line port, both forced by the
    frame-differenced domain:

    * **Full RMS, not mean-removed std.** exp7 removed each anchor's mean
      because absolute phases carry an arbitrary static bias b_k; the pair
      domain has already differenced b_k away, and a mis-attributed anchor
      under sustained platform velocity produces a *constant* per-pair
      residual — the one component std cannot see. RMS (mean included)
      restores exp7's sensitivity: its linear-in-time ghost signature is
      exactly this DC term after differencing.
    * **Wrap-aware tolerance.** Y passed through integer fixing, so every
      residual is capped near lambda/4 of the seeded prediction; the model
      tolerance (which grows with d_rms through T3) would cross that
      ceiling at ~0.1 m/s platform speed and admit everything. The
      effective tolerance is clamped at WRAP_TOL_FRAC * lambda/4 and the
      returned ``regime_valid`` flag records whether the model tolerance
      stayed below the clamp — when False, per-pair residual discrimination
      is saturated (sigma_theta * d_pair approaching lambda/4) and the
      dwell's consensus verdicts must be read as degraded (the C.1
      chirp-rate/stride reduction, not a bigger tolerance, is the honest
      fix in that regime)."""
    N, P = Y.shape
    x_all = np.linalg.lstsq(U2, Y, rcond=None)[0]           # (2, P)
    d_rms = float(np.sqrt(np.mean(np.sum(x_all ** 2, axis=0))))
    sig_pair = np.sqrt(2.0) * sigma_phi
    t3 = k_disp * np.deg2rad(sigma_theta_deg) * d_rms
    tol_model = CONSENSUS_TOL_MULT * np.sqrt(sig_pair ** 2 + t3 ** 2) \
        / k_disp
    tol_clamp = WRAP_TOL_FRAC * np.pi / k_disp              # frac of lam/4
    tol_m = min(tol_model, tol_clamp)
    regime_valid = tol_model < tol_clamp
    rs = np.random.default_rng(seed + 77)
    cols = np.linspace(0, P - 1, min(P, n_score_cols)).astype(int)
    best_keep, best_score = None, -1
    for _ in range(n_draws):
        idx = rs.choice(N, size=subset, replace=False)
        Us = U2[idx]
        sv = np.linalg.svd(Us, compute_uv=False)
        if sv[-1] < 1e-9 or sv[0] / sv[-1] > COND_MAX:
            continue                          # rank guard (belt-and-braces)
        x = np.linalg.lstsq(Us, Y[np.ix_(idx, cols)], rcond=None)[0]
        r = Y[:, cols] - U2 @ x
        agree = np.sqrt((r ** 2).mean(axis=1)) < tol_m
        if agree.sum() > best_score:
            best_score, best_keep = int(agree.sum()), agree.copy()
    keep = best_keep if best_keep is not None and best_keep.sum() >= subset \
        else np.ones(N, dtype=bool)
    sv = np.linalg.svd(U2[keep], compute_uv=False)
    if keep.sum() >= subset and sv[-1] > 1e-9 \
            and sv[0] / sv[-1] < COND_MAX:
        x = np.linalg.lstsq(U2[keep], Y[keep], rcond=None)[0]
    else:
        keep = np.ones(N, dtype=bool)
        x = x_all
    return {"d_hat": x.T, "keep": keep, "set_size": int(keep.sum()),
            "tol_m": float(tol_m), "tol_model_m": float(tol_model),
            "regime_valid": bool(regime_valid)}


def holdout_residual_um(ph_h: np.ndarray, Uh2: np.ndarray,
                        d_hat: np.ndarray, k_disp: float) -> float:
    res = np.diff(ph_h, axis=1) - k_disp * (Uh2 @ d_hat.T)
    res = (res + np.pi) % (2 * np.pi) - np.pi
    return float(res.std() / k_disp * 1e6)


# --------------------------------------------------------------------------
# D. seeding: GT and IMU dead-reckoning
# --------------------------------------------------------------------------

def gt_positions(seq: CascadeSequence) -> np.ndarray:
    return np.stack([np.interp(seq.times, seq.gt_times, seq.gt_poses[:, i])
                     for i in range(3)], axis=1)


def gt_attitude(seq: CascadeSequence, times: np.ndarray) -> np.ndarray:
    """GT quaternions (x y z w), hemisphere-continuous, component-lerped to
    ``times`` and renormalized (adequate at vicon rates)."""
    q = quat_continuous(seq.gt_poses[:, 3:7])
    qi = np.stack([np.interp(times, seq.gt_times, q[:, i])
                   for i in range(4)], axis=1)
    return qi / np.linalg.norm(qi, axis=1, keepdims=True)


def cascade_track(seq: CascadeSequence) -> tuple[np.ndarray, np.ndarray]:
    """The point the radar phase actually measures: cascade antenna world
    positions (base GT + attitude-rotated base_to_cascade lever arm) and
    R world<-cascade per frame. D.7: at the rig's 15.3 cm lever arm even
    0.1 deg of attitude wobble moves the cascade ~270 um relative to the
    base — first-order against lambda/4."""
    tf = seq.calib.transforms
    t_bc, R_bc = tf["base_to_cascade"]
    R_wb = quat_mats(gt_attitude(seq, seq.times))
    p_c = gt_positions(seq) + R_wb @ t_bc
    return p_c, R_wb @ R_bc


def cascade_frame_increments(p_c: np.ndarray,
                             R_wc: np.ndarray) -> np.ndarray:
    """Per-frame-pair displacement increments of the cascade point,
    expressed in the CASCADE frame at each pair's start — the frame the
    steering vectors live in. (The naive modes project world-frame
    increments onto cascade-frame steering vectors; base_to_cascade is a
    ~90 deg yaw, so that mismatch is first-order once integers are live.)"""
    dinc_w = np.diff(p_c, axis=0)
    return np.einsum("nji,nj->ni", R_wc[:-1], dinc_w)


def imu_bias_body(imu_t: np.ndarray, accel: np.ndarray,
                  seq: CascadeSequence, window_s: float = 5.0,
                  gravity: float = 9.80665) -> np.ndarray:
    """ZUPT-style body-frame accel bias, self-contained: over the quietest
    ``window_s`` of the log (minimum rolling std of the accel magnitude),
    b = mean(a_body) - R_bw [0, 0, g]. Attitude is only needed to place
    gravity; in flight a pre-flight still calibration or EKF plays this
    role. Without it the measured 0.2-0.65 m/s^2 residual double-integrates
    to 100s of mm per pair over a dwell (D.7 probe)."""
    n = max(int(round(window_s / max(np.median(np.diff(imu_t)), 1e-4))), 10)
    mag = np.linalg.norm(accel, axis=1)
    c1 = np.cumsum(np.insert(mag, 0, 0.0))
    c2 = np.cumsum(np.insert(mag * mag, 0, 0.0))
    m = (c1[n:] - c1[:-n]) / n
    var = np.maximum((c2[n:] - c2[:-n]) / n - m * m, 0.0)
    s = int(np.argmin(var))
    _, R_bi = seq.calib.transforms["base_to_imu"]
    R_wi = quat_mats(gt_attitude(seq, imu_t[s:s + n])) @ R_bi
    g_imu = np.einsum("nji,j->ni", R_wi, np.array([0.0, 0.0, gravity]))
    return accel[s:s + n].mean(axis=0) - g_imu.mean(axis=0)


def imu_increments_rotated(imu_t: np.ndarray, accel: np.ndarray,
                           seq: CascadeSequence, p_ref: np.ndarray,
                           R_wc: np.ndarray, dwell_start: int,
                           gravity: float = 9.80665,
                           zupt: bool = True) -> np.ndarray | None:
    """D.7 dead-reckoning: body accel rotated to world via GT attitude
    (interpolated to IMU sample times) composed with base_to_imu, gravity
    subtracted in the world frame, double-integrated from a GT central-
    difference v0 of the CASCADE point; the base->cascade lever-arm delta
    is added from GT attitude, and the result is expressed in the cascade
    frame like the reference increments. Attitude comes from GT, not gyro
    integration — the claim tested is 'known attitude + measured accel
    suffice to seed integers', the EKF-attitude analogue for a drone.
    Returns None when the IMU log does not cover the radar window."""
    tf = seq.calib.transforms
    _, R_bi = tf["base_to_imu"]
    radar_times = seq.times
    t0 = radar_times[dwell_start]
    k = dwell_start
    if 0 < k < radar_times.size - 1:
        dt0 = radar_times[k + 1] - radar_times[k - 1]
        v0 = (p_ref[k + 1] - p_ref[k - 1]) / max(dt0, 1e-9)
    elif k + 1 < radar_times.size:
        dt0 = radar_times[k + 1] - radar_times[k]
        v0 = (p_ref[k + 1] - p_ref[k]) / max(dt0, 1e-9)
    else:
        v0 = np.zeros(3)
    if zupt:
        accel = accel - imu_bias_body(imu_t, accel, seq, gravity=gravity)
    R_wb_imu = quat_mats(gt_attitude(seq, imu_t))
    a = np.einsum("nij,nj->ni", R_wb_imu, accel @ R_bi.T)
    a[:, 2] -= gravity
    sel = imu_t >= t0
    ts, asel = imu_t[sel], a[sel]
    if ts.size < 2 or ts[-1] < radar_times[-1] - 0.5:
        return None
    ts = np.concatenate([[t0], ts])
    aseg = np.vstack([asel[:1], asel])
    dt = np.diff(ts)
    v = np.concatenate([[v0], v0 + np.cumsum(aseg[:-1] * dt[:, None],
                                             axis=0)])
    p = np.concatenate([[np.zeros(3)],
                        np.cumsum(v[:-1] * dt[:, None], axis=0)])
    pr = np.stack([np.interp(radar_times, ts, p[:, i]) for i in range(3)],
                  axis=1)
    # dead reckoning tracks the IMU point (base: base_to_imu translation is
    # zero); the cascade lever-arm delta comes from GT attitude
    lever = p_ref - gt_positions(seq)
    dinc_w = np.diff(pr, axis=0) + np.diff(lever, axis=0)
    return np.einsum("nji,nj->ni", R_wc[:-1], dinc_w)


def imu_pair_bridge(imu_t: np.ndarray, accel: np.ndarray,
                    seq: CascadeSequence, p_ref: np.ndarray,
                    R_wc: np.ndarray, gravity: float = 9.80665,
                    zupt: bool = True) -> np.ndarray | None:
    """D.7 tracker-anchored seeding: the F-series design never dead-reckons
    a dwell — the IMU bridges one inter-measurement gap with velocity
    re-anchored by the radar's own track. Discrete form, exact for
    piecewise-integrable accel: with per-pair integrals I1_j = int a dt
    and I2_j = int int a dt^2 (velocity zeroed at each pair start),

        seed_i = dp_{i-1} + I1_{i-1} * dt_i + I2_i - I2_{i-1}

    where dp_{i-1} is the PREVIOUS pair's increment — in operation the
    tracker's last solved increment (error ~ the solve error), here the GT
    increment stands in for it and is labeled as such. The first pair uses
    the GT central-difference v0. Open-loop error therefore never
    accumulates beyond one 0.2 s frame gap — the ColoRadar analogue of the
    47 ms inter-burst budget, 4x the gap the design assumed."""
    tf = seq.calib.transforms
    _, R_bi = tf["base_to_imu"]
    radar_times = seq.times
    if zupt:
        accel = accel - imu_bias_body(imu_t, accel, seq, gravity=gravity)
    if imu_t[0] > radar_times[0] + 0.5 or imu_t[-1] < radar_times[-1] - 0.5:
        return None
    R_wb_imu = quat_mats(gt_attitude(seq, imu_t))
    a = np.einsum("nij,nj->ni", R_wb_imu, accel @ R_bi.T)
    a[:, 2] -= gravity
    n = radar_times.size
    I1 = np.zeros((n - 1, 3))
    I2 = np.zeros((n - 1, 3))
    for i in range(n - 1):
        t0, t1 = radar_times[i], radar_times[i + 1]
        s = (imu_t >= t0) & (imu_t <= t1)
        if s.sum() < 2:
            continue
        ts = np.concatenate([[t0], imu_t[s], [t1]])
        aa = np.vstack([a[s][:1], a[s], a[s][-1:]])
        dt = np.diff(ts)
        I1[i] = (aa[:-1] * dt[:, None]).sum(axis=0)
        v = np.concatenate([[np.zeros(3)],
                            np.cumsum(aa[:-1] * dt[:, None], axis=0)])
        I2[i] = (v[:-1] * dt[:, None]).sum(axis=0)
    dp = np.diff(p_ref, axis=0)                    # true previous increments
    seed = np.empty_like(dp)
    ddt = np.diff(radar_times)
    if radar_times.size > 2:
        v0 = (p_ref[2] - p_ref[0]) / max(radar_times[2] - radar_times[0],
                                         1e-9)
    else:
        v0 = np.zeros(3)
    seed[0] = v0 * ddt[0] + I2[0]
    seed[1:] = dp[:-1] + I1[:-1] * ddt[1:, None] + I2[1:] - I2[:-1]
    # dp tracks the cascade point, so the carried-forward term already
    # holds the lever arm; only its intra-pair change (second-order in
    # attitude rate) is missed by the base-frame accel integrals
    return np.einsum("nji,nj->ni", R_wc[:-1], seed)


def read_imu(seq_dir: str):
    """ColoRadar imu/ reader: rows 'ax ay az gx gy gz' + timestamps.txt.
    Returns (times, accel) or None when absent/malformed."""
    d = os.path.join(seq_dir, "imu")
    try:
        t = np.loadtxt(os.path.join(d, "timestamps.txt"))
        data = np.loadtxt(os.path.join(d, "imu_data.txt"))
    except (OSError, ValueError) as e:
        print(f"  imu/ unreadable ({e.__class__.__name__}) -- GT seeding")
        return None
    if data.ndim != 2 or data.shape[1] < 6 or t.size != data.shape[0]:
        print("  imu/ layout unexpected "
              f"(shape {getattr(data, 'shape', None)}) -- GT seeding")
        return None
    return t, data[:, :3]


def imu_increments(imu_t: np.ndarray, accel: np.ndarray,
                   radar_times: np.ndarray, gpos: np.ndarray,
                   dwell_start: int,
                   gravity: float = 9.80665) -> np.ndarray | None:
    """Dead-reckoned per-frame-pair displacement increments over one dwell.

    Honesty note (mirrors endtoend.py's IMU treatment): attitude is assumed
    world-aligned (the fixture convention; real runs must rotate accel by
    the GT attitude — extrinsics TODO, as in exp5's gt_increments), gravity
    is subtracted as a constant +z, and the initial velocity comes from the
    GT central difference at the dwell start; from there acceleration is
    double-integrated. The measured integer-agreement statistic therefore
    bundles v0 finite-difference error (the dominant term at radar-rate
    GT), integrator error, bias and noise — the seeding error budget of
    the method as actually run, not an isolated IMU spec. Returns None
    when the IMU log does not cover the radar window (the caller must fall
    back to GT seeding *and say so*)."""
    t0 = radar_times[dwell_start]
    k = dwell_start
    if 0 < k < radar_times.size - 1:
        # central difference: one-sided differences carry an O(dt/2 * a)
        # velocity error (~mm/s at hover accelerations) that alone busts
        # the lambda/4 integer margin over a dwell
        dt0 = radar_times[k + 1] - radar_times[k - 1]
        v0 = (gpos[k + 1] - gpos[k - 1]) / max(dt0, 1e-9)
    elif k + 1 < radar_times.size:
        dt0 = radar_times[k + 1] - radar_times[k]
        v0 = (gpos[k + 1] - gpos[k]) / max(dt0, 1e-9)
    else:
        v0 = np.zeros(3)
    a = accel.copy()
    a[:, 2] -= gravity
    sel = imu_t >= t0
    ts, asel = imu_t[sel], a[sel]
    if ts.size < 2 or ts[-1] < radar_times[-1] - 0.5:
        return None          # no/partial IMU coverage: caller falls back
    # prepend t0 itself so the first frame pair does not lose v0*(ts[0]-t0)
    # to the interp clamp (up to one IMU period of motion — above lambda/4
    # at walking pace)
    ts = np.concatenate([[t0], ts])
    aseg = np.vstack([asel[:1], asel])       # accel per integration segment
    dt = np.diff(ts)
    v = np.concatenate([[v0], v0 + np.cumsum(aseg[:-1] * dt[:, None],
                                             axis=0)])
    p = np.concatenate([[np.zeros(3)],
                        np.cumsum(v[:-1] * dt[:, None], axis=0)])
    pr = np.stack([np.interp(radar_times, ts, p[:, i]) for i in range(3)],
                  axis=1)
    return np.diff(pr, axis=0)


# --------------------------------------------------------------------------
# E. the per-dwell run
# --------------------------------------------------------------------------

def run_dwell_real(seq: CascadeSequence, f0: int, f1: int, n_anchors: int,
                   n_holdout: int, seed: int, target_az_deg: float = 0.0,
                   sigma_phi: float = 0.0115,
                   sigma_theta_deg: float = SIGMA_THETA_DEG,
                   seed_mode: str = "gt", baseline: bool = False,
                   guard_bins: int = 8) -> dict:
    """One dwell of the upgraded pipeline; returns a JSON-ready dict.

    ``baseline=True`` disables the co-range pre-filter and the D_A gate —
    the exp5-equivalent picker — so the pre-filter's own contribution to
    the P1 prediction is measurable as an ablation, not conflated with the
    consensus solve's."""
    c = seq.calib
    k_disp = 4 * np.pi / c.lam
    pick = pick_anchors_v2(
        seq, n_anchors, n_holdout, f0, f_end=f1,
        corange_tol_bins=None if baseline else 1,
        da_gate=None if baseline else DA_GATE,
        guard_bins=guard_bins)
    anchors, holdout = pick["anchors"], pick["holdout"]
    az = np.array([a for a, _ in anchors])
    U2 = np.stack([np.cos(az), np.sin(az)], axis=1)
    Uh2 = np.array([[np.cos(a), np.sin(a)]
                    for a, _ in holdout]).reshape(-1, 2)

    ph_a = track_phases(seq, anchors, f0, f1)
    ph_h = track_phases(seq, holdout, f0, f1)

    gpos = gt_positions(seq)
    rot_ok = seed_mode in ("gt-rot", "imu-rot", "imu-rot-track")
    if rot_ok and not ({"base_to_cascade", "base_to_imu"}
                       <= set(seq.calib.transforms)):
        print("  calib/transforms missing -- naive-frame fallback "
              "(seed_frame will say so)")
        rot_ok = False
        seed_mode = {"gt-rot": "gt", "imu-rot": "imu",
                     "imu-rot-track": "imu"}[seed_mode]
    if rot_ok:
        # D.7 frame-correct path: reference and seed increments for the
        # cascade point, expressed in the cascade frame
        p_ref, R_wc = cascade_track(seq)
        dinc_all = cascade_frame_increments(p_ref, R_wc)
    else:
        p_ref, R_wc = gpos, None
        dinc_all = np.diff(gpos, axis=0)
    dinc_gt = dinc_all[f0:f1 - 1]
    imu_used = False
    dinc_seed = dinc_gt
    if seed_mode in ("imu", "imu-rot", "imu-rot-track"):
        seq_root = os.path.dirname(os.path.dirname(seq.dir.rstrip("/")))
        imu = read_imu(seq_root)
        if imu is None:
            inc = None
        elif seed_mode == "imu-rot-track":
            inc = imu_pair_bridge(imu[0], imu[1], seq, p_ref, R_wc)
        elif seed_mode == "imu-rot":
            inc = imu_increments_rotated(imu[0], imu[1], seq, p_ref, R_wc,
                                         f0)
        else:
            inc = imu_increments(imu[0], imu[1], seq.times, gpos, f0)
        if inc is None:
            print("  IMU unavailable or not covering the radar window -- "
                  "GT seeding (int_seed_mode will say so)")
        else:
            dinc_seed = inc[f0:f1 - 1]
            imu_used = True

    Y = integer_fixed_increments(ph_a, U2, dinc_seed[:, :2], k_disp)
    Y_gt = integer_fixed_increments(ph_a, U2, dinc_gt[:, :2], k_disp)
    int_agree = float(np.mean(np.round((Y - Y_gt) * k_disp
                                       / (2 * np.pi)) == 0))

    d_plain = solve_plain(U2, Y)
    cons = consensus_solve(U2, Y, sigma_phi, k_disp,
                           sigma_theta_deg=sigma_theta_deg, seed=seed)
    d_cons = cons["d_hat"]

    err_p = d_plain - dinc_gt[:, :2]
    err_c = d_cons - dinc_gt[:, :2]
    alpha = alpha_report(az, np.deg2rad(target_az_deg), seed=seed)
    return {
        "f0": f0, "f1": f1,
        "anchors": [[float(np.rad2deg(a)), int(rb)] for a, rb in anchors],
        "flagged_corange": [[float(np.rad2deg(a)), int(rb)]
                            for a, rb in pick["flagged_corange"]],
        "d_a": pick["d_a"],
        "alpha": alpha,
        "consensus_set_size": cons["set_size"],
        "consensus_excluded": int(len(anchors) - cons["set_size"]),
        "consensus_excluded_az_deg": [float(np.rad2deg(a))
                                      for a, keep_it
                                      in zip(az, cons["keep"])
                                      if not keep_it],
        "consensus_tol_um": cons["tol_m"] * 1e6,
        "consensus_tol_model_um": cons["tol_model_m"] * 1e6,
        "consensus_regime_valid": cons["regime_valid"],
        "int_seed_mode": seed_mode if imu_used
        else ("gt-rot" if rot_ok else "gt"),
        "seed_frame": "cascade" if rot_ok else "world-naive",
        "int_agreement_vs_gt": int_agree,
        "gt_inc_rms_mm": [float(x * 1e3)
                          for x in dinc_gt[:, :2].std(axis=0)],
        "inc_err_rms_mm_plain": [float(x * 1e3) for x in err_p.std(axis=0)],
        "inc_err_rms_mm_consensus": [float(x * 1e3)
                                     for x in err_c.std(axis=0)],
        "holdout_um_plain": holdout_residual_um(ph_h, Uh2, d_plain, k_disp),
        "holdout_um_consensus": holdout_residual_um(ph_h, Uh2, d_cons,
                                                    k_disp),
    }


def run_sequence(root: str, sequence: str, frames: str | None = None,
                 dwell_s: float = 30.0, n_anchors: int = 9,
                 n_holdout: int = 3, seed: int = 0,
                 seed_mode: str = "gt", baseline: bool = False,
                 guard_bins: int = 8) -> dict:
    calib = CascadeCalib(os.path.join(root, "calib"))
    seq = CascadeSequence(os.path.join(root, "kitti", sequence), calib)
    f0, f1 = 0, seq.n_frames()
    if frames:
        f0, f1 = map(int, frames.split(":"))
        f1 = min(f1, seq.n_frames())
    frame_dt = float(np.median(np.diff(seq.times))) if seq.n_frames() > 1 \
        else 0.1
    per_dwell = max(int(round(dwell_s / frame_dt)), 8)
    dwells = []
    s = f0
    while s + max(per_dwell // 2, 8) <= f1:
        e = min(s + per_dwell, f1)
        print(f"dwell {len(dwells)}: frames {s}:{e}")
        try:
            dwells.append(run_dwell_real(seq, s, e, n_anchors, n_holdout,
                                         seed + len(dwells),
                                         seed_mode=seed_mode,
                                         baseline=baseline,
                                         guard_bins=guard_bins))
        except ValueError as err:
            # one thin dwell must not discard the rest of a long run
            print(f"  dwell failed: {err}")
            dwells.append({"f0": s, "f1": e, "error": str(err)})
        s = e
    dropped_tail = int(f1 - s)
    if dropped_tail:
        print(f"tail frames {s}:{f1} shorter than half a dwell -- dropped")
    ok = [d for d in dwells if "error" not in d]
    if not ok:
        raise ValueError(
            f"no processable dwell in frames {f0}:{f1} "
            f"(window {f1 - f0} frames, dwell {per_dwell})")
    hp = [d["holdout_um_plain"] for d in ok]
    hc = [d["holdout_um_consensus"] for d in ok]
    return {
        "sequence": sequence, "gt_source": seq.gt_source,
        "guard_bins": guard_bins,
        "n_frames": int(f1 - f0), "frame_dt_s": frame_dt,
        "n_dwells": len(dwells), "n_dwells_ok": len(ok),
        "frames_dropped_tail": dropped_tail,
        "holdout_um_plain_mean": float(np.mean(hp)),
        "holdout_um_consensus_mean": float(np.mean(hc)),
        "dwells": dwells,
    }


# --------------------------------------------------------------------------
# fixture: multi-anchor scene with coherent ghosts + IMU, ColoRadar layout
# --------------------------------------------------------------------------

FIXTURE_SEQ = "fixture_p1_run0"


def write_fixture_p1(root: str, n_frames: int = 96, fps: float = 10.0,
                     seed: int = 11) -> dict:
    """A P1-shaped synthetic sequence in the exact ColoRadar layout.

    Scene: 10 static anchors on the picker's 2.5-deg azimuth grid, distinct
    range bins, drawn +-5 deg elevations; the platform translates in 3-D
    (shaped low-frequency sway, z included so the alpha leak is real); each
    scatterer's range-phase follows its own true 3-D LOS. Two coherent
    ghosts are injected whose range-phase tracks their PARENT's LOS
    (mis-attribution, exp7's model): G1 at the parent's exact range bin (the
    co-range pre-filter's catch), G2 three bins away and strong enough to
    rank inside the anchor set (the consensus solve's catch). A biased+noisy
    IMU stream consistent with the true motion is written to imu/. Ground
    truth carries the true positions."""
    cal_dir = os.path.join(root, "calib", "cascade")
    if not os.path.isdir(cal_dir):
        write_fixture(root, n_frames=1)      # reuse exp5's calib writer
    calib = CascadeCalib(os.path.join(root, "calib"))
    lam = calib.lam
    rng = np.random.default_rng(seed)

    # all azimuths on the picker's grid (linspace(-55, 55, 45): 2.5-deg
    # steps) so believed DoA equals truth and T3 is noise-only here
    anchors_az = np.deg2rad(np.array(
        [-47.5, -35.0, -22.5, -10.0, 5.0, 17.5, 30.0, 42.5, -42.5, 25.0]))
    anchors_r = np.array([4.1, 6.3, 5.2, 8.7, 3.6, 7.4, 9.8, 6.9,
                          11.2, 12.6])
    anchors_el = np.deg2rad(rng.uniform(-5.0, 5.0, anchors_az.size))
    amps = np.array([500.0, 480.0, 460.0, 440.0, 420.0, 400.0, 390.0,
                     370.0, 310.0, 300.0])

    # ghosts: coherent copies of parent anchor 1 (az -35 deg, r 6.3 m)
    parent = 1
    rres = float(calib.range_axis()[1])
    # G1 amp ranks inside the baseline (no-pre-filter) anchor set so the
    # ablation arm actually carries it; the pre-filter flags it regardless
    ghosts = [
        {"az": np.deg2rad(12.5), "r": anchors_r[parent], "amp": 380.0},
        {"az": np.deg2rad(-15.0), "r": anchors_r[parent] + 3 * rres,
         "amp": 430.0},
    ]

    # motion generated smoothly at IMU rate and SAMPLED at radar frames.
    # Band-limited sum-of-sinusoids sway (0.1-2.2 Hz, 1/f amplitudes): the
    # synthetic accelerometer then carries only low-frequency content, so
    # Euler re-integration errors stay far below lambda/4 and the IMU
    # seeding measurement reflects bias+noise, not integrator artifacts.
    # (shaped_noise's f^-2 amplitude roll-off gives a FLAT acceleration
    # spectrum to Nyquist — unusable as an IMU truth signal.)
    n = n_frames
    fs_imu = 100.0
    step = int(round(fs_imu / fps))
    n_imu = (n - 1) * step + 1
    ti = np.arange(n_imu) / fs_imu

    def sway(rms: float) -> np.ndarray:
        f = rng.uniform(0.12, 2.2, 6)
        amp = 1.0 / f
        ph = rng.uniform(0, 2 * np.pi, 6)
        x = (amp[None, :]
             * np.sin(2 * np.pi * f[None, :] * ti[:, None] + ph)).sum(1)
        return x / x.std() * rms

    di = np.stack([sway(4.0e-3), sway(3.0e-3), sway(2.0e-3)], axis=1)
    d = di[::step]

    U = np.array([los_from_azel(a, e)
                  for a, e in zip(anchors_az, anchors_el)])
    seq_dir = os.path.join(root, "kitti", FIXTURE_SEQ)
    data_dir = os.path.join(seq_dir, "cascade", "adc_samples", "data")
    gt_dir = os.path.join(seq_dir, "groundtruth")
    imu_dir = os.path.join(seq_dir, "imu")
    for p in (data_dir, gt_dir, imu_dir):
        os.makedirs(p, exist_ok=True)

    c = calib
    t_fast = np.arange(c.num_samples) / c.fs
    steer = [np.exp(1j * np.pi * c.d_scale * c.virt_az * np.sin(az))
             for az in list(anchors_az) + [g["az"] for g in ghosts]]
    for fi in range(n):
        cube = np.zeros((c.num_tx, c.num_rx, c.num_chirps, c.num_samples),
                        dtype=complex)
        # true anchors: range modulated by own-LOS projection of d.
        # Geometric phase at the START frequency f0: the range FFT adds the
        # window-centroid term 2*pi*fb*(N-1)/(2*fs) ~ another half sweep,
        # so the peak-bin phase then evolves at the solver's mid-sweep
        # k_disp exactly (writing it at lam=mid-sweep would inflate the
        # phase-displacement gain by ~1.6% and dominate the error budget)
        for k in range(anchors_az.size):
            r = anchors_r[k] + U[k] @ d[fi]
            fb = 2 * c.slope * r / 2.998e8
            sig = amps[k] * np.exp(1j * (2 * np.pi * fb * t_fast
                                         + 4 * np.pi * c.f0 * r / 2.998e8))
            cube += steer[k][:, :, None, None] * sig[None, None, None, :]
        # ghosts: believed direction g.az, but phase follows the PARENT LOS
        for gi, g in enumerate(ghosts):
            r = g["r"] + U[parent] @ d[fi]
            fb = 2 * c.slope * r / 2.998e8
            sig = g["amp"] * np.exp(1j * (2 * np.pi * fb * t_fast
                                          + 4 * np.pi * c.f0 * r
                                          / 2.998e8))
            cube += steer[anchors_az.size + gi][:, :, None, None] \
                * sig[None, None, None, :]
        cube += rng.normal(0, 2.0, cube.shape) \
            + 1j * rng.normal(0, 2.0, cube.shape)
        out = np.empty((c.num_tx, c.num_rx, c.num_chirps, c.num_samples, 2),
                       dtype=np.int16)
        out[..., 0] = np.round(cube.real)
        out[..., 1] = np.round(cube.imag)
        # 1-indexed filenames, as ColoRadar+ ships them — exercises the
        # bridge's autodetect branch the real rerun will take
        out.tofile(os.path.join(data_dir, f"frame_{fi + 1}.bin"))

    times = np.arange(n) / fps
    np.savetxt(os.path.join(seq_dir, "cascade", "adc_samples",
                            "timestamps.txt"), times)
    np.savetxt(os.path.join(gt_dir, "timestamps.txt"), times)
    np.savetxt(os.path.join(gt_dir, "groundtruth_poses.txt"),
               np.column_stack([d, np.zeros((n, 3)), np.ones(n)]))

    # IMU at 100 Hz: accel = d''(t) + gravity + small bias + noise
    acc = np.gradient(np.gradient(di, ti, axis=0), ti, axis=0)
    acc[:, 2] += 9.80665
    bias = np.array([0.0003, -0.00025, 0.0002])
    acc_meas = acc + bias + rng.normal(0, 0.005, acc.shape)
    np.savetxt(os.path.join(imu_dir, "timestamps.txt"), ti)
    np.savetxt(os.path.join(imu_dir, "imu_data.txt"),
               np.column_stack([acc_meas, np.zeros_like(acc_meas)]))

    return {"anchors_az": anchors_az, "anchors_r": anchors_r,
            "anchors_el": anchors_el, "ghosts": ghosts, "parent": parent,
            "d": d, "times": times, "seq_dir": seq_dir, "calib": calib}


# --------------------------------------------------------------------------
# bundle + figure + check (exp67_report pattern)
# --------------------------------------------------------------------------

def build(root: str) -> dict:
    """The committed fixture bundle: deterministic, dataset-free."""
    truth = write_fixture_p1(root)
    out = run_sequence(root, FIXTURE_SEQ, dwell_s=4.8, n_anchors=9,
                       n_holdout=2, seed=0, seed_mode="gt")
    out_imu = run_sequence(root, FIXTURE_SEQ, dwell_s=4.8, n_anchors=9,
                           n_holdout=2, seed=0, seed_mode="imu")
    out_base = run_sequence(root, FIXTURE_SEQ, dwell_s=4.8, n_anchors=9,
                            n_holdout=2, seed=0, seed_mode="gt",
                            baseline=True)
    U = np.array([los_from_azel(a, e)
                  for a, e in zip(truth["anchors_az"], truth["anchors_el"])])
    u_t = los_from_azel(0.0, 0.0)
    return {
        "fixture": {
            "n_anchors": int(truth["anchors_az"].size),
            "n_ghosts": len(truth["ghosts"]),
            "alpha_true_geometry": alpha_gain(U, u_t),
            "sway_rms_mm": [float(x * 1e3)
                            for x in truth["d"].std(axis=0)],
        },
        "gt_seeded": out,
        "baseline_no_prefilter": {
            "holdout_um_plain_mean": out_base["holdout_um_plain_mean"],
            "holdout_um_consensus_mean":
                out_base["holdout_um_consensus_mean"],
            "consensus_excluded": [d["consensus_excluded"]
                                   for d in out_base["dwells"]],
        },
        "imu_seeded": {
            "int_agreement_vs_gt": [d["int_agreement_vs_gt"]
                                    for d in out_imu["dwells"]],
            "holdout_um_consensus_mean":
                out_imu["holdout_um_consensus_mean"],
        },
    }


def make_figure(data: dict, path: str) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    g = data.get("gt_seeded") or data["run"]
    dw = [d for d in g["dwells"] if "error" not in d]
    fig, axs = plt.subplots(1, 3, figsize=(16.0, 4.6))
    a, b, c = axs

    x = np.arange(len(dw))
    base = data.get("baseline_no_prefilter")
    if base is not None:
        a.axhline(base["holdout_um_plain_mean"], ls=":", color="tab:red")
        a.text(0.02, base["holdout_um_plain_mean"] * 1.02,
               "no pre-filter, plain (baseline mean)", color="tab:red",
               fontsize=9)
    a.bar(x - 0.2, [d["holdout_um_plain"] for d in dw], 0.4,
          label="plain LS", color="gray")
    a.bar(x + 0.2, [d["holdout_um_consensus"] for d in dw], 0.4,
          label="consensus", color="tab:blue")
    a.set_xlabel("dwell")
    a.set_ylabel("held-out residual [um]")
    a.set_title("The P1 prediction, measured:\n"
                "consensus vs plain held-out residual")
    a.legend()

    b.plot(x, [d["consensus_set_size"] for d in dw], "o-",
           color="tab:blue", label="consensus set size")
    b.plot(x, [len(d["flagged_corange"]) for d in dw], "s--",
           color="tab:red", label="co-range flagged")
    b.set_xlabel("dwell")
    b.set_title("Availability per dwell:\nconsensus set + flagged ghosts")
    b.legend()

    al = [d["alpha"]["alpha_abs_p95"] for d in dw]
    c.bar(x, al, 0.5, color="tab:blue")
    c.axhline(ALPHA_GATE, ls="--", color="dimgray")
    c.text(0.02, ALPHA_GATE * 1.05, f"2-D gate {ALPHA_GATE}",
           color="dimgray")
    c.set_xlabel("dwell")
    c.set_ylabel("|alpha| p95 over el bound")
    c.set_title("Per-dwell z-aliasing gain alpha\n(el bound +-5 deg)")

    fig.tight_layout()
    fig.savefig(path, dpi=130)
    plt.close(fig)


def _compare(new, old, path: str = "") -> list:
    bad = []
    if isinstance(new, dict) and isinstance(old, dict):
        for k in set(new) | set(old):
            if k not in new or k not in old:
                bad.append(f"{path}/{k}: missing on one side")
            else:
                bad += _compare(new[k], old[k], f"{path}/{k}")
    elif isinstance(new, list) and isinstance(old, list):
        if len(new) != len(old):
            bad.append(f"{path}: length {len(new)} vs {len(old)}")
        else:
            for i, (x, y) in enumerate(zip(new, old)):
                bad += _compare(x, y, f"{path}[{i}]")
    elif new is None or old is None:
        if new is not old:
            bad.append(f"{path}: {new!r} != {old!r}")
    elif isinstance(new, bool) or isinstance(old, bool) \
            or isinstance(new, str) or isinstance(old, str):
        if new != old:
            bad.append(f"{path}: {new!r} != {old!r}")
    else:
        an, ao = float(new), float(old)
        same = (np.isnan(an) and np.isnan(ao)) or bool(
            np.isclose(an, ao, rtol=1e-9, atol=1e-12, equal_nan=True))
        if not same:
            bad.append(f"{path}: {an!r} != {ao!r}")
    return bad


def _self_test(tmp_root: str) -> None:
    truth = write_fixture_p1(tmp_root)
    calib = CascadeCalib(os.path.join(tmp_root, "calib"))
    seq = CascadeSequence(truth["seq_dir"], calib)
    n = seq.n_frames()

    pick = pick_anchors_v2(seq, 9, 2, 0)
    # (1) the co-range ghost (same bin as the parent) is flagged
    raxis = pick["raxis"]
    parent_r = truth["anchors_r"][truth["parent"]]
    flagged_r = [raxis[rb] for _, rb in pick["flagged_corange"]]
    assert any(abs(fr - parent_r) < 0.15 for fr in flagged_r), \
        f"co-range ghost not flagged: {flagged_r} vs parent {parent_r:.2f}"
    # ...and the far-bin ghost G2 sits inside the anchor set
    g2_az = float(np.rad2deg(truth["ghosts"][1]["az"]))
    anchor_azs = [float(np.rad2deg(a)) for a, _ in pick["anchors"]]
    assert any(abs(a - g2_az) < 1.5 for a in anchor_azs), \
        f"G2 not picked as anchor: {anchor_azs}"

    out = run_dwell_real(seq, 0, n, 9, 2, seed=0)
    # (2) consensus excludes exactly the admitted ghost — by IDENTITY, not
    # count: the excluded azimuth must be G2's, and only G2's
    assert out["consensus_excluded"] == 1, \
        f"consensus exclusions != 1: {out}"
    assert len(out["consensus_excluded_az_deg"]) == 1 and \
        abs(out["consensus_excluded_az_deg"][0] - g2_az) < 1.5, \
        f"excluded wrong anchor: {out['consensus_excluded_az_deg']}"
    assert out["consensus_regime_valid"], out
    # (2b) tight-tolerance arm — the exp67 C.5 mismatch trade, MEASURED:
    # at sigma_theta=0.1 deg the tolerance (~25 um) no longer absorbs the
    # elevation leak sin(el)*dz of the 2-D model (up to ~90 um at the
    # fixture's +-5 deg els and mm-class z sway), so good anchors fall out
    # alongside the ghost. The ghost must STILL be excluded, and the
    # over-exclusion must be visible — the tolerance model on real 3-D
    # motion has to absorb the el-leak term, which the default grid-sigma
    # tolerance does at hallway scales.
    out_tight = run_dwell_real(seq, 0, n, 9, 2, seed=0,
                               sigma_theta_deg=0.1)
    assert any(abs(a - g2_az) < 1.5
               for a in out_tight["consensus_excluded_az_deg"]), \
        f"tight arm kept the ghost: {out_tight['consensus_excluded_az_deg']}"
    assert out_tight["consensus_excluded"] > out["consensus_excluded"], \
        "tight arm did not exhibit the C.5 over-exclusion trade"
    # (3) consensus improves (or matches) the held-out residual
    assert out["holdout_um_consensus"] <= out["holdout_um_plain"] + 1e-9, \
        (out["holdout_um_plain"], out["holdout_um_consensus"])
    # (4) the solve tracks the injected motion to well under 0.1 mm now
    # that the fixture phase scale is exact
    assert max(out["inc_err_rms_mm_consensus"]) < 0.1, out
    # (4b) baseline ablation: with the pre-filter and D_A off, both ghosts
    # enter and consensus must drop both; plain holdout must be worse than
    # the pre-filtered arm's
    out_b = run_dwell_real(seq, 0, n, 9, 2, seed=0, baseline=True)
    assert out_b["consensus_excluded"] >= 2, out_b
    assert out_b["holdout_um_plain"] > out["holdout_um_plain"], \
        (out_b["holdout_um_plain"], out["holdout_um_plain"])
    # (5) IMU-seeded integers mostly agree with GT-seeded
    out_imu = run_dwell_real(seq, 0, n, 9, 2, seed=0, seed_mode="imu")
    assert out_imu["int_seed_mode"] == "imu"
    assert out_imu["int_agreement_vs_gt"] > 0.9, \
        out_imu["int_agreement_vs_gt"]
    # (5b) constant-velocity ghost, pure estimator level: the pair-domain
    # residual of a mis-attributed anchor under sustained velocity is a DC
    # offset — invisible to exp7's mean-removed std, caught by the RMS
    # statistic this port uses
    rngu = np.random.default_rng(3)
    azu = np.deg2rad(np.linspace(-50, 50, 9))
    U2u = np.stack([np.cos(azu), np.sin(azu)], axis=1)
    v_pair = np.array([1.5e-3, 0.4e-3])          # 1.5 mm/pair sustained
    ku = 4 * np.pi / calib.lam
    Yu = (U2u @ v_pair)[:, None] + 0.0115 / ku * rngu.standard_normal(
        (9, 200))
    u_par = np.array([np.cos(np.deg2rad(-35)), np.sin(np.deg2rad(-35))])
    Yu[4] = float(u_par @ v_pair) + 0.0115 / ku * rngu.standard_normal(200)
    cu = consensus_solve(U2u, Yu, 0.0115, ku, sigma_theta_deg=0.1, seed=0)
    assert not cu["keep"][4] and cu["set_size"] == 8, \
        f"DC ghost not excluded: keep={cu['keep']}"
    # (5c) wrap-regime clamp: at walking-pace d_rms the model tolerance
    # exceeds the lambda/4 wrap ceiling and the dwell must say so
    Yw = (U2u @ np.array([0.1, 0.02]))[:, None] \
        + 0.0115 / ku * rngu.standard_normal((9, 200))
    cw = consensus_solve(U2u, Yw, 0.0115, ku, seed=0)
    assert not cw["regime_valid"], cw
    assert cw["tol_m"] <= WRAP_TOL_FRAC * np.pi / ku + 1e-12, cw
    # (6) alpha exactness: noise-free drop-z error equals alpha*K*dz
    U = np.array([los_from_azel(a, e)
                  for a, e in zip(truth["anchors_az"],
                                  truth["anchors_el"])])
    u_t = los_from_azel(0.0, 0.0)
    alpha = alpha_gain(U, u_t)
    d = truth["d"]
    x2 = np.linalg.lstsq(U[:, :2], U @ d.T, rcond=None)[0]
    err_t = u_t[:2] @ x2 - u_t @ d.T
    pred = alpha * d[:, 2]
    assert np.max(np.abs(err_t - pred)) < 1e-9, "alpha law violated"
    print(f"exp5b self_test OK: co-range flagged {len(flagged_r)}, "
          f"consensus excluded G2 (tight tol: ghost out + "
          f"{out_tight['consensus_excluded'] - 1} good over-excluded — "
          f"the C.5 trade), baseline "
          f"holdout {out_b['holdout_um_plain']:.1f} vs pre-filtered "
          f"{out['holdout_um_plain']:.1f} -> consensus "
          f"{out['holdout_um_consensus']:.1f} um, inc err "
          f"{max(out['inc_err_rms_mm_consensus'])*1e3:.0f} um, DC-ghost + "
          f"wrap-regime estimator checks OK, IMU integer agreement "
          f"{out_imu['int_agreement_vs_gt']*100:.1f}%, alpha "
          f"{alpha:+.4f} exact")


def main() -> None:
    import argparse
    import tempfile
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", help="ColoRadar dataset root (real-data mode)")
    ap.add_argument("--sequence", default="ec_hallways_run4")
    ap.add_argument("--frames", default=None)
    ap.add_argument("--n-anchors", type=int, default=9)
    ap.add_argument("--holdout", type=int, default=3)
    ap.add_argument("--seed-mode",
                    choices=["gt", "imu", "gt-rot", "imu-rot",
                             "imu-rot-track"],
                    default="gt",
                    help="gt/imu are the naive world-frame modes; the -rot "
                         "modes (D.7) use GT attitude + the base_to_imu / "
                         "base_to_cascade extrinsics for frame-correct "
                         "cascade-point seeding; imu-rot dead-reckons the "
                         "dwell open-loop (ZUPT bias-calibrated), "
                         "imu-rot-track re-anchors velocity per frame pair "
                         "from the previous increment (the tracker bound)")
    ap.add_argument("--dwell-s", type=float, default=30.0,
                    help="dwell length in seconds (default 30; the hover-"
                         "regime ASPEN windows are shorter than 30 s, so "
                         "they need dwells sized to the still stretch)")
    ap.add_argument("--guard-bins", type=int, default=8,
                    help="near-field guard: zero the first N range bins of "
                         "the picker's energy map (default 8 per the D.6 "
                         "verdict; pass 4 to reproduce the exp5-era guard, "
                         "which admits the cascade coupling residue at "
                         "bin 4)")
    ap.add_argument("--baseline", action="store_true",
                    help="exp5-equivalent picker: no co-range pre-filter, "
                         "no D_A gate (the ablation arm)")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    os.makedirs(RESULTS_DIR, exist_ok=True)
    if args.self_test:
        with tempfile.TemporaryDirectory() as td:
            _self_test(td)
        return
    if args.root:
        out = run_sequence(args.root, args.sequence, frames=args.frames,
                           dwell_s=args.dwell_s,
                           n_anchors=args.n_anchors,
                           n_holdout=args.holdout,
                           seed_mode=args.seed_mode,
                           baseline=args.baseline,
                           guard_bins=args.guard_bins)
        tag = args.sequence + ("_baseline" if args.baseline else "") \
            + f"_{args.seed_mode}" \
            + (f"_{args.frames.replace(':', '-')}" if args.frames else "") \
            + (f"_g{args.guard_bins}" if args.guard_bins != 8 else "")
        json_path = os.path.join(RESULTS_DIR, f"exp5b_{tag}.json")
        fig_path = os.path.join(RESULTS_DIR, f"fig_rbec_exp5b_{tag}.png")
        data = {"run": out, "seed_mode": args.seed_mode,
                "baseline": bool(args.baseline)}
        with open(json_path, "w") as fh:
            json.dump(data, fh, indent=1)
        print(f"wrote {json_path}")
        try:
            make_figure(data, fig_path)
            print(f"wrote {fig_path}")
        except ImportError:
            print("matplotlib not available -- figure skipped")
        print(f"\nheld-out residual: plain "
              f"{out['holdout_um_plain_mean']:.1f} um, consensus "
              f"{out['holdout_um_consensus_mean']:.1f} um")
        return

    json_path = os.path.join(RESULTS_DIR, "exp5b.json")
    fig_path = os.path.join(RESULTS_DIR, "fig_rbec_exp5b.png")
    with tempfile.TemporaryDirectory() as td:
        data = build(td)
    if args.check:
        with open(json_path) as fh:
            committed = json.load(fh)
        bad = _compare(data, committed)
        if bad:
            print(f"MISMATCH ({len(bad)} values):")
            for m in bad[:20]:
                print(" ", m)
            sys.exit(1)
        print(f"exp5b_upgrade --check: regenerated fixture bundle matches "
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
