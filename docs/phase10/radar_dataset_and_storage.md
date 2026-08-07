# The radar dataset: layout, compression, and cross-language access

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md). The dataset
*is* the deliverable of Phase 10 — the harness exists so that this artifact is
trustworthy. Design goal, stated as a test:

> **A stranger with no access to us can open a dwell, know exactly what the radar
> was doing, when each sample was taken, where the aircraft was, what the subject
> was doing, and what the reference sensor said — and can rebuild any product from
> raw.**

Tags: **[meas]** measured on this machine · **[calc]** arithmetic here ·
**[code]** read from source/docs on GitHub · **[corrob]** search extracts ·
**[unver]** inference · **[prim]** primary document read (2026-08 verification
pass — see [`radar_primary_source_findings.md`](radar_primary_source_findings.md)).

---

## Part A — Format decision

| Candidate | Verdict for the **recorder** | Verdict for **publication** |
|---|---|---|
| **Flat length-prefixed shards + Parquet index** | ✅ Append-only, crash-safe, zero-dependency, memory-mappable; the pattern `raw_mavlink.bin` already proves in this repo | ✅ with documented readers |
| HDF5 | ❌ single-writer only; SWMR appends can't create objects mid-flight, exclude variable-length types, and journaling never shipped (2008 RFC still "under investigation" per HDF Group, 2024) — crash recovery is an external utility, not a library guarantee **[prim, HDF Group docs + forum]** | ✅ great for consumers; produce by offline export |
| Zarr | ❌ many small objects per frame in the RT path | ✅ ideal for chunked N-D cloud access; offline export |
| Parquet with the cube in a binary column | ❌ as the live recorder | ✅✅ **the MATLAB-friendly variant** — see §E.3 |

**Decision:** the recorder writes **flat shards + Parquet index + JSON manifest**.
An offline `radar-export` tool produces the consumer variants (Parquet-embedded,
HDF5, Zarr) from that single source of truth. Nothing in the recorder path depends
on a library a flight build should not carry.

---

## Part B — Layout

```
mission_000123/                          # the existing mission dataset
  manifest.json                          # + a radar section
  radar/
    manifest_radar.json                  # profile+calibration hashes, modes, board fw,
                                         # authorization_id, extrinsics, dataset_version
    dwell_index.parquet                  # one row per dwell (the entry point)
    frame_index.parquet                  # one row per frame: where its bytes are
    edge_ledger.parquet                  # HTE frame-start edges (the µs time base)
    device_state.parquet                 # per-frame die temps, gain, TX mask, cal events
    motion_ref.parquet                   # bracket IMU ≥1 kHz
    flight_context.parquet               # resampled-at-record-time FC state per frame
    ground_truth.parquet                 # chest strap / PPG, own clock + alignment
    scene.parquet                        # per-dwell geometry, posture, surface, subject
    tracks.parquet                       # live-tier detections (derived, reproducible)
    vital_estimates.parquet              # pipeline outputs, with pipeline identity
    pipeline_runs.parquet                # name, version, git hash, config hash, model hash
    shards/
      000000.rsh … NNNNNN.rsh            # compressed frame payloads
    checksums.txt                        # per-shard SHA-256, written at seal
    DATASET_CARD.md                      # generated: what this is, how to read it
```

### B.1 Shard record format

One shard is a sequence of self-describing records — deliberately the same shape
as the proven raw-MAVLink capture, so a torn tail after `kill -9` is expected and
detectable:

```
record header (32 B, little-endian):
  magic u32 = 'RSH1' · dwell_id u32 · frame_index u32 · transform u8 · codec u8 ·
  flags u16 · uncompressed_len u32 · compressed_len u32 · crc32_uncompressed u32 ·
  reserved u32
payload: compressed_len bytes
```

Any language can walk that with `fread`. The Parquet `frame_index` makes it
random-access: `(dwell_id, frame_index) → (shard, byte_offset, len, transform,
codec, crc32)`.

### B.2 The join keys (this is what "very synchronised" means concretely)

Every table carries `dwell_id`, and every per-frame table carries `frame_index`.
The two time columns are always present and never conflated:

