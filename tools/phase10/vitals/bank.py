"""The assembled estimator bank: one call from unwrapped phase to fused
respiration + cardiac estimates (survey B.7-B.9 end to end)."""

from __future__ import annotations

import numpy as np

from .dsp import (CARDIAC_BAND, RESP_BAND, bandpass, estimate_then_notch,
                  remove_impulses, successive_diff)
from .estimators import (Estimate, autocorr, fft_peak, fuse, hmusic, music,
                         peak_intervals)
from .vmd import vmd_cardiac


def process_window(phase: np.ndarray, fs: float,
                   use_vmd: bool = True) -> tuple[Estimate, Estimate, dict]:
    """Full chain on one coherent window of unwrapped phase [rad].
    Returns (respiration, cardiac, diagnostics)."""
    clean = remove_impulses(successive_diff(phase), thresh=1.5)
    x = bandpass(np.cumsum(clean), fs, (0.05, min(8.0, 0.45 * fs)))

    resp_est = [fft_peak(x, fs, RESP_BAND),
                autocorr(x, fs, RESP_BAND),
                peak_intervals(x, fs, RESP_BAND),
                music(x, fs, RESP_BAND)]
    resp = fuse(resp_est, tol_hz=0.03)

    card_sig = estimate_then_notch(x, fs, resp.rate_hz)
    card_est = [fft_peak(card_sig, fs, CARDIAC_BAND),
                autocorr(card_sig, fs, CARDIAC_BAND),
                peak_intervals(card_sig, fs, CARDIAC_BAND),
                music(card_sig, fs, CARDIAC_BAND),
                hmusic(card_sig, fs, CARDIAC_BAND, n_harmonics=2)]
    if use_vmd:
        card_est.append(Estimate(
            fft_peak(vmd_cardiac(card_sig, fs), fs, CARDIAC_BAND).rate_hz,
            fft_peak(vmd_cardiac(card_sig, fs), fs, CARDIAC_BAND).confidence,
            "vmd+fft"))
    card = fuse(card_est, tol_hz=0.08)
    return resp, card, {"resp_est": resp_est, "card_est": card_est}


def sliding(phase: np.ndarray, fs: float, window_s: float = 30.0,
            step_s: float = 5.0, **kw):
    """Yield (t_center, resp, cardiac) over sliding windows."""
    n = phase.size
    w = int(window_s * fs)
    s = int(step_s * fs)
    for start in range(0, n - w + 1, s):
        seg = phase[start:start + w]
        resp, card, _ = process_window(seg, fs, **kw)
        yield (start + w / 2) / fs, resp, card
