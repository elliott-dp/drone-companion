"""The rate-estimator bank (survey B.9 made executable).

Every estimator returns an ``Estimate`` with a rate, a confidence in [0,1],
and its name — because the survey's core doctrine is *run several and fuse
with an explicit confidence*, and cross-estimator agreement is itself the
most useful confidence feature. NumPy-only.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .dsp import bandpass


@dataclass
class Estimate:
    rate_hz: float
    confidence: float
    name: str

    @property
    def rate_per_min(self) -> float:
        return self.rate_hz * 60.0


def _band_confidence(power: np.ndarray, freqs: np.ndarray,
                     band: tuple[float, float], peak_idx: int,
                     halfwidth_hz: float = 0.1) -> float:
    """TI-style confidence (verified [prim]): energy in a +-halfwidth
    window around the peak over the remaining band energy, mapped to
    [0, 1) as r/(1+r)."""
    sel = (freqs >= band[0]) & (freqs <= band[1])
    peak_f = freqs[peak_idx]
    win = sel & (np.abs(freqs - peak_f) <= halfwidth_hz)
    p_peak = power[win].sum()
    p_rest = power[sel].sum() - p_peak
    if p_rest <= 0:
        return 1.0
    r = p_peak / p_rest
    return float(r / (1.0 + r))


def fft_peak(x: np.ndarray, fs: float, band: tuple[float, float],
             zoom: int = 8) -> Estimate:
    """Windowed, zero-padded FFT peak with parabolic interpolation."""
    x = np.asarray(x, dtype=float)
    n = x.size
    w = np.hanning(n)
    nfft = int(2 ** np.ceil(np.log2(n * zoom)))
    X = np.abs(np.fft.rfft((x - x.mean()) * w, nfft)) ** 2
    f = np.fft.rfftfreq(nfft, 1.0 / fs)
    sel = np.where((f >= band[0]) & (f <= band[1]))[0]
    if sel.size == 0:
        return Estimate(0.0, 0.0, "fft")
    i = sel[np.argmax(X[sel])]
    # parabolic refinement
    if 0 < i < f.size - 1 and X[i] > 0:
        a, b, c = np.log(X[i - 1] + 1e-30), np.log(X[i] + 1e-30), \
            np.log(X[i + 1] + 1e-30)
        d = 0.5 * (a - c) / (a - 2 * b + c + 1e-30)
        d = np.clip(d, -0.5, 0.5)
    else:
        d = 0.0
    rate = f[i] + d * (f[1] - f[0])
    return Estimate(float(rate), _band_confidence(X, f, band, i), "fft")


def autocorr(x: np.ndarray, fs: float, band: tuple[float, float]) -> Estimate:
    """Autocorrelation peak in the lag window implied by ``band``
    (robust to spectral leakage; ambiguous at harmonics — fusion's job)."""
    x = bandpass(np.asarray(x, dtype=float), fs, band)
    n = x.size
    ac = np.correlate(x, x, mode="full")[n - 1:]
    ac /= ac[0] + 1e-30
    lo = max(2, int(fs / band[1]))
    hi = min(n - 2, int(fs / band[0]))
    if hi <= lo:
        return Estimate(0.0, 0.0, "autocorr")
    seg = ac[lo:hi]
    i = int(np.argmax(seg)) + lo
    # parabolic refinement on the correlation peak
    a, b, c = ac[i - 1], ac[i], ac[i + 1]
    d = 0.5 * (a - c) / (a - 2 * b + c + 1e-30)
    d = float(np.clip(d, -0.5, 0.5))
    lag = i + d
    conf = float(np.clip(ac[i], 0.0, 1.0))
    return Estimate(float(fs / lag), conf, "autocorr")


def peak_intervals(x: np.ndarray, fs: float,
                   band: tuple[float, float]) -> Estimate:
    """Median inter-peak interval in the band-passed signal (gives
    beat-to-beat structure; sensitive to impulse noise — run after
    remove_impulses)."""
    y = bandpass(np.asarray(x, dtype=float), fs, band)
    min_dist = int(fs / band[1])
    idx = []
    last = -min_dist
    for i in range(1, y.size - 1):
        if y[i] > y[i - 1] and y[i] >= y[i + 1] and y[i] > 0:
            if i - last >= min_dist:
                idx.append(i)
                last = i
            elif idx and y[i] > y[idx[-1]]:
                idx[-1] = i
                last = i
    if len(idx) < 3:
        return Estimate(0.0, 0.0, "intervals")
    iv = np.diff(idx) / fs
    iv = iv[(iv >= 1.0 / band[1]) & (iv <= 1.0 / band[0])]
    if iv.size < 2:
        return Estimate(0.0, 0.0, "intervals")
    rate = 1.0 / np.median(iv)
    cv = iv.std() / iv.mean()
    return Estimate(float(rate), float(np.clip(1.0 - cv, 0.0, 1.0)),
                    "intervals")


def _hankel_covariance(x: np.ndarray, m: int) -> np.ndarray:
    """Forward-backward averaged covariance from Hankel snapshots of
    length m (standard single-channel MUSIC construction)."""
    n = x.size
    L = n - m + 1
    H = np.lib.stride_tricks.sliding_window_view(x, m)[:L]
    R = (H.conj().T @ H) / L
    J = np.eye(m)[::-1]
    return 0.5 * (R + J @ R.conj() @ J)


def music(x: np.ndarray, fs: float, band: tuple[float, float],
          n_sources: int = 2, m: int | None = None,
          grid_hz: float = 0.005) -> Estimate:
    """Single-channel MUSIC on the analytic band-passed signal.

    Decimates so the m-tap covariance window spans multiple periods of the
    band's lowest frequency — without this, low-band (respiration) MUSIC
    has a sub-period aperture and estimates garbage (found by the
    self-test, where it then dragged the fusion low)."""
    y = bandpass(np.asarray(x, dtype=float), fs, band)
    dec = max(1, int(fs / (4.0 * band[1])))
    if dec > 1:
        y = y[::dec]
        fs = fs / dec
    # analytic signal via FFT one-siding
    Y = np.fft.fft(y)
    h = np.zeros(y.size)
    h[0] = 1.0
    h[1:(y.size + 1) // 2] = 2.0
    z = np.fft.ifft(Y * h)
    m = m or min(64, y.size // 3)
    R = _hankel_covariance(z, m)
    w, V = np.linalg.eigh(R)
    En = V[:, : m - n_sources]                     # noise subspace
    f_grid = np.arange(band[0], band[1], grid_hz)
    k = np.arange(m)
    A = np.exp(2j * np.pi * f_grid[None, :] * k[:, None] / fs)
    denom = np.linalg.norm(En.conj().T @ A, axis=0) ** 2
    P = 1.0 / (denom + 1e-30)
    i = int(np.argmax(P))
    conf = _band_confidence(P, f_grid, band, i, halfwidth_hz=0.1)
    return Estimate(float(f_grid[i]), conf, "music")


def fuse(estimates: list[Estimate], tol_hz: float = 0.03) -> Estimate:
    """Cross-estimator fusion (survey B.9): find the largest cluster of
    estimates agreeing within ``tol_hz``, confidence-weight its mean, and
    make the agreement fraction part of the fused confidence.

    ``tol_hz`` must be band-appropriate: the default 0.03 Hz suits the
    respiration band; use ~0.08-0.1 Hz for cardiac. A loose tolerance lets
    one bad estimator drag the cluster mean (self-test finding: a wrong
    MUSIC respiration estimate 0.075 Hz off still joined a 0.1 Hz cluster
    and biased the fundamental — which then mislocated every harmonic
    notch)."""
    valid = [e for e in estimates if e.rate_hz > 0 and e.confidence > 0]
    if not valid:
        return Estimate(0.0, 0.0, "fused")
    best_cluster: list[Estimate] = []
    for anchor in valid:
        cluster = [e for e in valid
                   if abs(e.rate_hz - anchor.rate_hz) <= tol_hz]
        if (len(cluster), sum(e.confidence for e in cluster)) > \
                (len(best_cluster), sum(e.confidence for e in best_cluster)):
            best_cluster = cluster
    wsum = sum(e.confidence for e in best_cluster)
    rate = sum(e.rate_hz * e.confidence for e in best_cluster) / wsum
    agreement = len(best_cluster) / len(valid)
    conf = agreement * (wsum / len(best_cluster))
    return Estimate(float(rate), float(conf), "fused")
