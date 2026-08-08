"""Scene geometries and the GDOP analysis for the anchor least-squares solve.

Experiment 2 (radar_rbec_method.md Part C.3): the per-frame translation solve
d_hat = argmin sum w_k (phi_k - K uk.d)^2 has covariance
(lambda/4pi)^2 (U^T W U)^-1 — a GDOP problem (Langley 1999). What matters
operationally is the projection onto the target LOS:
sigma_pred = sigma_phi * (lambda/4pi) * sqrt(u_t^T (U^T W U)^-1 u_t).

We also compute the common-mode rejection factor of the solve+differencing:
a phase offset eps common to all anchors leaks into d_hat and then into the
target prediction as eps * u_t^T (U^T W U)^-1 U^T W 1; the residual seen at
the target is eps * (1 - that). Bracketing anchors around the target should
drive this toward zero (paper §C.3 rule 2/§E).
"""

from __future__ import annotations

import numpy as np

from .core import los_from_azel


def scene_ground_ring(h: float, ground_ranges: np.ndarray,
                      azimuths: np.ndarray) -> np.ndarray:
    """Anchors on a flat ground plane seen from hover height ``h``:
    LOS unit vectors for each (ground range, azimuth) pair, radar at origin,
    x forward, z up. Returns (N,3)."""
    us = []
    for r in ground_ranges:
        el = -np.arctan2(h, r)                     # depression
        for az in azimuths:
            us.append(los_from_azel(az, el))
    return np.asarray(us)


def scene_sector(n: int, az_span: float, el_lo: float, el_hi: float,
                 rng: np.random.Generator) -> np.ndarray:
    """N anchors uniform over an azimuth span (centred on 0) and an
    elevation interval."""
    az = rng.uniform(-az_span / 2, az_span / 2, n)
    el = rng.uniform(el_lo, el_hi, n)
    return np.array([los_from_azel(a, e) for a, e in zip(az, el)])


def dop_matrix(U: np.ndarray, w: np.ndarray | None = None) -> np.ndarray:
    """(U^T W U)^-1 with unit weights by default. Raises LinAlgError when the
    LOS directions span rank < 3 (e.g. collapsed azimuth spread) — note this
    is the actual singularity condition for the translation-only solve;
    coplanar LOS *tips* off the origin (one shared depression angle) are
    ill-conditioned but NOT singular (review finding). np.linalg.inv alone
    does not reliably raise on numerically singular input, hence the
    explicit rank check."""
    if w is None:
        w = np.ones(U.shape[0])
    A = (U * w[:, None]).T @ U
    if np.linalg.matrix_rank(U, tol=1e-9) < U.shape[1]:
        raise np.linalg.LinAlgError("anchor LOS directions span rank < 3")
    return np.linalg.inv(A)


def axis_dops(U: np.ndarray, w: np.ndarray | None = None) -> np.ndarray:
    """Per-axis DOPs sqrt(diag((U^T W U)^-1)) in the given frame."""
    return np.sqrt(np.diag(dop_matrix(U, w)))


def target_dop(U: np.ndarray, u_t: np.ndarray,
               w: np.ndarray | None = None) -> float:
    """DOP of the prediction along the target LOS:
    sqrt(u_t^T (U^T W U)^-1 u_t)."""
    D = dop_matrix(U, w)
    return float(np.sqrt(u_t @ D @ u_t))


def common_mode_rejection(U: np.ndarray, u_t: np.ndarray,
                          w: np.ndarray | None = None) -> float:
    """Fraction of a common anchor-phase offset that SURVIVES at the target
    after solve+differencing: |1 - u_t^T (U^T W U)^-1 U^T W 1|.
    0 = perfect implicit cancellation; 1 = no cancellation."""
    if w is None:
        w = np.ones(U.shape[0])
    D = dop_matrix(U, w)
    g = D @ (U * w[:, None]).T @ np.ones(U.shape[0])
    return float(abs(1.0 - u_t @ g))


def condition_number(U: np.ndarray) -> float:
    return float(np.linalg.cond(U.T @ U))