* `cc_mono_ns` — the Jetson's monotonic clock, HTE-anchored where available. **The
  canonical axis.**
* `radar_ts_ns` — the TDA2's own per-frame timestamp, recorded **verbatim**.

plus `edge_index` (HTE), `coherence` ∈ {`hardware`, `estimated`, `broken`}, and
`gap_frames` (0 = contiguous). Alignment is *recorded as a fitted result*
(offset + ppm + residual per dwell), never applied silently to a raw column.

### B.3 The tables that make it interpretable

Beyond the obvious, these are the ones whose absence would quietly ruin the
dataset (each earned by a failure mode in the survey):

| Table | Why it exists |
|---|---|
| `device_state` | Four die temperatures, RX gain, TX mask, MIMO scheme, calibration/monitoring events per frame. Phase-vs-temperature drift and the **documented, non-disableable 1 Hz APLL/synth-VCO recalibration** are only removable offline if they sit beside the phase — record the `ENABLE_CAL_REPORT` async reports (timestamp, die temperature, hardware-updated flag) in this table, and note the internal temperature sensor is only ±7 °C accurate **[prim, SPRACF4C + ICD; test E10]** |
| `edge_ledger` | The only µs-class time base; also the authoritative frame count for drop detection |
| `motion_ref` | ≥1 kHz bracket IMU: the platform-motion channel every compensation method needs |
| `scene` | Standoff, depression angle, surface, subject posture, wind, and `control_kind` ∈ {`airborne`,`tethered`,`landed`,`empty`}. **Empty dwells are the false-alarm denominator** |
| `ground_truth` | Chest strap / PPG with its own clock and a recorded alignment method — without it nothing is verifiable |
| `pipeline_runs` | A `vital_estimate` row without pipeline identity is worthless in six months |

---

## Part C — Compression: measured, not hoped

Method **[meas]**: synthetic cascade frames (256 fast-time × 64 chirps × 16 RX,
complex int16) containing complex-Gaussian thermal noise, a **very strong
zero-Doppler clutter/leakage return** (the dominant dynamic-range consumer in any
ground-looking geometry), and several moving targets with per-chirp Doppler and
per-RX spatial phase. Three gain regimes. Script:
`radar_compress_bench.py` (in the session scratchpad; should be committed to
`tools/phase10/`).

### C.1 Lossless

Compression ratio (higher is better), zstd level 1 unless noted:

| Transform | Full-scale (16 bits used) | Typical (peak ≈12 bits) | Quiet scene (peak ≈10 bits) |
|---|---|---|---|
| raw int16 | 1.00 | 1.19 | 1.37 |
| raw int16, lzma-6 | 1.02 | 1.49 | 2.06 |
| byte-plane split | 1.02 | 1.81 | 1.92 |
| delta along fast-time | 1.07 | 1.31 | 1.46 |
| bit-pack to used bits | 1.00 | 1.33 | 1.60 |
| delta along **slow-time** | 1.29 | 1.72 | 2.50 |
| **delta slow-time + byte-plane** | **1.46** | **1.99** | **2.67** |
| delta slow-time + byte-plane, zstd-9 | 1.45 | 2.03 | 2.71 |
| delta slow-time + byte-plane, lzma-6 | — | 2.09 | 2.85 |

Throughput on this x86 container, indicative only — **must be re-measured on the
Orin's A78AE, where expect roughly one third to one half** **[meas]**: zstd-1 on
raw ≈ 640–1340 MB/s; zstd-1 on delta+byte-plane ≈ 380–400 MB/s; zstd-9 drops to
30–70 MB/s; lzma ≈ 2–3 MB/s (unusable in the RT path, fine for archival re-pack).

### C.2 Three conclusions that shape the design

1. **The codec is not where the ratio comes from — the transform is.** Plain
   zstd on raw int16 is 1.0–1.4×. That is why "just enable compression" would
   have disappointed.
2. **The best lossless transform is a slow-time difference, which is also the
   classic static-clutter canceller.** The archive representation and the first
   processing stage are the *same operation*. Store the delta domain and MTI is
   free; reconstruct by cumulative sum along chirps.
