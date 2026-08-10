"""Virtual-array beam patterns under per-chip-correlated calibration errors.

Answers experiment 1 (radar_rbec_method.md Part F.3 caveat): the classic
random-error sidelobe floor Rp^2 ~= sigma^2/K assumes i.i.d. per-channel
errors, but the MMWCAS residual is partly *correlated per chip* (TI SPRACF4C:
calibration codes are common to all chains within one MMIC). This module
Monte-Carlos the actual floor and the casualty->anchor-beam leakage.

Array model (approximation, stated in the results doc):
  * 86-element contiguous lambda/2 azimuth virtual ULA (SWRU553A: 86
    non-overlapping azimuth positions).
  * Each virtual element = (TX chip, RX chip) pair. The exact TIDEP-01012
    TX-offset map was not transcribed from SWRU553A Fig. 27; we use a
    parameterised plausible mapping (9 azimuth TX on the three slave chips,
    16 RX in four contiguous 4-element chip blocks) and, as a robustness
    check, a seeded random permutation of chip assignments. The correlated-
    floor conclusion must hold for both, or it is mapping-dependent and the
    real map must be transcribed first.
"""

from __future__ import annotations

import numpy as np

from .core import unit  # noqa: F401  (kept for API symmetry)

N_AZ = 86          # non-overlapping azimuth virtual elements
N_CHIPS = 4        # master + 3 slaves


def chip_map(rng: np.random.Generator | None = None,
             permute: bool = False) -> tuple[np.ndarray, np.ndarray]:
    """Return (tx_chip, rx_chip) per virtual element, each in 0..3
    (0 = master, 1..3 = slaves/devices 2..4).

    REAL map, primary-sourced from TIDUEN5A Figures 5 and 6 (read visually,
    2026-08): azimuth TX at lambda/2 positions {0,4,...,32} — TX12/11/10 on
    device 4 at {0,4,8}, TX9/8/7 on device 3 at {12,16,20}, TX6/5/4 on
    device 2 at {24,28,32}; the master's TX1-3 are elevation-only. RX at
    positions {0..3} = device 4, {11..14} = master, {46..49} = device 3,
    {50..53} = device 2 (RX-B/C/A blocks, total span 26.5 lambda as
    annotated). Virtual position p = tx + rx covers 0..85 exactly; of the
    144 (tx,rx) pairs, overlapped positions are resolved deterministically
    in favour of the lowest TX position (TI's channel selection may differ —
    the 86-of-144 subset choice; measured cal data supersedes).

    ``permute=True`` shuffles both assignments with ``rng`` — the
    mapping-sensitivity ablation (a FIXED permutation; hold it constant
    across MC draws).
    """
    tx_pos = np.array([0, 4, 8, 12, 16, 20, 24, 28, 32])
    tx_chips = np.array([3, 3, 3, 2, 2, 2, 1, 1, 1])
    rx_pos = np.array(list(range(0, 4)) + list(range(11, 15))
                      + list(range(46, 50)) + list(range(50, 54)))
    rx_chips = np.array([3] * 4 + [0] * 4 + [2] * 4 + [1] * 4)

    tx_chip = np.full(N_AZ, -1)
    rx_chip = np.full(N_AZ, -1)
    for ti in np.argsort(tx_pos):                 # lowest TX position wins
        for ri in range(rx_pos.size):
            p = tx_pos[ti] + rx_pos[ri]
            if tx_chip[p] == -1:
                tx_chip[p] = tx_chips[ti]
                rx_chip[p] = rx_chips[ri]
    assert (tx_chip >= 0).all(), "virtual array not fully covered"
    if permute:
        assert rng is not None
        tx_chip = rng.permutation(tx_chip)
        rx_chip = rng.permutation(rx_chip)
    return tx_chip, rx_chip


def taper(kind: str = "kaiser30") -> np.ndarray:
    """Amplitude taper. Kaiser windows with beta set for ~-30/-40 dB design
    sidelobes (measured levels are reported by the experiment, not assumed)."""
    if kind == "uniform":
        w = np.ones(N_AZ)
    elif kind == "kaiser30":          # measured design SLL ~ -30.4 dB
        w = np.kaiser(N_AZ, 4.0)
    elif kind == "kaiser40":          # measured design SLL ~ -40.5 dB
        w = np.kaiser(N_AZ, 5.5)
    else:
        raise ValueError(kind)
    return w / w.sum()


def draw_phase_errors(rng: np.random.Generator, sigma_iid_deg: float,
                      sigma_chip_deg: float,
                      chips: tuple[np.ndarray, np.ndarray] | None = None
                      ) -> np.ndarray:
    """Per-virtual-element phase error [rad]: i.i.d. part + TX-chip and
    RX-chip common parts (each chip draw shared by every element using that
    chip). Total per-element variance = sigma_iid^2 + 2*sigma_chip^2.

    Note (review finding): the TX-path and RX-path residuals of one MMIC are
    drawn independently even when tx_chip == rx_chip — a stated
    approximation; a common per-chip LO error would share the draw. The
    measured-cal-vector rerun (validation doc follow-up 5) supersedes this.

    ``chips``: pass a fixed (tx_chip, rx_chip) mapping. The mapping must be
    held FIXED across Monte-Carlo draws — re-drawing it per draw averages
    over the map ensemble and hides any fixed map's discrete spurs (the bug
    the first version of this module had).
    """
    if chips is None:
        chips = chip_map()
    tx_chip, rx_chip = chips
    e_iid = np.deg2rad(sigma_iid_deg) * rng.standard_normal(N_AZ)
    e_tx = np.deg2rad(sigma_chip_deg) * rng.standard_normal(N_CHIPS)
    e_rx = np.deg2rad(sigma_chip_deg) * rng.standard_normal(N_CHIPS)
    return e_iid + e_tx[tx_chip] + e_rx[rx_chip]


