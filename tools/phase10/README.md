# tools/phase10 — radar harness tooling

Phase 10 is a **proposal** ([`docs/phase10/`](../../docs/phase10/)); nothing in the
Rust workspace implements it yet. What lives here is the tooling the proposal
depends on for its numbers, so that every figure in the design documents is
reproducible rather than asserted.

## `radar_compress_bench.py`

Measures how compressible TI cascade raw ADC frames actually are, and what
controlled-loss modes cost in *displacement noise* — the unit that matters for a
vital-signs payload.

```bash
pip install numpy zstandard
python3 tools/phase10/radar_compress_bench.py
```

It synthesises physically-shaped frames (complex int16; complex-Gaussian thermal
noise, a very strong zero-Doppler clutter/leakage return, and several moving
targets with per-chirp Doppler and per-RX spatial phase) at three gain regimes,
then reports:

* lossless compression ratio for six reversible transforms × six codecs, with
  input throughput, and
* controlled-loss requantisation: ratio, SNR loss, phase-noise σ_φ, and the
  implied displacement σ_d in micrometres at 79 GHz.

Results and their interpretation are in
[`docs/phase10/radar_dataset_and_storage.md`](../../docs/phase10/radar_dataset_and_storage.md) §C.
Headline: the ratio comes from the **transform**, not the codec — plain zstd on
raw int16 is 1.0–1.4×, while a slow-time delta plus byte-plane split reaches
1.5–2.7× losslessly, and that transform is also the classic static-clutter
canceller.

**Caveat carried in the docs too:** the numbers were produced on an x86 container.
Ratios should transfer; **throughput must be re-measured on the Orin's A78AE**
(expect roughly a third to a half), and the compression acceptance test must
ultimately run on *real* captures, not synthetic ones.

## `bench/` — the Phase 10.0 bench analysis stack

See [`bench/README.md`](bench/README.md). Turns the E-test inventory into
analyses that run before the hardware exists: `*_idx.bin` parsing, drop-rate
reporting (E2/E4/E5), three-way ledger reconciliation and drift fitting
(E7/E8), and the calibration-step estimator with its detection-power sizing
(E10/D1). `python3 -m tools.phase10.bench.selftest` proves all eleven analyses
recover injected ground truth. Protocols that consume it:
[`docs/phase10/phase10_bench_manual.md`](../../docs/phase10/phase10_bench_manual.md).

## `rbec/` — RBEC numerical validation

See [`rbec/README.md`](rbec/README.md).

## Planned (not yet written)

* `fake_radar.py` — control-plane test double (TCP:5001 session emulating the
  configure/arm/start/stop sequence) plus a synthetic frame producer, so the
  harness is testable in CI with no radar attached.
* `pipelines/{rust,python,matlab}/` — trivial reference pipeline stages whose
  byte-identical outputs prove the cross-language contract.
