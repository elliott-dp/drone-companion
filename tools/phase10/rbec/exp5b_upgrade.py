"""exp5b: the P1 upgrade pass over the exp5 real-data harness.

Implements every item of thesis_plan.md §4 P1 ("exp5 upgrade run") as pure
additions to the exp5 machinery, per the re-scoping in
radar_rbec_validation.md §F.4 and radar_rbec_validation_exp67.md §B.4/§C.4/§D:

  A. **Per-dwell z-aliasing gain alpha** (exp67 B.4): the exp6 closed form
     computed from the dwell's anchor geometry. Wall/hallway anchors carry no
     measured elevation, so alongside the nominal value the report carries a
     seeded Monte-Carlo p95 |alpha| over an elevation bound (default +-5 deg)
     — the quantity that justifies keeping the 2-D solve when it stays under
     the ~0.02 gate. (alpha's own full uncertainty propagation is P2, not
     here.)
  B. **Co-range structural pre-filter** (exp67 C.4): candidate anchors
     sharing a range bin (within ``corange_tol_bins``) are clustered before
     any solve; only the strongest of a cluster is admitted, the rest are
     flagged as suspected sidelobe/ring ghosts and reported. D_A is computed
     per candidate and kept for its actual job — rejecting incoherent
     clutter — never cited as ghost protection.
  C. **Subset-consensus solve** (exp7, adapted 2-D): RANSAC-style minimal
     3-anchor subsets over the dwell's frame pairs, agreement tolerance from
     the a-priori sigma_phi + T3 variance model (frame-pair phase sigma is
     sqrt(2)*sigma_phi; sigma_theta defaults to the 2.5-deg-grid DoA
     quantization, 2.5/sqrt(12) ~ 0.72 deg), condition-guarded geometry
     check, full refit on the winning set, plain-LS fallback. Consensus set
     size is reported per dwell as the availability metric.
  D. **IMU-seeded integers** (F.4 "IMU-seeded (not GT-seeded)"): a ColoRadar
     ``imu/`` reader plus a deliberately honest dead-reckoning seed —
     attitude assumed world-aligned and dwell-start velocity taken from
     ground truth, acceleration double-integrated in between (a full INS
     mechanisation is out of scope, as in endtoend.py's IMU note). Scored as
     the fraction of frame pairs whose fixed integer agrees with the
     GT-seeded fix — "how far IMU seeding gets", per the exp5 docstring.
  E. **Full-sequence run**: the sequence is streamed in dwells (default
     30 s); anchors are re-picked per dwell (anchor migration), and the
     per-dwell report carries alpha, the consensus set size, flagged
     ghosts, and the held-out static-cell residual for the plain and
     consensus solves side by side.

Prediction on record (thesis_plan.md P1): the 555 um held-out residual
improves once co-range ghosts are excluded, or the identical-range clusters
were not ghosts — informative either way. This module measures exactly that
via ``holdout_um_plain`` vs ``holdout_um_consensus``.

Without a dataset the module validates end-to-end on a seeded synthetic
fixture in the exact ColoRadar layout (multi-anchor scene, two injected
coherent ghosts, 3-D platform motion, synthetic IMU) and ``--check``
verifies the committed fixture bundle ``docs/phase10/results/exp5b.json``
in the exp67_report sense (rtol 1e-9). With data:

    python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
        --sequence ec_hallways_run4 [--frames 0:2192] [--seed-mode imu]

writes ``exp5b_<sequence>.json`` + figure next to the fixture bundle.

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

from .coloradar_bridge import (CascadeCalib, CascadeSequence, range_fft,
                               steer_beam, write_fixture)
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
    el0 = np.zeros_like(az) if anchor_el is None \
        else np.asarray(anchor_el, dtype=float)
    u_t = los_from_azel(target_az, target_el)
    U0 = np.array([los_from_azel(a, e) for a, e in zip(az, el0)])
    nominal = alpha_gain(U0, u_t)
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
                    corange_tol_bins: int = 1) -> dict:
    """exp5's energy-ranked picker + the co-range pre-filter + D_A.
    Returns admitted anchors, holdout cells, flagged co-range ghosts, and
    the diagnostics the per-dwell report carries."""
    c = seq.calib
    az_grid = np.deg2rad(np.linspace(-55, 55, 45))
    raxis = c.range_axis()
    max_bin = int(np.searchsorted(raxis, max_range_m))
    wf = range(f_start, min(f_start + warmup_frames, seq.n_frames()))
    emap = energy_map_fast(seq, wf, az_grid, max_bin)
    emap[:, :4] = 0.0                        # near-field/leakage guard
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
    admitted, flagged = corange_prefilter(cands, corange_tol_bins)
    pool = admitted[:2 * (n_anchors + n_holdout)]
    cells = [(az, rb) for az, rb, _ in pool]
    da = amplitude_dispersion(seq, cells, wf)
    coherent = [(cell, d) for cell, d in zip(cells, da) if d < DA_GATE]
    cells = [cell for cell, _ in coherent][:n_anchors + n_holdout]
    da = [d for _, d in coherent][:n_anchors + n_holdout]
    anchors = cells[:n_anchors]
    holdout = cells[n_anchors:n_anchors + n_holdout]
    if len(anchors) < 4 or not holdout:
        raise ValueError(
            f"anchor pool too thin after gates: {len(anchors)} anchors, "
            f"{len(holdout)} holdout (candidates {len(cands)}, co-range "
            f"flagged {len(flagged)}, D_A dropped "
            f"{len(cells) - len(coherent) if len(coherent) < len(cells) else 0})")
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
    solve. Y is (N, P) integer-fixed LOS increments in metres; the agreement
    statistic is the per-anchor residual RMS over subsampled pairs, against
    the a-priori tolerance from the sigma_phi + T3 variance model (frame-pair
    phase sigma = sqrt(2)*sigma_phi)."""
    N, P = Y.shape
    x_all = np.linalg.lstsq(U2, Y, rcond=None)[0]           # (2, P)
    d_rms = float(np.sqrt(np.mean(np.sum(x_all ** 2, axis=0))))
    sig_pair = np.sqrt(2.0) * sigma_phi
    t3 = k_disp * np.deg2rad(sigma_theta_deg) * d_rms
    tol_m = CONSENSUS_TOL_MULT * np.sqrt(sig_pair ** 2 + t3 ** 2) / k_disp
    rs = np.random.default_rng(seed + 77)
    cols = np.linspace(0, P - 1, min(P, n_score_cols)).astype(int)
    best_keep, best_score = None, -1
    for _ in range(n_draws):
        idx = rs.choice(N, size=subset, replace=False)
        Us = U2[idx]
        sv = np.linalg.svd(Us, compute_uv=False)
        if sv[-1] < 1e-9 or sv[0] / sv[-1] > COND_MAX:
            continue                          # rank/condition guard
        x = np.linalg.lstsq(Us, Y[np.ix_(idx, cols)], rcond=None)[0]
        r = Y[:, cols] - U2 @ x
        agree = r.std(axis=1) < tol_m
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
            "tol_m": float(tol_m)}


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


