"""Test E2/E4/E5: drop-rate and cross-device analysis of a TDA2 capture.

A capture is four per-device ``*_data.bin`` + ``*_idx.bin`` pairs. This module
answers, from the index files alone (no bulk read):

* **E2** — how many frames were dropped, where, and at what rate, per capture
  mode and frame periodicity.
* **E4** — whether the per-record ``flags`` field encodes drop status
  (see :func:`tools.phase10.bench.idx.flags_histogram`).
* **E5** — whether the four devices' timestamps come from one TDA2 clock
  (alignment is then a lookup) or from several (it becomes an estimation
  problem).

Method note, deliberately conservative: a gap is inferred from a timestamp
delta, because the index carries **no frame sequence number**. A delta of
``k * T`` within tolerance is read as ``k - 1`` lost frames. Deltas that are not
near-integer multiples of the nominal period are reported as *anomalies* rather
than silently rounded — an irregular cadence is a different (and worse) finding
than a dropped frame, and conflating them would hide it.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from .idx import IdxFile, parse_idx


@dataclass
class GapEvent:
    after_record: int          # index of the record preceding the gap
    delta_ns: int
    lost_frames: int           # k-1, from round(delta / T)
    integer_error: float       # |delta/T - round(delta/T)|, 0 == perfectly periodic


@dataclass
class DeviceReport:
    name: str
    n_records: int
    declared_num_idx: int
    duration_ns: int
    median_period_ns: float
    gaps: list[GapEvent] = field(default_factory=list)
    anomalies: list[GapEvent] = field(default_factory=list)
    size_declared: int = 0
    size_summed: int = 0

    @property
    def lost_frames(self) -> int:
        return sum(g.lost_frames for g in self.gaps)

    @property
    def expected_frames(self) -> int:
        """Frames the cadence implies over the observed span (inclusive)."""
        if self.n_records < 2 or self.median_period_ns <= 0:
            return self.n_records
        return int(round(self.duration_ns / self.median_period_ns)) + 1

    @property
    def drop_rate(self) -> float:
        exp = self.expected_frames
        return 0.0 if exp <= 0 else (exp - self.n_records) / exp

    @property
    def last_frame_lost(self) -> bool:
        """The known TDA2 signature: header claims more frames than exist."""
        return self.declared_num_idx > self.n_records


@dataclass
class CaptureReport:
    directory: Path
    nominal_period_ns: int
    devices: list[DeviceReport]
    cross_device_max_skew_ns: int | None
    one_clock: bool | None

    @property
    def worst_drop_rate(self) -> float:
        return max((d.drop_rate for d in self.devices), default=0.0)

    @property
    def device_count_disagreement(self) -> int:
        counts = [d.n_records for d in self.devices]
        return (max(counts) - min(counts)) if counts else 0

    def summary(self) -> str:
        lines = [
            f"capture {self.directory}  nominal period {self.nominal_period_ns/1e6:.3f} ms",
            f"{'device':<14}{'recs':>7}{'decl':>7}{'lost':>6}{'drop%':>8}"
            f"{'median T ms':>13}{'anom':>6}{'lastlost':>10}",
        ]
        for d in self.devices:
            lines.append(
                f"{d.name:<14}{d.n_records:>7}{d.declared_num_idx:>7}{d.lost_frames:>6}"
                f"{100*d.drop_rate:>8.3f}{d.median_period_ns/1e6:>13.4f}"
                f"{len(d.anomalies):>6}{str(d.last_frame_lost):>10}"
            )
        if self.cross_device_max_skew_ns is not None:
            lines.append(
                f"cross-device max |Δt| per frame: {self.cross_device_max_skew_ns} ns"
                f"  → one_clock={self.one_clock}"
            )
        lines.append(f"device count disagreement: {self.device_count_disagreement}")
        return "\n".join(lines)


def analyse_device(
    idx: IdxFile,
    nominal_period_ns: int,
    *,
    integer_tol: float = 0.15,
    gap_factor: float = 1.5,
) -> DeviceReport:
    ts = idx.timestamps_ns
    name = idx.path.name
    if ts.size == 0:
        return DeviceReport(name, 0, idx.header.num_idx, 0, 0.0)

    deltas = np.diff(ts)
    median_T = float(np.median(deltas)) if deltas.size else float(nominal_period_ns)
    # Use the *nominal* period for gap arithmetic (the configured cadence is the
    # authority); the measured median is reported so a wrong nominal is obvious.
    T = float(nominal_period_ns)

    gaps: list[GapEvent] = []
    anomalies: list[GapEvent] = []
    for i, d in enumerate(deltas):
        if d <= gap_factor * T:
            continue
        k = d / T
        k_round = round(k)
        err = abs(k - k_round)
        ev = GapEvent(int(i), int(d), max(int(k_round) - 1, 0), float(err))
        (gaps if err <= integer_tol else anomalies).append(ev)

    declared, present = idx.declared_vs_present
    size_declared, size_summed = idx.size_accounting
    return DeviceReport(
        name=name,
        n_records=present,
        declared_num_idx=declared,
        duration_ns=int(ts[-1] - ts[0]),
        median_period_ns=median_T,
        gaps=gaps,
        anomalies=anomalies,
        size_declared=size_declared,
        size_summed=size_summed,
    )


def _device_sort_key(p: Path) -> tuple:
    m = re.search(r"(\d+)", p.stem)
    return (int(m.group(1)) if m else 0, p.name)


def analyse_capture(
    directory: str | Path,
    nominal_period_ns: int,
    *,
    pattern: str = "*_idx.bin",
    one_clock_tol_ns: int = 1_000,
) -> CaptureReport:
    directory = Path(directory)
    paths = sorted(directory.glob(pattern), key=_device_sort_key)
    if not paths:
        raise FileNotFoundError(f"no {pattern} under {directory}")
    idxs = [parse_idx(p) for p in paths]
    devices = [analyse_device(i, nominal_period_ns) for i in idxs]

    skew: int | None = None
    one_clock: bool | None = None
    if len(idxs) > 1:
        n = min(i.n_records for i in idxs)
        if n > 0:
            stack = np.stack([i.timestamps_ns[:n] for i in idxs])
            # Compare like-indexed frames; if one clock feeds all devices the
            # per-frame spread is ns-class, not ms-class.
            spread = stack.max(axis=0) - stack.min(axis=0)
            skew = int(spread.max())
            one_clock = bool(skew <= one_clock_tol_ns)

    return CaptureReport(directory, int(nominal_period_ns), devices, skew, one_clock)


def sweep_table(reports: dict[str, CaptureReport]) -> str:
    """E2's deliverable: drop rate versus mode/periodicity, one row per capture."""
    lines = [f"{'capture':<28}{'T ms':>9}{'recs':>8}{'lost':>7}{'drop%':>9}{'anom':>6}"]
    for label, r in reports.items():
        recs = sum(d.n_records for d in r.devices)
        lost = sum(d.lost_frames for d in r.devices)
        anom = sum(len(d.anomalies) for d in r.devices)
        lines.append(
            f"{label:<28}{r.nominal_period_ns/1e6:>9.3f}{recs:>8}{lost:>7}"
            f"{100*r.worst_drop_rate:>9.3f}{anom:>6}"
        )
    return "\n".join(lines)
