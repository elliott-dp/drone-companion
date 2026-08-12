# Phase 10.0 bench analysis stack

The analysis half of the bench manual
([`docs/phase10/phase10_bench_manual.md`](../../../docs/phase10/phase10_bench_manual.md)).
Pure Python + NumPy, no hardware, no network. Every module is exercised by
`selftest.py`, which builds synthetic artefacts with **known** answers and
asserts the analysis recovers them — so on bench day a surprising result
implicates the radar, not the script.

```bash
python3 -m tools.phase10.bench.selftest      # ~10 s, all cases must pass
```

| Module | Tests served | Contents |
|---|---|---|
| `idx.py` | E2, E4, E5, E7 | `*_idx.bin` reader/writer (24 B header + **48 B** records, asserted at import), torn-tail tolerance, `flags` histogram |
| `capture.py` | **E2**, E4, E5 | Per-device drop report from timestamps alone, gap list with lost-frame counts, *anomaly* class for non-integer deltas, last-frame-lost detection, cross-device skew → one-clock verdict, sweep table |
| `ledger.py` | **E7**, E8 | Three-way reconciliation (edges ↔ `idx` ↔ live tier) by integer slot matching under a single unknown shift; capture drops vs **missed edges** kept distinct; robust affine drift fit → ppm + residual + Locked/Degraded/Unlocked; per-dwell `coherence` verdict |
| `calstep.py` | **E10**, D1 | Synchronous averaging of the differenced phase at logged cal events; blind period fold (documented as confusable); Page–Hinkley for unmodelled steps; minimum-detectable-step and required-epochs sizing; band-RMS report in rad and µm |
| `selftest.py` | — | 11 ground-truth cases |

Constants and band-RMS are imported from
[`../rbec/core.py`](../rbec/core.py) rather than duplicated, so the bench numbers
and the RBEC budget numbers cannot drift apart.

## What the self-test currently proves

```
idx round-trip + torn-tail handling ....... ok
drop detection: found 5/5, rate 0.826% .... ok
cross-device skew: 3 dev spacings detected  ok
irregular cadence flagged as anomaly ...... ok
ledger: 4/4 drops, drift +12.50 ppm (truth +12.5), resid 284 ns  ok
ledger: missed edges kept distinct ........ ok
cal step: recovered +0.02048 rad (+6.18 um) vs truth +0.020; MDS 0.00613 rad, needs 29 epochs .. ok
cal step: false alarms 0/20 at alpha=0.01 . ok
blind fold at 1 Hz is fooled by vibration . ok (documented trap)
page-hinkley located step at 1999 (truth 1999)  ok
scale: 1 rad = 302.0 um at 79 GHz .... ok
```

## E10 detection power — why this test is worth a bench hour

The estimator's minimum detectable step is
`MDS = 4.902 · σ_diff / √N` (two-sided α = 0.01, power 0.99), where `σ_diff` is
the per-sample noise of the *differenced* phase and `N` is the number of 1 Hz
calibration epochs — i.e. the record length in seconds:

| σ_diff (rad) | 60 s | 300 s | 1800 s |
|---|---|---|---|
| 0.005 | 0.0032 rad = **0.96 µm** | 0.0014 rad = 0.43 µm | 0.0006 rad = 0.17 µm |
| 0.010 | 0.0063 rad = **1.91 µm** | 0.0028 rad = 0.85 µm | 0.0012 rad = 0.35 µm |
| 0.020 | 0.0127 rad = **3.82 µm** | 0.0057 rad = 1.71 µm | 0.0023 rad = 0.70 µm |
| 0.050 | 0.0316 rad = 9.56 µm | 0.0142 rad = 4.27 µm | 0.0058 rad = 1.74 µm |

Against a cardiac target of ~100 µm, a **five-minute** static-reflector record
resolves the APLL/VCO step to roughly 1 µm across the plausible noise range. So
the one quantity TI never publishes is cheap to measure to two orders of
magnitude below the signal it might contaminate — and whichever way it lands, the
answer is defensible.

Two guards the self-test enforces rather than assumes: the estimator must **not**
fire on step-free records (0/20 false alarms at α = 0.01), and the blind
fixed-period fold **is** fooled by a 1 Hz mechanical artefact — which is exactly
why the protocol requires `ENABLE_CAL_REPORT` timestamps and treats a blind
positive as inadmissible.

## Model boundaries

These analyses deliberately do **not** model: RF propagation, target
scattering, TDA2 firmware behaviour, or the real hover motion PSD (that is the
RBEC stack's job). `capture.py` infers drops from timestamps because the index
carries no sequence number, and reports non-integer deltas as anomalies rather
than rounding them into drops. `ledger.py` assumes the two clocks share the
configured cadence to within a ppm-level rate — true for a crystal-scheduled
frame timer, and the residual it reports is what falsifies the assumption.