3. **Ratio depends on how much of int16 the capture actually uses.** A full-scale
   capture is nearly incompressible; a typical one gives ~2×. So RX gain is a
   *storage* parameter as well as an RF one — and `device_state` must record it or
   the ratios are uninterpretable.

### C.3 Controlled loss, budgeted in micrometres

Dropping the k least-significant bits of a typical (12-bit-peak) capture **[meas]**:

| k | Kept bits | Ratio (zstd-3) | SNR loss | σ_φ (rad) | σ_d at 79 GHz |
|---|---|---|---|---|---|
| 0 | 12 | 1.19 | — | 0 | 0 |
| 2 | 10 | 1.38 | 53.0 dB | 0.0016 | **0.48 µm** |
| 4 | 8 | 1.75 | 39.5 dB | 0.0075 | **2.3 µm** |
| 5 | 7 | 2.36 | 33.3 dB | 0.0153 | 4.6 µm |
| 6 | 6 | 4.19 | 27.2 dB | 0.0310 | 9.4 µm |
| 7 | 5 | 8.00 | 21.1 dB | 0.0624 | 18.9 µm |
| 8 | 4 | 13.12 | 15.0 dB | 0.1251 | 37.8 µm |

Against a cardiac target of ~0.1 mm = **100 µm** (0.33 rad) and respiration of
1–12 mm, even k=6 (4.2× compression, 9.4 µm) looks survivable on paper.

**Read that table with two caveats, both important:**

* σ_d here is derived from *whole-frame* amplitude SNR. Requantisation noise
  spreads across all range bins while a target concentrates under the range FFT's
  processing gain — so for a strong isolated target the true figure is **better**
  than the table, and for a weak target sitting beside a huge clutter return it is
  **worse**. This is an order-of-magnitude budget, not an acceptance test.
* The acceptance test is empirical and belongs in Phase 10.0: record a static
  corner reflector plus a live subject losslessly, then re-quantise offline at each
  k and measure the change in the *final* rate estimates and their confidence.
  That experiment is cheap and it settles the question permanently.

**Policy:** the archival tier is **lossless, always**. Controlled loss is an
opt-in per-dwell mode for endurance sorties, recorded in `frame_index.transform`
with its k, so every consumer can see exactly what was thrown away.

### C.4 Recommended encoder chain

```
int16 cube  →  slow-time delta (int16 wrapping)  →  byte-plane split  →  zstd-1
```

with the transform id recorded per frame so future formats can coexist. Rationale:
best measured lossless ratio, ~400 MB/s single-core headroom against a 63 MB/s
worst-case input **[meas/calc]**, trivially invertible in any language (a
cumulative sum plus a byte interleave — no codec-specific maths), and it lands the
data in the same domain the clutter-cancelling first stage wants.

### C.5 Sortie economics **[calc]**

| Mode | 30 s dwell, raw | at 2× lossless | 30 dwells compressed |
|---|---|---|---|
| VITALS-1 (0.66 MB/s) | 20 MB | 10 MB | ~0.3 GB |
| VITALS-3 (3.93 MB/s) | 118 MB | 59 MB | ~1.8 GB |
| SCAN-12 (62.9 MB/s) | 1.9 GB | 950 MB | ~28 GB |

Against an Orin Nano NVMe planning floor of 200–350 MB/s (Gen3 ×4 slot;
community measurements span ~100–800 MB/s by drive — bench the chosen one, M2)
**[prim]**, even the worst case has 3–5× headroom. **Storage is not the
constraint people assume it is** — provided the transform+codec runs in the
recorder, which costs well under one core.

---

## Part D — Ground truth and controls (non-negotiable)

The dataset's value is proportional to how falsifiable it is:

