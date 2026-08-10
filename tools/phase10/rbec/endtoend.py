"""End-to-end simulation of the RBEC deterministic solve (experiment 3).

Simulates, at chirp granularity, a 30 s hover dwell: 3-axis platform sway +
rotor vibration lines, N anchor beams with fixed per-anchor angle errors,
casualty leakage into anchor beams (complex-domain), master-APLL common
phase steps and per-chip residual steps, per-chirp phase noise; then runs
the RBEC pipeline: within-burst unwrap, IMU-seeded integer fixing across the
inter-frame gap, per-frame (angle-error-aware) weighted least squares,
target-phase prediction and subtraction. Outputs the in-band residual error
against the paper's budgets, the integer-fix failure count, and the
common-mode rejection of the injected steps.

Deliberate model boundaries (stated in the results doc):
  * Beam formation is parametric (leakage ratio per anchor), not a full
    array snapshot — the array-level leakage numbers come from experiment 1.
  * The IMU is modelled as a per-gap displacement-increment error, sized
    from accel-noise arithmetic, not a full INS mechanisation.
  * No range/beam migration: sway RMS is kept within the validity bounds
    the paper states (range bin/10, azimuth PSF); pushing beyond needs the
    envelope-shift machinery of validation rung V1.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

from .core import (CARDIAC_BAND, CARDIAC_AMP_M, K_DISP, LAMBDA, RESP_AMP_M,
                   RESP_BAND, band_rms, los_from_azel, shaped_noise)


@dataclass
class SimConfig:
    duration_s: float = 30.0
    frame_hz: float = 20.0
    chirps_per_burst: int = 12
    chirp_spacing_s: float = 250e-6
    # platform motion
    sway_rms_m: float = 0.02          # per axis; knee 0.3 Hz
    sway_knee_hz: float = 0.3
    vib_lines: tuple = ((93.0, 76e-6), (187.0, 30e-6))   # (Hz, m) per axis z
    # scene
    n_anchors: int = 9
    anchor_az_span_deg: float = 100.0
    anchor_el_lo_deg: float = -45.0
    anchor_el_hi_deg: float = -15.0
    target_az_deg: float = 0.0
    target_el_deg: float = -30.0
    # errors
    anchor_angle_err_deg: float = 0.1     # 1-sigma per axis, fixed per dwell
    anchor_snr_db: float = 25.0           # per chirp
    target_snr_db: float = 15.0
    leak_amp_ratio: float = 0.0316        # casualty->anchor amplitude leak (-30 dB power)
    imu_gap_sigma_m: float = 50e-6        # per-axis displacement error per gap
    # instrument steps
    apll_step_rad: float = 0.05           # master common step, every ~1 s
    chip_step_rad: float = 0.005          # per-chip step size
    beam_chip_mismatch: float = 0.1       # weight dissimilarity factor
    # chest
    resp_hz: float = 0.25
    cardiac_hz: float = 1.17
    # estimator
    use_angle_aware_wls: bool = True
    solve_iters: int = 2
    # heterogeneous-anchor option: anchor 0 becomes a corner reflector
    # (angle error x0.05, +20 dB SNR) — the case WLS exists for
    corner_anchor: bool = False
    # mitigations (validation follow-up 2)
    chest_prior: bool = False      # chest-velocity prior on the target seam
    raim_seam: bool = False        # anchor-consensus IMU correction per seam
    slip_repair: bool = False      # post-track 2pi cycle-slip repair
    # leakage topologies (validation follow-up 1)
    leak_topology: str = "all"     # 'all' anchors leak, or 'single' (anchor 0)
    target_leak_ratio: float = 0.0  # anchor-0 echo into the TARGET beam


@dataclass
class SimResult:
    resp_err_rms_m: float
    cardiac_err_rms_m: float
    cardiac_err_rms_rad: float
    raw_cardiac_motion_m: float
    integer_fix_failures: int
    integer_fail_target: int
    integer_fail_anchor: int
    n_seams: int
    step_cmrr_db: float | None
    extras: dict = field(default_factory=dict)


def chest_waveform(t: np.ndarray, cfg: SimConfig) -> np.ndarray:
    """LOS-projected chest displacement [m]: respiration + harmonics
    (survey B.8) + cardiac fundamental with a 2nd harmonic."""
    resp = RESP_AMP_M * (np.sin(2 * np.pi * cfg.resp_hz * t)
                         + 0.30 * np.sin(2 * np.pi * 2 * cfg.resp_hz * t + 0.7)
                         + 0.15 * np.sin(2 * np.pi * 3 * cfg.resp_hz * t + 1.1))
    card = CARDIAC_AMP_M * (np.sin(2 * np.pi * cfg.cardiac_hz * t)
                            + 0.40 * np.sin(2 * np.pi * 2 * cfg.cardiac_hz * t + 0.3))
    return resp + card


def platform_motion(t_grid: np.ndarray, fs: float, cfg: SimConfig,
                    rng: np.random.Generator) -> np.ndarray:
    """(len(t),3) displacement on a uniform grid."""
    n = t_grid.size
    d = np.stack([shaped_noise(n, fs, cfg.sway_knee_hz, cfg.sway_rms_m, rng)
                  for _ in range(3)], axis=1)
    for f_hz, amp in cfg.vib_lines:
        ph = rng.uniform(0, 2 * np.pi, 3)
        d += amp * np.sin(2 * np.pi * f_hz * t_grid[:, None] + ph[None, :])
    return d


def step_train(t: np.ndarray, step_rad: float, rate_hz: float,
               rng: np.random.Generator) -> np.ndarray:
    """Piecewise-constant phase from steps of +-step_rad at ~rate_hz
    (regular schedule, random signs — SPRACF4C: cal fires each ~1 s)."""
    out = np.zeros_like(t)
    if step_rad <= 0:
        return out
    times = np.arange(1.0 / rate_hz, t[-1], 1.0 / rate_hz)
    times += rng.uniform(-0.1, 0.1, times.size) / rate_hz
    for tt in times:
        out[t >= tt] += step_rad * rng.choice([-1.0, 1.0])
    return out


def run(cfg: SimConfig, seed: int, motion_on: bool = True,
        steps_only: bool = False) -> SimResult:
    rng = np.random.default_rng(seed)

    # --- timing -----------------------------------------------------------
    n_frames = int(round(cfg.duration_s * cfg.frame_hz))
    burst = np.arange(cfg.chirps_per_burst) * cfg.chirp_spacing_s
    t_chirp = (np.arange(n_frames)[:, None] / cfg.frame_hz
               + burst[None, :]).ravel()                       # (F*C,)
    fs_grid = 4000.0
    t_grid = np.arange(0.0, cfg.duration_s + 2.0 / fs_grid, 1.0 / fs_grid)

    # --- truth ------------------------------------------------------------
    d_grid = platform_motion(t_grid, fs_grid, cfg, rng)
    if not motion_on:
        d_grid[:] = 0.0
    d = np.stack([np.interp(t_chirp, t_grid, d_grid[:, i]) for i in range(3)],
                 axis=1)                                       # (F*C,3)
    chest = chest_waveform(t_chirp, cfg)
    if steps_only:
        chest[:] = 0.0

    # --- scene ------------------------------------------------------------
    u_t = los_from_azel(np.deg2rad(cfg.target_az_deg),
                        np.deg2rad(cfg.target_el_deg))
    az = np.deg2rad(np.linspace(-cfg.anchor_az_span_deg / 2,
                                cfg.anchor_az_span_deg / 2, cfg.n_anchors))
    el = np.deg2rad(rng.uniform(cfg.anchor_el_lo_deg, cfg.anchor_el_hi_deg,
                                cfg.n_anchors))
    U_true = np.array([los_from_azel(a, e) for a, e in zip(az, el)])
    derr_k = np.full(cfg.n_anchors, np.deg2rad(cfg.anchor_angle_err_deg))
    snr_k = np.full(cfg.n_anchors, cfg.anchor_snr_db)
    if cfg.corner_anchor:
        derr_k[0] *= 0.05
        snr_k[0] += 20.0
    U_est = np.array([los_from_azel(a + derr_k[k] * rng.standard_normal(),
                                    e + derr_k[k] * rng.standard_normal())
                      for k, (a, e) in enumerate(zip(az, el))])

    # --- instrument steps -------------------------------------------------
    psi_common = step_train(t_chirp, cfg.apll_step_rad, 1.0, rng)
    chip_steps = np.stack([step_train(t_chirp, cfg.chip_step_rad, 1.0, rng)
                           for _ in range(4)], axis=1)
    # per-beam residual of per-chip steps: weight-dissimilarity model
    mism_a = cfg.beam_chip_mismatch * rng.standard_normal((cfg.n_anchors, 4))
    mism_t = cfg.beam_chip_mismatch * rng.standard_normal(4)

    # --- received phases (complex where leakage matters) ------------------
    sig_ak = 1.0 / np.sqrt(2 * 10 ** (snr_k / 10))
    sig_t = 1.0 / np.sqrt(2 * 10 ** (cfg.target_snr_db / 10))
    phi_target_geo = K_DISP * (d @ u_t) + K_DISP * chest
    if cfg.target_leak_ratio > 0:
        # anchor-0 echo entering the target beam through its sidelobe: the
        # reverse leakage direction the review flagged (anchors are strong)
        anchor0_phase = K_DISP * (d @ U_true[0]) + psi_common
        t_clean = np.exp(1j * (phi_target_geo + psi_common + chip_steps @ mism_t))
        t_leak = cfg.target_leak_ratio * np.exp(1j * anchor0_phase)
        phi_t = (np.angle(t_clean + t_leak)
                 + sig_t * rng.standard_normal(t_chirp.size))
    else:
        phi_t = (phi_target_geo + psi_common + chip_steps @ mism_t
                 + sig_t * rng.standard_normal(t_chirp.size))

    phi_a = np.empty((cfg.n_anchors, t_chirp.size))
    for k in range(cfg.n_anchors):
        geo = K_DISP * (d @ U_true[k])
        inst = psi_common + chip_steps @ mism_a[k]
        clean = np.exp(1j * (geo + inst))
        rho = cfg.leak_amp_ratio if (cfg.leak_topology == "all" or k == 0) \
            else 0.0
        leak = rho * np.exp(1j * (phi_target_geo + inst))
        noisy = clean + leak
        phi_a[k] = np.angle(noisy) + sig_ak[k] * rng.standard_normal(t_chirp.size)

    # --- tracking: within-burst unwrap + IMU-seeded seam fixing -----------
    C = cfg.chirps_per_burst
    n_fail_anchor = 0
    n_fail_target = 0

    # ONE physical IMU: one displacement-increment error per gap, shared by
    # every track and projected via each u (review fix — the first version
    # drew independent errors per track, breaking the cross-anchor failure
    # correlation the RAIM mitigation relies on).
    imu_err = cfg.imu_gap_sigma_m * rng.standard_normal((n_frames, 3))

    # Latent-trap guard (review finding): the failure counter assumes the
    # target's own chest motion across a gap never exceeds the pi margin by
    # itself; assert it so raising chest parameters cannot silently corrupt.
    gap_idx = np.arange(1, n_frames) * C
    chest_gap = np.abs(chest[gap_idx] - chest[gap_idx - 1]).max()
    assert K_DISP * chest_gap < np.pi, "chest motion per gap exceeds pi"

    wrap = lambda x: (x + np.pi) % (2 * np.pi) - np.pi

    # Unified per-frame tracking: anchors are fixed FIRST each seam so that
    # (raim_seam) their sub-integer innovations — which contain the shared
    # IMU error projected on each u_k and no chest term — can be LS-solved
    # for the common IMU error and the correction applied to the TARGET's
    # prediction before its integer fix. This is the RAIM design the method
    # paper promises, exploiting exactly the failure asymmetry exp3 found:
    # anchors are unambiguous where the target seam is not.
    ph_t = phi_t.reshape(n_frames, C)
    ph_a = phi_a.reshape(cfg.n_anchors, n_frames, C)
    out_t = np.empty_like(ph_t)
    out_a = np.empty_like(ph_a)
    out_t[0] = np.unwrap(ph_t[0])
    for k in range(cfg.n_anchors):
        out_a[k, 0] = np.unwrap(ph_a[k, 0])

    # anti-cascade chest prior: median of the last 5 WRAPPED res-diffs.
    # A slipped frame shifts res by 2pi persistently — wrapped diffs are
    # unaffected except for one transition sample, which the median rejects
    # (the first, cascading implementation of this prior is why the naive
    # res_prev1 - res_prev2 version failed catastrophically; see exp4).
    cum_imu = np.zeros(3)
    res_hist: list[float] = []
    diff_hist: list[float] = []

    for f in range(1, n_frames):
        i_prev, i_now = f * C - 1, f * C
        delta_true = d[i_now] - d[i_prev]
        delta_imu = delta_true + imu_err[f]

        innov = np.empty(cfg.n_anchors)
        for k in range(cfg.n_anchors):
            burst_u = np.unwrap(ph_a[k, f])
            pred = out_a[k, f - 1, -1] + K_DISP * (delta_imu @ U_true[k])
            n_cyc = np.round((pred - burst_u[0]) / (2 * np.pi))
            true_cyc = np.round(((out_a[k, f - 1, -1]
                                  + K_DISP * (delta_true @ U_true[k]))
                                 - burst_u[0]) / (2 * np.pi))
            if n_cyc != true_cyc:
                n_fail_anchor += 1
            out_a[k, f] = burst_u + 2 * np.pi * n_cyc
            innov[k] = (burst_u[0] + 2 * np.pi * n_cyc) - pred

        delta_for_target = delta_imu
        if cfg.raim_seam:
            # innov_k ~= -K (imu_err[f] . u_k) + noise  ->  LS estimate
            err_hat, *_ = np.linalg.lstsq(U_true, -innov / K_DISP, rcond=None)
            delta_for_target = delta_imu - err_hat

        burst_u = np.unwrap(ph_t[f])
        pred = out_t[f - 1, -1] + K_DISP * (delta_for_target @ u_t)
        if cfg.chest_prior and len(diff_hist) >= 2:
            pred += float(np.median(diff_hist[-5:]))
        n_cyc = np.round((pred - burst_u[0]) / (2 * np.pi))
        true_cyc = np.round(((out_t[f - 1, -1]
                              + K_DISP * (delta_true @ u_t))
                             - burst_u[0]) / (2 * np.pi))
        if n_cyc != true_cyc:
            n_fail_target += 1
        out_t[f] = burst_u + 2 * np.pi * n_cyc

        cum_imu = cum_imu + delta_imu
        res = out_t[f, -1] - K_DISP * (cum_imu @ u_t)
        if res_hist:
            diff_hist.append(float(wrap(res - res_hist[-1])))
        res_hist.append(float(res))

    phi_t_u = out_t.ravel()
    phi_a_u = out_a.reshape(cfg.n_anchors, -1)
    n_fail = n_fail_anchor + n_fail_target

    # --- per-frame solve --------------------------------------------------
    fmean = lambda x: x.reshape(-1, n_frames, C).mean(axis=2)
    ya = fmean(phi_a_u)                    # (N, F)
    yt = fmean(phi_t_u[None, :])[0]        # (F,)

    n_repairs = 0
    if cfg.slip_repair:
        # GNSS-style cycle-slip repair (causal per frame): reference each
        # track to the IMU-integrated platform prediction; a residual
        # frame-to-frame jump near a 2pi multiple is a slipped integer and
        # is snapped back. Works because everything else in the residual
        # (chest slope, noise, steps) is << pi per frame — except when the
        # IMU itself is so bad the detector's own noise nears pi, which the
        # results table shows honestly. NOTE: one shared IMU error means
        # slips co-occur across tracks; this per-track repair still works
        # because the reference subtracts the same shared prediction.
        cum = np.cumsum(imu_err, axis=0)   # IMU-error part of the reference
        cum[0] = 0.0

        def repair(y: np.ndarray, u: np.ndarray) -> np.ndarray:
            nonlocal n_repairs
            # platform prediction from IMU = true motion + accumulated error
            d_frame = fmean((d @ u)[None, :])[0]
            ref = K_DISP * (d_frame + cum @ u)
            r = y - ref
            dr = np.diff(r)
            k = np.round(dr / (2 * np.pi))
            k[np.abs(dr - 2 * np.pi * k) > 0.9 * np.pi] = 0  # ambiguous: skip
            n_repairs += int(np.count_nonzero(k))
            corr = np.concatenate([[0.0], np.cumsum(2 * np.pi * k)])
            return y - corr

        yt = repair(yt, u_t)
        for kk in range(cfg.n_anchors):
            ya[kk] = repair(ya[kk], U_true[kk])

    ya = ya - ya[:, :1]
    yt = yt - yt[0]

    d_hat = np.zeros((n_frames, 3))
    sig_frame_k = sig_ak / np.sqrt(C)
    for f in range(1, n_frames):
        w = np.ones(cfg.n_anchors)
        dprev = d_hat[f - 1]
        for _ in range(cfg.solve_iters if cfg.use_angle_aware_wls else 1):
            if cfg.use_angle_aware_wls:
                var = np.empty(cfg.n_anchors)
                for k in range(cfg.n_anchors):
                    perp = dprev - (dprev @ U_est[k]) * U_est[k]
                    var[k] = (sig_frame_k[k] ** 2
                              + (K_DISP * derr_k[k]
                                 * np.linalg.norm(perp)) ** 2)
                w = 1.0 / var
            A = (U_est * w[:, None]).T @ U_est
            b = (U_est * w[:, None]).T @ (ya[:, f] / K_DISP)
            dprev = np.linalg.solve(A, b)
        d_hat[f] = dprev

    # --- residual against truth ------------------------------------------
    pred_t = K_DISP * (d_hat @ u_t)
    resid = yt - pred_t                          # should be K*chest (+err)
    chest_frame = fmean(chest[None, :])[0]
    chest_frame = chest_frame - chest_frame[0]
    err = resid / K_DISP - chest_frame           # metres

    fs_f = cfg.frame_hz
    # raw platform motion in the cardiac band, on frame means (uniform grid;
    # the first version fed the bursty chirp grid to band_rms — 16 % error)
    raw_frames = fmean((d @ u_t)[None, :])[0]
    res = SimResult(
        resp_err_rms_m=band_rms(err, fs_f, RESP_BAND),
        cardiac_err_rms_m=band_rms(err, fs_f, CARDIAC_BAND),
        cardiac_err_rms_rad=band_rms(err, fs_f, CARDIAC_BAND) * K_DISP,
        raw_cardiac_motion_m=band_rms(raw_frames, fs_f, CARDIAC_BAND),
        integer_fix_failures=n_fail,
        integer_fail_target=n_fail_target,
        integer_fail_anchor=n_fail_anchor,
        n_seams=(n_frames - 1) * (cfg.n_anchors + 1),
        step_cmrr_db=None,
        extras={"n_repairs": n_repairs},
    )

    if steps_only and cfg.apll_step_rad > 0:
        raw = band_rms(yt, fs_f, (0.5, 3.0))
        post = band_rms(resid, fs_f, (0.5, 3.0))
        res.step_cmrr_db = 20 * np.log10(raw / post) if post > 0 else np.inf
    return res
