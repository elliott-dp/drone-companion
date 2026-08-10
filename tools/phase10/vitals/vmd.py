"""Variational Mode Decomposition (Dragomiretskiy & Zosso, IEEE TSP 62(3),
2014), implemented from the paper's ADMM update equations as transcribed in
the 2026-08 algorithm-spec scout.

Updates (all in the Fourier domain, omega >= 0):
  mode:    u_k <- (f - sum_{i!=k} u_i + lambda/2) / (1 + 2*alpha*(w - w_k)^2)
  centre:  w_k <- int w |u_k|^2 dw / int |u_k|^2 dw          (Eq. 30)
  lagr.:   lambda <- lambda + tau * (f - sum_k u_k)

Practice defaults follow the de-facto reference implementation and the
VMD-for-vitals literature: mirror extension by half the signal on each
side, K=5, alpha ~ 1000-2000, tau=0 for noisy data (paper III-C: drop the
multiplier when noise is present), tol 5e-6. K/alpha are a *declared
method parameter* here (survey B.8 caveat): the bank uses a fixed rule,
not per-recording tuning.
"""

from __future__ import annotations

import numpy as np


def vmd(x: np.ndarray, k: int = 5, alpha: float = 2000.0, tau: float = 0.0,
        tol: float = 5e-6, max_iter: int = 500, seed: int = 0
        ) -> tuple[np.ndarray, np.ndarray]:
    """Decompose ``x`` into ``k`` modes. Returns (modes (k, n), centre
    frequencies in cycles/sample (k,))."""
    x = np.asarray(x, dtype=float)
    n = x.size
    # mirror extension by n/2 on each side
    half = n // 2
    xe = np.concatenate([x[:half][::-1], x, x[-half:][::-1]])
    ne = xe.size
    f_hat = np.fft.fft(xe)
    # one-sided processing: keep full array, work on positive freqs
    w = np.fft.fftfreq(ne)                       # cycles/sample
    pos = w >= 0

    rng = np.random.default_rng(seed)
    # init centre frequencies: spread over (0, 0.5) — 'grid' scheme
    wk = (0.5 * (np.arange(k) + 0.5) / k) * np.ones(k)
    u_hat = np.zeros((k, ne), dtype=complex)
    lam = np.zeros(ne, dtype=complex)

    for _ in range(max_iter):
        u_prev = u_hat.copy()
        for i in range(k):
            others = u_hat.sum(axis=0) - u_hat[i]
            resid = f_hat - others + lam / 2.0
            u_hat[i] = resid / (1.0 + 2.0 * alpha * (np.abs(w) - wk[i]) ** 2)
            p = np.abs(u_hat[i][pos]) ** 2
            denom = p.sum()
            if denom > 0:
                wk[i] = float((w[pos] * p).sum() / denom)
        if tau > 0:
            lam = lam + tau * (f_hat - u_hat.sum(axis=0))
        num = np.abs(u_hat - u_prev) ** 2
        den = np.abs(u_prev) ** 2
        if den.sum() > 0 and num.sum() / den.sum() < tol:
            break

    order = np.argsort(wk)
    modes = np.real(np.fft.ifft(u_hat[order], axis=1))[:, half:half + n]
    return modes, wk[order]


def vmd_cardiac(x: np.ndarray, fs: float,
                band: tuple[float, float] = (0.8, 3.5),
                k: int = 5, alpha: float = 2000.0) -> np.ndarray:
    """Sum of the VMD modes whose centre frequencies land in ``band`` —
    the VMD route to a cardiac signal (survey B.8)."""
    modes, wk = vmd(x, k=k, alpha=alpha)
    f_hz = wk * fs
    sel = (f_hz >= band[0]) & (f_hz <= band[1])
    if not sel.any():
        return np.zeros_like(x)
    return modes[sel].sum(axis=0)
