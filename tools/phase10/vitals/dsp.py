"""Band separation and phase-domain conditioning for the vitals estimator
bank (survey radar_dsp_ml_survey.md B.7/B.8 made executable).

Offline chain — filters are FFT-domain zero-phase (exactly reproducible,
no causality requirement; the live-tier causal ports are a 10.3 concern).
NumPy-only.
"""

from __future__ import annotations

import numpy as np

RESP_BAND = (0.1, 0.5)      # Hz
CARDIAC_BAND = (0.6, 4.0)   # Hz — wider than TI's 0.8-2.0 on BOTH ends:
                            # the survey's Part E argues the 120 BPM cap
                            # excludes the tachycardic casualty, and the
                            # Erlangen benchmark found a clinical resting
                            # subject at 46 BPM below a 0.8 Hz floor
                            # (GDN0010) — bradycardia is real too. Matches
                            # the (0.6, 4) Hz band the Erlangen-tuned
                            # literature converged on.


def bandpass(x: np.ndarray, fs: float, band: tuple[float, float],
             roll_bins: int = 2) -> np.ndarray:
    """Zero-phase FFT-domain band-pass with a raised-cosine edge of
    ``roll_bins`` bins to avoid ringing from a brick wall."""
    x = np.asarray(x, dtype=float)
    n = x.size
    X = np.fft.rfft(x - x.mean())
    f = np.fft.rfftfreq(n, 1.0 / fs)
    h = np.zeros_like(f)
    h[(f >= band[0]) & (f <= band[1])] = 1.0
    if roll_bins > 0:
        k = np.ones(2 * roll_bins + 1) / (2 * roll_bins + 1)
        h = np.convolve(h, k, mode="same")
    return np.fft.irfft(X * h, n)


def successive_diff(phase: np.ndarray) -> np.ndarray:
    """TI-chain stage (verified [prim]): kills DC and slow drift,
    pre-emphasises the cardiac band (+6 dB/octave)."""
    return np.diff(phase, prepend=phase[0])


def remove_impulses(x: np.ndarray, thresh: float = 1.5) -> np.ndarray:
    """TI-chain impulse removal (verified [prim]: forward/backward
    difference both beyond +-thresh -> replace by neighbour mean). Operates
    on the differenced phase in TI's chain; here on whatever it is given.
    NOTE (survey A2): never use across a *dropped frame* — spikes within
    received data only."""
    y = x.copy()
    b = y[1:-1] - y[:-2]
    fdiff = y[1:-1] - y[2:]
    bad = (np.abs(b) > thresh) & (np.abs(fdiff) > thresh) \
        & (np.sign(b) == np.sign(fdiff))
    idx = np.where(bad)[0] + 1
    y[idx] = 0.5 * (y[idx - 1] + y[idx + 1])
    return y


def harmonic_notch(x: np.ndarray, fs: float, f0: float,
                   harmonics: range = range(2, 6),
                   rel_bw: float = 0.04, rel_bw_k2: float = 0.08) -> np.ndarray:
    """Notch k*f0 for k in ``harmonics`` (survey B.8: respiration harmonics
    3..5 land on the cardiac band and are often stronger than the cardiac
    line; TI's chain only treats k=2 — we treat 2..5 by default)."""
    x = np.asarray(x, dtype=float)
    n = x.size
    X = np.fft.rfft(x)
    f = np.fft.rfftfreq(n, 1.0 / fs)
    h = np.ones_like(f)
    for k in harmonics:
        fk = k * f0
        # the 2nd harmonic is by far the strongest in-band competitor
        # (Erlangen benchmark: a 0.6 Hz band floor put it in-band and a
        # narrow notch let it win on some subjects) — notch it wider
        bw = rel_bw_k2 if k == 2 else rel_bw
        h[np.abs(f - fk) <= max(bw * fk, 2.0 * fs / n)] = 0.0
    return np.fft.irfft(X * h, n)


def estimate_then_notch(phase: np.ndarray, fs: float,
                        resp_hz: float | None = None) -> np.ndarray:
    """The B.8 cascade: band-limit to cardiac after notching the measured
    respiration fundamental's harmonics. If ``resp_hz`` is None it is
    estimated from the respiration band first (FFT peak)."""
    from .estimators import fft_peak
    if resp_hz is None:
        resp_hz = fft_peak(phase, fs, RESP_BAND).rate_hz
    y = harmonic_notch(phase, fs, resp_hz)
    return bandpass(y, fs, CARDIAC_BAND)
