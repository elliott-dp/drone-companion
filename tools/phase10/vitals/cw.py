"""CW-radar I/Q conditioning: ellipse-fit imbalance compensation followed
by arctangent demodulation — the recommended preprocessing for the
Erlangen Six-Port datasets (their papers point to Singh et al., TMTT 2013:
correct offset, gain and phase errors before demodulation).

Ellipse fit: Fitzgibbon-style algebraic least squares (direct ellipse fit)
on the I/Q scatter; the correction recentres, de-tilts and rescales the
locus to a circle. NumPy-only.
"""

from __future__ import annotations

import numpy as np


def fit_ellipse(x: np.ndarray, y: np.ndarray):
    """Direct least-squares ellipse fit (Fitzgibbon/Halir-Flusser).
    Returns (cx, cy, a, b, theta)."""
    x = np.asarray(x, dtype=float)
    y = np.asarray(y, dtype=float)
    xm, ym = x.mean(), y.mean()
    xs, ys = x - xm, y - ym
    D1 = np.column_stack([xs ** 2, xs * ys, ys ** 2])
    D2 = np.column_stack([xs, ys, np.ones_like(xs)])
    S1 = D1.T @ D1
    S2 = D1.T @ D2
    S3 = D2.T @ D2
    T = -np.linalg.solve(S3, S2.T)
    M = S1 + S2 @ T
    C = np.array([[0, 0, 2.0], [0, -1.0, 0], [2.0, 0, 0]])
    Mred = np.linalg.solve(C, M)
    w, V = np.linalg.eig(Mred)
    # pick the eigenvector satisfying the ellipse constraint 4ac - b^2 > 0
    cond = 4 * V[0] * V[2] - V[1] ** 2
    a1 = V[:, np.where(cond > 0)[0][0]].real
    a2 = T @ a1
    A, B, Cc, D, E, F = a1[0], a1[1], a1[2], a2[0], a2[1], a2[2]
    # centre (in shifted frame)
    den = B ** 2 - 4 * A * Cc
    cx = (2 * Cc * D - B * E) / den
    cy = (2 * A * E - B * D) / den
    theta = 0.5 * np.arctan2(B, A - Cc)
    # axis lengths
    num = 2 * (A * E ** 2 + Cc * D ** 2 + F * B ** 2 - B * D * E
               - 4 * A * Cc * F) / -den
    ct, st = np.cos(theta), np.sin(theta)
    ap = A * ct ** 2 + B * ct * st + Cc * st ** 2
    cp = A * st ** 2 - B * ct * st + Cc * ct ** 2
    a_ax = np.sqrt(abs(num / (2 * ap)))
    b_ax = np.sqrt(abs(num / (2 * cp)))
    return cx + xm, cy + ym, a_ax, b_ax, theta


def iq_correct(i: np.ndarray, q: np.ndarray) -> np.ndarray:
    """Ellipse-fit compensation -> complex circular locus."""
    cx, cy, a_ax, b_ax, th = fit_ellipse(i, q)
    ct, st = np.cos(th), np.sin(th)
    xr = (i - cx) * ct + (q - cy) * st
    yr = -(i - cx) * st + (q - cy) * ct
    if a_ax > 0 and b_ax > 0:
        xr = xr / a_ax
        yr = yr / b_ax
    return xr + 1j * yr


def cw_phase(i: np.ndarray, q: np.ndarray) -> np.ndarray:
    """Ellipse-corrected arctangent phase, unwrapped [rad]."""
    z = iq_correct(i, q)
    return np.unwrap(np.angle(z))
