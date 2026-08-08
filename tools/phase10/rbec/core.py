"""Shared constants and helpers for the RBEC numerical validation stack.

Conventions
-----------
* Wavelength at 79 GHz centre; phase-displacement scale 4*pi/lambda (monostatic).
* All angles in radians internally; degrees only at the CLI/report boundary.
* Every experiment takes an explicit seed; results must be bit-reproducible.
* Vital bands per the survey: respiration 0.1-0.5 Hz, cardiac 0.8-3.0 Hz.
"""

from __future__ import annotations

import numpy as np

C0 = 2.998e8                      # m/s
F_CENTRE = 79.0e9                 # Hz
LAMBDA = C0 / F_CENTRE            # 3.794e-3 m
K_DISP = 4.0 * np.pi / LAMBDA     # rad per metre of LOS displacement (two-way)

RESP_BAND = (0.1, 0.5)            # Hz
CARDIAC_BAND = (0.8, 3.0)         # Hz

# Budget targets (radar_rbec_method.md Part F.1): residual *in band*.
CARDIAC_AMP_M = 0.1e-3            # 0.1 mm chest displacement
CARDIAC_AMP_RAD = K_DISP * CARDIAC_AMP_M          # ~0.331 rad
CARDIAC_RESIDUAL_BUDGET_RAD = CARDIAC_AMP_RAD / 3.0
RESP_AMP_M = 4.0e-3               # nominal 4 mm respiration used in sims


def band_rms(x: np.ndarray, fs: float, band: tuple[float, float]) -> float:
    """RMS of ``x`` restricted to ``band`` (Hz), via the periodogram.

    Parseval-consistent: band_rms over (0, fs/2) equals np.std(x) for a
    zero-mean signal (single-sided PSD, DC and Nyquist bins excluded from
    doubling).
    """
    x = np.asarray(x, dtype=float)
    n = x.size
    xf = np.fft.rfft(x - x.mean())
    freqs = np.fft.rfftfreq(n, d=1.0 / fs)
    # single-sided power per bin
    p = (np.abs(xf) ** 2) / n**2
    p[1:] *= 2.0
    if n % 2 == 0:
        p[-1] /= 2.0
    sel = (freqs >= band[0]) & (freqs <= band[1])
    return float(np.sqrt(p[sel].sum()))


def shaped_noise(n: int, fs: float, f_knee: float, rms: float,
                 rng: np.random.Generator) -> np.ndarray:
    """Low-frequency-dominated Gaussian noise with a 1/(1+(f/f_knee)^2)
    amplitude shaping (2nd-order roll-off in power), scaled to ``rms``.

    Used for hover sway: energy concentrated below ~f_knee, matching the
    'position variations up to ~20 cm amplitude, slow' character of the
    published hover measurements. This is a synthetic stand-in, not a
    measured PSD, and the results document says so.
    """
    white = rng.standard_normal(n)
    xf = np.fft.rfft(white)
    f = np.fft.rfftfreq(n, d=1.0 / fs)
    shape = 1.0 / (1.0 + (f / f_knee) ** 2)
    shape[0] = 0.0                      # zero-mean, no DC drift term
    x = np.fft.irfft(xf * shape, n)
    s = x.std()
    if s > 0:
        x *= rms / s
    return x


def unit(v: np.ndarray) -> np.ndarray:
    v = np.asarray(v, dtype=float)
    return v / np.linalg.norm(v)


def los_from_azel(az: float, el: float) -> np.ndarray:
    """Unit LOS vector from azimuth (about +z, from +x) and elevation
    (positive above the x-y plane; ground anchors from a hovering radar
    have negative elevation = depression)."""
    ce = np.cos(el)
    return np.array([ce * np.cos(az), ce * np.sin(az), np.sin(el)])