def read_imu(seq_dir: str):
    """ColoRadar imu/ reader: rows 'ax ay az gx gy gz' + timestamps.txt.
    Returns (times, accel) or None when absent/malformed."""
    d = os.path.join(seq_dir, "imu")
    try:
        t = np.loadtxt(os.path.join(d, "timestamps.txt"))
        data = np.loadtxt(os.path.join(d, "imu_data.txt"))
        if data.ndim != 2 or data.shape[1] < 3 or t.size != data.shape[0]:
            return None
        return t, data[:, :3]
    except OSError:
        return None


def imu_increments(imu_t: np.ndarray, accel: np.ndarray,
                   radar_times: np.ndarray, gpos: np.ndarray,
                   dwell_start: int, gravity: float = 9.80665) -> np.ndarray:
    """Dead-reckoned per-frame-pair displacement increments over one dwell.

    Honesty note (mirrors endtoend.py's IMU treatment): attitude is assumed
    world-aligned (the fixture convention; real runs must rotate accel by
    the GT attitude — extrinsics TODO, as in exp5's gt_increments), gravity
    is subtracted as a constant +z, and the initial velocity comes from the
    GT finite difference at the dwell start; from there acceleration is
    double-integrated. This measures "how far IMU seeding gets", not
    full-INS performance."""
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
    if ts.size < 2:
        return np.diff(gpos, axis=0)
    dt = np.diff(ts)
    v = np.concatenate([[v0], v0 + np.cumsum(asel[:-1] * dt[:, None],
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
                   seed_mode: str = "gt") -> dict:
    """One dwell of the upgraded pipeline; returns a JSON-ready dict."""
    c = seq.calib
    k_disp = 4 * np.pi / c.lam
    pick = pick_anchors_v2(seq, n_anchors, n_holdout, f0)
    anchors, holdout = pick["anchors"], pick["holdout"]
    az = np.array([a for a, _ in anchors])
    U2 = np.stack([np.cos(az), np.sin(az)], axis=1)
    Uh2 = np.array([[np.cos(a), np.sin(a)]
                    for a, _ in holdout]).reshape(-1, 2)

    ph_a = track_phases(seq, anchors, f0, f1)
    ph_h = track_phases(seq, holdout, f0, f1)

    gpos = gt_positions(seq)
    dinc_gt = np.diff(gpos, axis=0)[f0:f1 - 1]
    imu_used = False
    if seed_mode == "imu":
        seq_root = os.path.dirname(os.path.dirname(seq.dir.rstrip("/")))
        imu = read_imu(seq_root)
        if imu is None:
            print("  imu/ absent or malformed -- falling back to GT seeding")
            dinc_seed = dinc_gt
        else:
            dinc_seed = imu_increments(imu[0], imu[1], seq.times,
                                       gpos, f0)[f0:f1 - 1]
            imu_used = True
    else:
        dinc_seed = dinc_gt

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
        "int_seed_mode": "imu" if imu_used else "gt",
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
                 seed_mode: str = "gt") -> dict:
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
        dwells.append(run_dwell_real(seq, s, e, n_anchors, n_holdout,
                                     seed + len(dwells),
                                     seed_mode=seed_mode))
        s = e
    hp = [d["holdout_um_plain"] for d in dwells]
    hc = [d["holdout_um_consensus"] for d in dwells]
    return {
        "sequence": sequence, "gt_source": seq.gt_source,
        "n_frames": int(f1 - f0), "frame_dt_s": frame_dt,
        "n_dwells": len(dwells),
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
    ghosts = [
        {"az": np.deg2rad(12.5), "r": anchors_r[parent], "amp": 140.0},
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
        # true anchors: range modulated by own-LOS projection of d
        for k in range(anchors_az.size):
            r = anchors_r[k] + U[k] @ d[fi]
            fb = 2 * c.slope * r / 2.998e8
            sig = amps[k] * np.exp(1j * (2 * np.pi * fb * t_fast
                                         + 4 * np.pi * r / lam))
            cube += steer[k][:, :, None, None] * sig[None, None, None, :]
        # ghosts: believed direction g.az, but phase follows the PARENT LOS
        for gi, g in enumerate(ghosts):
            r = g["r"] + U[parent] @ d[fi]
            fb = 2 * c.slope * r / 2.998e8
            sig = g["amp"] * np.exp(1j * (2 * np.pi * fb * t_fast
                                          + 4 * np.pi * r / lam))
            cube += steer[anchors_az.size + gi][:, :, None, None] \
                * sig[None, None, None, :]
        cube += rng.normal(0, 2.0, cube.shape) \
            + 1j * rng.normal(0, 2.0, cube.shape)
        out = np.empty((c.num_tx, c.num_rx, c.num_chirps, c.num_samples, 2),
                       dtype=np.int16)
        out[..., 0] = np.round(cube.real)
        out[..., 1] = np.round(cube.imag)
        out.tofile(os.path.join(data_dir, f"frame_{fi}.bin"))

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

    g = data["gt_seeded"]
    dw = g["dwells"]
    fig, axs = plt.subplots(1, 3, figsize=(16.0, 4.6))
    a, b, c = axs

    x = np.arange(len(dw))
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
    # (2) consensus excludes the admitted ghost
    assert out["consensus_excluded"] >= 1, \
        f"consensus excluded nothing: {out}"
    # (3) consensus improves (or matches) the held-out residual
    assert out["holdout_um_consensus"] <= out["holdout_um_plain"] + 1e-9, \
        (out["holdout_um_plain"], out["holdout_um_consensus"])
    # (4) the solve tracks the injected motion to sub-half-mm accuracy
    assert max(out["inc_err_rms_mm_consensus"]) < 0.5, out
    # (5) IMU-seeded integers mostly agree with GT-seeded
    out_imu = run_dwell_real(seq, 0, n, 9, 2, seed=0, seed_mode="imu")
    assert out_imu["int_seed_mode"] == "imu"
    assert out_imu["int_agreement_vs_gt"] > 0.9, \
        out_imu["int_agreement_vs_gt"]
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
          f"consensus excluded {out['consensus_excluded']}, holdout "
          f"{out['holdout_um_plain']:.1f} -> "
          f"{out['holdout_um_consensus']:.1f} um, IMU integer agreement "
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
    ap.add_argument("--seed-mode", choices=["gt", "imu"], default="gt")
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
                           n_anchors=args.n_anchors,
                           n_holdout=args.holdout,
                           seed_mode=args.seed_mode)
        json_path = os.path.join(RESULTS_DIR,
                                 f"exp5b_{args.sequence}.json")
        fig_path = os.path.join(RESULTS_DIR,
                                f"fig_rbec_exp5b_{args.sequence}.png")
        data = {"gt_seeded": out}
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
