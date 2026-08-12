"""Tests E7/E8: three-way frame-ledger reconciliation and radar↔CC drift.

Implements the reconciliation rules from
``docs/phase10/radar_transport_and_sync.md`` §D.4, and nothing more:

1. The **HTE edge count is the authoritative frame count** — an edge means the
   frame fired, whatever happened to its data afterwards.
2. ``*_idx.bin`` entries are matched to edges in monotonic order; an edge with
   no entry is a **capture drop**; an entry with no edge is a **missed edge**
   (a *timestamping* failure, which is a different fault and must not be
   reported as a capture drop).
3. Per-device count disagreement is a **de-alignment fault** — surfaced by
   :mod:`tools.phase10.bench.capture`, never repaired here.
4. Edge times versus TDA timestamps yield the **offset and drift (ppm)** per
   dwell, with the fit residual reported and a Locked/Degraded/Unlocked quality
   state on the same discipline ``cc-timesync`` already uses.
5. **Never interpolate a missing frame.**

Matching method. Both series are near-periodic at the same configured cadence
``T`` but live in different clocks with a ppm-level relative rate. Over a 30 s
dwell at 20 Hz, 10 ppm is 300 µs — 0.6 % of a 50 ms period — so assigning each
timestamp to an integer *slot* ``round((t - t0) / T)`` is robust, and matching
reduces to integer set intersection under the single unknown slot shift between
the two series. That shift is found by maximising overlap, which is exact for
any drop pattern that leaves the two series sharing a majority of slots.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

# Quality thresholds on the affine-fit residual, tied to the project's own
# timing budget: 159 µs buys −20 dB cancellation of a 100 Hz line, and 250 µs
# holds the translational residual under 50 µm at 0.2 m/s hover velocity.
LOCKED_RESIDUAL_NS = 50_000
DEGRADED_RESIDUAL_NS = 250_000


@dataclass
class DriftFit:
    ppm: float
    offset_ns: float
    residual_rms_ns: float
    n_points: int
    n_trimmed: int

    @property
    def quality(self) -> str:
        if self.n_points < 8:
            return "UNLOCKED"
        if self.residual_rms_ns <= LOCKED_RESIDUAL_NS:
            return "LOCKED"
        if self.residual_rms_ns <= DEGRADED_RESIDUAL_NS:
            return "DEGRADED"
        return "UNLOCKED"


@dataclass
class Reconciliation:
    n_edges: int
    n_idx: int
    slot_shift: int
    matched: list[tuple[int, int]] = field(default_factory=list)   # (edge i, idx j)
    capture_drops: list[int] = field(default_factory=list)          # edge indices
    missed_edges: list[int] = field(default_factory=list)           # idx indices
    drift: DriftFit | None = None
    live_matched: int = 0
    live_unmatched: int = 0

    @property
    def coherence(self) -> str:
        """Dataset-level verdict for the dwell."""
        if not self.matched:
            return "broken"
        if self.missed_edges:
            return "estimated"      # the µs time base has holes
        if self.drift is not None and self.drift.quality == "UNLOCKED":
            return "estimated"
        return "hardware"

    def summary(self) -> str:
        lines = [
            f"edges {self.n_edges}  idx {self.n_idx}  matched {len(self.matched)}"
            f"  slot_shift {self.slot_shift}",
            f"capture drops {len(self.capture_drops)}  missed edges {len(self.missed_edges)}",
        ]
        if self.drift:
            d = self.drift
            lines.append(
                f"drift {d.ppm:+.3f} ppm  offset {d.offset_ns/1e6:.3f} ms  "
                f"residual {d.residual_rms_ns:.0f} ns rms (n={d.n_points}, "
                f"trimmed {d.n_trimmed}) → {d.quality}"
            )
        if self.live_matched or self.live_unmatched:
            lines.append(
                f"live tier: matched {self.live_matched}, unmatched {self.live_unmatched}"
            )
        lines.append(f"coherence = {self.coherence}")
        return "\n".join(lines)


def _slots(t: np.ndarray, period_ns: float) -> np.ndarray:
    t = np.asarray(t, dtype=np.float64)
    return np.rint((t - t[0]) / period_ns).astype(np.int64)


def _best_shift(a: np.ndarray, b: np.ndarray, search: int) -> int:
    """Integer shift s maximising |a ∩ (b + s)|."""
    sa = set(a.tolist())
    best_s, best_n = 0, -1
    for s in range(-search, search + 1):
        n = sum(1 for v in b if (v + s) in sa)
        if n > best_n:
            best_s, best_n = s, n
    return best_s


def fit_drift(
    tda_ns: np.ndarray, cc_ns: np.ndarray, *, trim_sigma: float = 3.0
) -> DriftFit:
    """Least-squares affine fit ``cc ≈ a·tda + b`` with one 3σ trim pass.

    Reported as ppm = (a − 1)·1e6. The trim exists because a single mis-matched
    pair would otherwise dominate the slope; it is a *reported* count, not a
    silent cleanup.
    """
    x = np.asarray(tda_ns, dtype=np.float64)
    y = np.asarray(cc_ns, dtype=np.float64)
    n0 = x.size
    if n0 < 2:
        return DriftFit(float("nan"), float("nan"), float("nan"), n0, 0)

    # Centre for numerical conditioning: ns-scale values over a 30 s dwell.
    x0, y0 = x.mean(), y.mean()
    a, b = np.polyfit(x - x0, y - y0, 1)
    resid = (y - y0) - (a * (x - x0) + b)
    keep = np.ones_like(resid, dtype=bool)
    if n0 >= 8:
        s = resid.std()
        if s > 0:
            keep = np.abs(resid) <= trim_sigma * s
            if keep.sum() >= max(4, int(0.5 * n0)):
                a, b = np.polyfit((x - x0)[keep], (y - y0)[keep], 1)
                resid = (y - y0)[keep] - (a * (x - x0)[keep] + b)
            else:
                keep = np.ones_like(resid, dtype=bool)

    rms = float(np.sqrt(np.mean(resid**2)))
    offset = float(y0 + b - a * x0)
    return DriftFit(
        ppm=float((a - 1.0) * 1e6),
        offset_ns=offset,
        residual_rms_ns=rms,
        n_points=int(keep.sum()),
        n_trimmed=int(n0 - keep.sum()),
    )


def reconcile(
    edges_ns: np.ndarray,
    idx_ts_ns: np.ndarray,
    nominal_period_ns: int,
    *,
    live_ns: np.ndarray | None = None,
    live_tol_ns: int | None = None,
) -> Reconciliation:
    edges = np.asarray(edges_ns, dtype=np.int64)
    idx = np.asarray(idx_ts_ns, dtype=np.int64)
    rec = Reconciliation(n_edges=edges.size, n_idx=idx.size, slot_shift=0)
    if edges.size == 0 or idx.size == 0:
        return rec

    se = _slots(edges, nominal_period_ns)
    sx = _slots(idx, nominal_period_ns)
    search = max(4, int(max(se[-1], sx[-1])) + 1)
    shift = _best_shift(se, sx, search)
    rec.slot_shift = shift

    edge_by_slot = {int(s): i for i, s in enumerate(se)}
    idx_by_slot = {int(s) + shift: j for j, s in enumerate(sx)}

    for s, i in sorted(edge_by_slot.items()):
        j = idx_by_slot.get(s)
        if j is None:
            rec.capture_drops.append(i)
        else:
            rec.matched.append((i, j))
    for s, j in sorted(idx_by_slot.items()):
        if s not in edge_by_slot:
            rec.missed_edges.append(j)

    if len(rec.matched) >= 2:
        mi = np.array([m[0] for m in rec.matched])
        mj = np.array([m[1] for m in rec.matched])
        rec.drift = fit_drift(idx[mj], edges[mi])

    if live_ns is not None and rec.drift is not None:
        tol = live_tol_ns if live_tol_ns is not None else nominal_period_ns // 2
        live = np.asarray(live_ns, dtype=np.int64)
        matched_edge_times = edges[[m[0] for m in rec.matched]]
        for t in live:
            d = np.min(np.abs(matched_edge_times - t)) if matched_edge_times.size else tol + 1
            if d <= tol:
                rec.live_matched += 1
            else:
                rec.live_unmatched += 1

    return rec