| Channel | Requirement |
|---|---|
| **Reference vital signs** | A chest-strap ECG/PPG device, sampled and stored with **its own clock plus a recorded alignment procedure**. Use a Polar-H10-class strap — the recent public mmWave vital-sign dataset (Twente 2024) validates against exactly that; note the clinical-grade Erlangen radar datasets use a Task Force Monitor (ECG/ICG/continuous BP) instead, so the strap buys comparability with the newer mmWave sets, not the clinical ones **[prim, the datasets' own papers]** |
| **Scene calibration** | A corner reflector at a surveyed position in every session: absolute phase/amplitude reference, and the static anchor the compensation methods need |
| **Empty dwells** | Identical geometry, no human. The false-alarm denominator; without them "it detected someone" is unfalsifiable |
| **Landed control** | Every airborne session includes a landed/tripod dwell on the same subject, so platform degradation is *measured* rather than argued |
| **Mannequin dwells** | A body-shaped, non-breathing target: separates "detects a person-sized object" from "detects life" |
| **Posture and geometry** | Supine / prone / standing / seated, plus standoff and depression angle — the sensed displacement is chest-normal projected on the line of sight, so geometry is part of the measurement |

---

## Part E — Reading it from three languages

### E.1 Rust

Native: `cc-radar-store` exposes the reader used by `radar-replay`, so an
in-process pipeline sees exactly the recorded bytes.

### E.2 Python

```python
import pyarrow.parquet as pq, numpy as np, zstandard as zstd
idx = pq.read_table("radar/frame_index.parquet").to_pandas()
row = idx[(idx.dwell_id == 7) & (idx.frame_index == 0)].iloc[0]
with open(f"radar/shards/{row.shard:06d}.rsh", "rb") as f:
    f.seek(row.byte_offset + 32)                      # skip the record header
    raw = zstd.ZstdDecompressor().decompress(f.read(row.compressed_len))
cube = inverse_transform(raw, row.transform)          # byte-join, cumsum over chirps
cube = cube.reshape(n_chirp, n_rx, n_samp, 2)         # I/Q last
```

### E.3 MATLAB — and the reason the export variant exists

MATLAB has no built-in zstd, so pointing MATLAB users at the shards would force a
MEX build. But **MATLAB's `parquetread` decompresses Parquet's own zstd pages**.
So `radar-export --format parquet-embedded` writes the *transformed* (not yet
entropy-coded) cube into a Parquet binary column and lets Parquet's page
compression do the work:

```matlab
t = parquetread("radar/export/dwell_0007.parquet");   % zstd handled by MATLAB
buf = t.cube{1};                                      % transformed bytes
cube = inverse_transform_matlab(buf, t.transform(1));  % cumsum + byte de-interleave
```

The inverse transform is deliberately trivial arithmetic in every language — that
is a *format design requirement*, not an implementation detail. For consumers who
prefer it, the same tool emits HDF5 (universally readable, MATLAB-native) and
Zarr (chunked N-D for Python/ML). One source of truth, three convenience exports,
each with a recorded checksum of the source.

### E.4 The parity test

CI runs a trivial pipeline over a committed fixture dwell in Rust, Python and
MATLAB (or Octave in CI) and asserts **byte-identical** output. Cross-language
parity is a test, not an aspiration — otherwise the "any pipeline" promise decays
within a month.

---

## Part F — Integrity, versioning, privacy

* **Integrity:** per-shard SHA-256 at seal; `radar-inspect` recomputes and
  reconciles index ↔ shard ↔ edge ledger, and reports dwell completeness
  (longest gap-free coherent segment) as a first-class number — because that
  length is what determines whether a dwell can yield a heart rate at all.
* **Versioning:** `dataset_version` in the radar manifest; schemas live in one
  module so a change is a compile-time event; a released dataset version is never
  edited in place.
* **Provenance:** profile hash, calibration hash, board firmware, harness git
  hash, and per-row pipeline/model hashes. A capture whose profile or calibration
  hash does not match the configured one is **refused, not recorded**.
* **Privacy:** respiration and heart-rate estimates of identifiable people are
  health-adjacent personal data, and radar cardiac/gait signatures are
  person-distinguishing — so the dataset is pseudonymous at best. Practical
  obligations: pseudonymous `subject_id` with the mapping stored outside the
  dataset, consent records outside the dataset, encryption at rest, an explicit
  retention clock, access control, and a budgeted secure-erase turnaround
  **[corrob]**. Ethics review must be in place **before human data collection
  begins**, not merely before release — the published radar-vitals datasets all
  cite committee approvals obtained ahead of the experiments (FAU 85_15B;
  U Twente CIS 230671) **[prim]** — and the `DATASET_CARD.md` must state the
  approval and consent basis.
