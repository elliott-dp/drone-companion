"""Self-test: prove the bench analyses recover injected ground truth.

No hardware, no network. Every case builds a synthetic artefact with a *known*
answer and asserts the analysis finds it — so that on bench day a surprising
result implicates the radar, not the script.

Run from the repo root:

    python3 -m tools.phase10.bench.selftest
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import numpy as np

from .calstep import (
    fold_at_events,
    fold_at_period,
    min_detectable_step,
    page_hinkley,
    required_epochs,
)
from .capture import analyse_capture
from .idx import RECORD_DTYPE, flags_histogram, parse_idx, write_idx
from .ledger import reconcile
from ..rbec.core import K_DISP

FS = 20.0
T_NS = int(round(1e9 / FS))          # 50 ms
FRAME_BYTES = 3_145_728              # SCAN-12
RNG_SEED = 20260810


def _synth_capture(
    root: Path,
    n_frames: int,
    drop_at: dict[int, int],
    *,
    n_dev: int = 4,
    period_ns: int = T_NS,
    dev_skew_ns: int = 0,
    declared_extra: int = 0,
) -> Path:
    """Write a 4-device capture with `drop_at = {after_frame: n_lost}`."""
    slots = []
    s = 0
    for f in range(n_frames):
        slots.append(s)
        s += 1 + drop_at.get(f, 0)
    slots = np.array(slots, dtype=np.int64)
    for d in range(n_dev):
        ts = slots * period_ns + d * dev_skew_ns + 1_000_000_000
        write_idx(
            root / f"master_{d}_idx.bin" if d == 0 else root / f"slave{d}_{d}_idx.bin",
            ts,
            frame_bytes=FRAME_BYTES // n_dev,
            declared_num_idx=n_frames + declared_extra,
        )
    return root


def test_idx_roundtrip() -> None:
    assert RECORD_DTYPE.itemsize == 48
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "x_idx.bin"
        ts = np.arange(10, dtype=np.int64) * T_NS
        write_idx(p, ts, frame_bytes=1234, record_flags=np.full(10, 7, dtype=np.uint32))
        f = parse_idx(p)
        assert f.n_records == 10
        assert np.array_equal(f.timestamps_ns, ts)
        assert f.records["size"][0] == 1234
        assert f.size_accounting == (1234 * 10, 1234 * 10)
        assert flags_histogram(f) == {7: 10}
        # torn trailing record is truncated, not fatal
        raw = p.read_bytes()
        p.write_bytes(raw[:-17])
        assert parse_idx(p).n_records == 9
    print("  idx round-trip + torn-tail handling ....... ok")


def test_drop_detection() -> None:
    truth = {100: 1, 250: 3, 400: 1}       # 5 lost frames in 600 recorded
    with tempfile.TemporaryDirectory() as td:
        root = _synth_capture(Path(td), 600, truth, dev_skew_ns=0, declared_extra=1)
        rep = analyse_capture(root, T_NS)
    dev = rep.devices[0]
    assert dev.n_records == 600, dev.n_records
    assert dev.lost_frames == 5, dev.lost_frames
    assert [g.lost_frames for g in dev.gaps] == [1, 3, 1]
    assert not dev.anomalies, dev.anomalies
    assert dev.last_frame_lost is True          # declared_extra=1
    exp = 605
    assert dev.expected_frames == exp, dev.expected_frames
    assert abs(dev.drop_rate - 5 / exp) < 1e-12
    assert rep.one_clock is True and rep.cross_device_max_skew_ns == 0
    print(f"  drop detection: found 5/5, rate {100*dev.drop_rate:.3f}% .... ok")


def test_cross_device_skew() -> None:
    with tempfile.TemporaryDirectory() as td:
        root = _synth_capture(Path(td), 200, {}, dev_skew_ns=2_000_000)   # 2 ms apart
        rep = analyse_capture(root, T_NS)
    assert rep.cross_device_max_skew_ns == 6_000_000, rep.cross_device_max_skew_ns
    assert rep.one_clock is False
    print("  cross-device skew: 3 dev spacings detected  ok")


def test_irregular_cadence_is_not_a_drop() -> None:
    """A non-integer delta must be an anomaly, never rounded into a drop."""
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "m_0_idx.bin"
        ts = (np.arange(100, dtype=np.int64) * T_NS)
        ts[50:] += int(0.6 * T_NS)          # a 1.6 T delta: not a clean multiple
        write_idx(p, ts, frame_bytes=100)
        rep = analyse_capture(Path(td), T_NS)
    dev = rep.devices[0]
    assert dev.lost_frames == 0, dev.lost_frames
    assert len(dev.anomalies) == 1, dev.anomalies
    print("  irregular cadence flagged as anomaly ...... ok")


def test_ledger_recovers_drops_and_drift() -> None:
    rng = np.random.default_rng(RNG_SEED)
    n = 600
    ppm_true = 12.5
    slots = np.arange(n, dtype=np.int64)
    edges = 5_000_000_000 + slots * T_NS + rng.normal(0, 200, n).astype(np.int64)
    # TDA clock: different origin, +12.5 ppm rate, its own small jitter
    tda_all = (
        1_234_567_000
        + (slots * T_NS * (1.0 - ppm_true * 1e-6)).astype(np.int64)
        + rng.normal(0, 200, n).astype(np.int64)
    )
    lost = np.array([17, 118, 119, 400])          # capture drops
    keep = np.setdiff1d(slots, lost)
    rec = reconcile(edges, tda_all[keep], T_NS)

    assert rec.n_edges == n and rec.n_idx == n - lost.size
    assert sorted(rec.capture_drops) == lost.tolist(), rec.capture_drops
    assert rec.missed_edges == [], rec.missed_edges
    assert rec.drift is not None
    assert abs(rec.drift.ppm - ppm_true) < 0.5, rec.drift.ppm
    assert rec.drift.quality == "LOCKED", rec.drift.summary if False else rec.drift
    assert rec.coherence == "hardware"
    print(
        f"  ledger: 4/4 drops, drift {rec.drift.ppm:+.2f} ppm "
        f"(truth {ppm_true:+.1f}), resid {rec.drift.residual_rms_ns:.0f} ns  ok"
    )


def test_ledger_distinguishes_missed_edges() -> None:
    """An idx entry with no edge is a timestamping failure, not a capture drop."""
    n = 200
    slots = np.arange(n, dtype=np.int64)
    tda = 1_000_000_000 + slots * T_NS
    edges = 9_000_000_000 + slots * T_NS
    missing_edges = np.array([30, 31, 90])
    rec = reconcile(edges[np.setdiff1d(slots, missing_edges)], tda, T_NS)
    assert len(rec.missed_edges) == 3, rec.missed_edges
    assert rec.capture_drops == [], rec.capture_drops
    assert rec.coherence == "estimated"
    print("  ledger: missed edges kept distinct ........ ok")


def test_calstep_recovers_known_step() -> None:
    rng = np.random.default_rng(RNG_SEED + 1)
    dur_s = 300.0
    n = int(dur_s * FS)
    t = np.arange(n) / FS
    step_true = 0.02          # rad — well below the 0.33 rad cardiac signal
    sigma = 0.01              # per-sample phase noise

    phase = rng.normal(0, sigma, n).cumsum() * 0.02        # slow drift
    phase += 0.30 * np.sin(2 * np.pi * 0.25 * t)           # respiration-like
    phase += rng.normal(0, sigma, n)                       # white
    events = np.arange(1.0, dur_s, 1.0)                    # 1 Hz cal events
    for e in events:                                       # inject the steps
        phase[t >= e] += step_true

    res = fold_at_events(phase, FS, events, half_window=4)
    mds = min_detectable_step(res.sigma_diff_rad, res.n_epochs)
    assert res.detected, res.summary()
    assert abs(res.step_rad - step_true) < 4 * res.sem_rad, res.summary()
    assert mds < step_true, (mds, step_true)
    need = required_epochs(step_true, res.sigma_diff_rad)
    print(
        f"  cal step: recovered {res.step_rad:+.5f} rad "
        f"({res.step_um:+.2f} um) vs truth {step_true:+.3f}; "
        f"MDS {mds:.5f} rad, needs {need} epochs .. ok"
    )


def test_calstep_controls_false_alarms() -> None:
    """With no step injected, the estimator must not claim one."""
    fires = 0
    trials = 20
    for k in range(trials):
        rng = np.random.default_rng(1000 + k)
        n = int(120 * FS)
        t = np.arange(n) / FS
        phase = 0.30 * np.sin(2 * np.pi * 0.25 * t) + rng.normal(0, 0.01, n)
        res = fold_at_events(phase, FS, np.arange(1.0, 120.0, 1.0), half_window=4)
        fires += int(res.detected)
    # alpha = 0.01 two-sided; allow a little slack for 20 trials
    assert fires <= 2, fires
    print(f"  cal step: false alarms {fires}/{trials} at alpha=0.01 . ok")


def test_blind_fold_is_confusable() -> None:
    """Document the trap: a bench vibration at the cal period folds identically."""
    rng = np.random.default_rng(RNG_SEED + 2)
    n = int(300 * FS)
    t = np.arange(n) / FS
    # No calibration step at all — just a 1 Hz mechanical artefact.
    phase = 0.05 * np.sin(2 * np.pi * 1.0 * t) + rng.normal(0, 0.005, n)
    blind = fold_at_period(phase, FS, 1.0, half_window=4)
    assert blind.detected, "a 1 Hz artefact should fool the blind fold — that is the point"
    print("  blind fold at 1 Hz is fooled by vibration . ok (documented trap)")


def test_page_hinkley_locates_unmodelled_step() -> None:
    rng = np.random.default_rng(RNG_SEED + 3)
    n = 4000
    x = rng.normal(0, 0.01, n)
    x[2000:] += 0.05                      # a single unmodelled level shift
    hits = page_hinkley(np.diff(x), delta=0.005, threshold=0.05)
    assert hits, "expected at least one change point"
    assert min(abs(h - 1999) for h in hits) <= 5, hits
    print(f"  page-hinkley located step at {hits[0]} (truth 1999)  ok")


def test_displacement_conversion() -> None:
    assert abs(K_DISP / 1000.0 - 3.311) < 0.01          # rad per mm at 79 GHz
    one_rad_um = 1.0 / K_DISP * 1e6
    assert abs(one_rad_um - 302.0) < 1.0, one_rad_um
    print(f"  scale: 1 rad = {one_rad_um:.1f} um at 79 GHz .... ok")


def main() -> int:
    print("bench analysis self-test")
    print("-" * 56)
    for fn in (
        test_idx_roundtrip,
        test_drop_detection,
        test_cross_device_skew,
        test_irregular_cadence_is_not_a_drop,
        test_ledger_recovers_drops_and_drift,
        test_ledger_distinguishes_missed_edges,
        test_calstep_recovers_known_step,
        test_calstep_controls_false_alarms,
        test_blind_fold_is_confusable,
        test_page_hinkley_locates_unmodelled_step,
        test_displacement_conversion,
    ):
        fn()
    print("-" * 56)
    print("all bench analyses recover injected ground truth")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
