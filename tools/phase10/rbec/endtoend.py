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
    phi_t = (phi_target_geo + psi_common + chip_steps @ mism_t
             + sig_t * rng.standard_normal(t_chirp.size))

    phi_a = np.empty((cfg.n_anchors, t_chirp.size))
    for k in range(cfg.n_anchors):
        geo = K_DISP * (d @ U_true[k])
        inst = psi_common + chip_steps @ mism_a[k]
        clean = np.exp(1j * (geo + inst))
        leak = cfg.leak_amp_ratio * np.exp(1j * (phi_target_geo + inst))
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

    def track(phi_wrapped: np.ndarray, u: np.ndarray,
              is_target: bool) -> np.ndarray:
        nonlocal n_fail_anchor, n_fail_target
        ph = phi_wrapped.reshape(n_frames, C).copy()
        out = np.empty_like(ph)
        out[0] = np.unwrap(ph[0])
        for f in range(1, n_frames):
            burst_u = np.unwrap(ph[f])
            i_prev, i_now = f * C - 1, f * C
            delta_true = d[i_now] - d[i_prev]
            delta_imu = delta_true + imu_err[f]
            pred = out[f - 1, -1] + K_DISP * (delta_imu @ u)
            n_cyc = np.round((pred - burst_u[0]) / (2 * np.pi))
            true_cyc = np.round(((out[f - 1, -1]
                                  + K_DISP * (delta_true @ u))
                                 - burst_u[0]) / (2 * np.pi))
            if n_cyc != true_cyc:
                if is_target:
                    n_fail_target += 1
                else:
                    n_fail_anchor += 1
            out[f] = burst_u + 2 * np.pi * n_cyc
        return out.ravel()

    phi_t_u = track(phi_t, u_t, is_target=True)
    phi_a_u = np.stack([track(phi_a[k], U_true[k], is_target=False)
                        for k in range(cfg.n_anchors)])
    n_fail = n_fail_anchor + n_fail_target

    # --- per-frame solve --------------------------------------------------
    fmean = lambda x: x.reshape(-1, n_frames, C).mean(axis=2)
    ya = fmean(phi_a_u)                    # (N, F)
    yt = fmean(phi_t_u[None, :])[0]        # (F,)
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
    )

    if steps_only and cfg.apll_step_rad > 0:
        raw = band_rms(yt, fs_f, (0.5, 3.0))
        post = band_rms(resid, fs_f, (0.5, 3.0))
        res.step_cmrr_db = 20 * np.log10(raw / post) if post > 0 else np.inf
    return res