def pattern_db(weights: np.ndarray, phase_err: np.ndarray,
               theta_grid: np.ndarray, steer: float = 0.0) -> np.ndarray:
    """Array factor power [dB rel. the on-steer (mainlobe) response] on
    ``theta_grid`` (rad), steered to ``steer``; lambda/2 spacing."""
    n = np.arange(N_AZ)
    grid = np.concatenate([[steer], np.asarray(theta_grid, dtype=float)])
    psi = np.pi * (np.sin(grid)[:, None] - np.sin(steer)) * n[None, :]
    af = (weights[None, :] * np.exp(1j * (psi + phase_err[None, :]))).sum(axis=1)
    p = np.abs(af) ** 2
    return 10.0 * np.log10(p[1:] / p[0])


def leakage_stats(rng: np.random.Generator, sigma_iid_deg: float,
                  sigma_chip_deg: float, taper_kind: str,
                  offsets_deg: np.ndarray, n_mc: int = 500,
                  permute_map: bool = False, steer_deg: float = 0.0) -> dict:
    """Monte Carlo of the anchor-beam response toward a casualty offset by
    ``offsets_deg`` from the anchor steering direction (``steer_deg``;
    off-boresight anchors have wider real-angle beams, so leakage worsens
    ~1/cos(steer) — a review finding this parameter exposes).

    Returns mean (of POWER, then dB — dB-averaging is biased -2.5 dB for
    exponential sidelobe power) and 90th-percentile leakage per offset, the
    far-sidelobe mean-power floor over 10-60 deg from the mainlobe, and the
    taper-aware analytic i.i.d. floor sigma^2 * sum(w^2)/(sum w)^2 for
    comparison. The chip map is drawn ONCE (fixed across MC draws).
    """
    w = taper(taper_kind)
    steer = np.deg2rad(steer_deg)
    off = steer + np.deg2rad(np.asarray(offsets_deg, dtype=float))
    grid = np.concatenate([off, steer + np.deg2rad(np.arange(10.0, 61.0, 1.0))])
    chips = chip_map(rng, permute=True) if permute_map else chip_map()
    leak_p = np.empty((n_mc, off.size))
    floor_p = np.empty(n_mc)
    for m in range(n_mc):
        pe = draw_phase_errors(rng, sigma_iid_deg, sigma_chip_deg, chips=chips)
        pat = pattern_db(w, pe, grid, steer=steer)
        leak_p[m] = 10 ** (pat[: off.size] / 10)
        floor_p[m] = np.mean(10 ** (pat[off.size:] / 10))
    sig_tot = np.deg2rad(np.sqrt(sigma_iid_deg ** 2 + 2 * sigma_chip_deg ** 2))
    analytic = sig_tot ** 2 * (w ** 2).sum() / w.sum() ** 2
    return {
        "offsets_deg": np.asarray(offsets_deg, dtype=float),
        "leak_mean_db": 10 * np.log10(leak_p.mean(axis=0)),
        "leak_p90_db": 10 * np.log10(np.percentile(leak_p, 90, axis=0)),
        "floor_mean_db": 10 * np.log10(float(floor_p.mean())),
        "analytic_iid_floor_db": 10 * np.log10(analytic),
    }


def spur_scan(rng: np.random.Generator, sigma_iid_deg: float,
              sigma_chip_deg: float, taper_kind: str, n_mc: int = 400,
              permute_map: bool = False, top: int = 3) -> dict:
    """Fine sin-space scan of the ERROR-EXCESS power (mean errored pattern
    minus the deterministic error-free pattern) to expose the discrete
    spurious lobes a fixed chip map produces (the default map's period-16 RX
    block structure puts spurs at sin(theta) ~ k/8). This is the observable
    the original experiment lacked (review finding)."""
    w = taper(taper_kind)
    sin_grid = np.linspace(0.03, 0.98, 1500)
    grid = np.arcsin(sin_grid)
    chips = chip_map(rng, permute=True) if permute_map else chip_map()
    p0 = 10 ** (pattern_db(w, np.zeros(N_AZ), grid) / 10)
    acc = np.zeros_like(sin_grid)
    for m in range(n_mc):
        pe = draw_phase_errors(rng, sigma_iid_deg, sigma_chip_deg, chips=chips)
        acc += 10 ** (pattern_db(w, pe, grid) / 10)
    excess = np.maximum(acc / n_mc - p0, 1e-12)
    order = np.argsort(excess)[::-1][:top]
    return {
        "spur_sin": sin_grid[order],
        "spur_deg": np.rad2deg(grid[order]),
        "spur_db": 10 * np.log10(excess[order]),
        "median_excess_db": float(10 * np.log10(np.median(excess))),
    }
