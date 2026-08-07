# Real-time architecture and compute budget

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md). This document
answers, with arithmetic: **what has to be real-time, what does not, what it
costs, and what could actually overload the companion computer.**

Tags: **[calc]** arithmetic here · **[meas]** measured on this machine (x86
container — indicative only for the Orin) · **[corrob]** search extracts ·
**[unver]** inference · **[prim]** primary document read (2026-08 verification
pass — see [`radar_primary_source_findings.md`](radar_primary_source_findings.md)).

---

## Part A — Framing: this is soft real-time with generous deadlines

At 20 Hz the frame period is **50 ms**. By real-time standards that is enormous —
two orders of magnitude above the scheduling jitter of an ordinary Linux system.
So the risk is *not* scheduler latency, and a PREEMPT_RT kernel (available as a
JetPack option) is almost certainly unnecessary. The real risks are:

1. **Throughput cliffs** — a disk stall, a page-fault storm, a thermal throttle.
2. **Unbounded queues** — the classic way a soft-RT system turns a 50 ms deadline
   into a 5-second one.
3. **Priority inversion between the record and an experiment** — an ML job
   starving the recorder on a device with one GPU and no accelerators.

The architecture is therefore built on bounded queues, preallocation, measured
headroom, and explicit drop accounting — exactly the doctrine `cc-mission-log`
already applies to telemetry.

---

## Part B — Task classes and deadlines

| Class | Tasks | Deadline | If missed |
|---|---|---|---|
| **Hard (for data integrity, not for flight)** | Receive every frame from the transport; capture every SYNC edge; hand every frame to the writer; seal/fsync on schedule | 50 ms per frame; edges immediately (hardware-timestamped, so software latency is irrelevant to *accuracy* — only to not losing the event) | **A gap in the record.** Counted, recorded, and surfaced. Never silently absorbed. |
| **Soft** | Live-tier DSP → tracks → range/angle products → `CC_VITALS_REPORT` assembly; operator display | ~200 ms; report at ≤1 Hz | Skip the tick. The report carries `quality_flags` saying the estimate is stale. |
| **Deferred (the decision path)** | Band separation, rate estimation, and the **ML presence classifier** — the stage that decides `human_present` and releases the vital signs | window cadence (1–2 s) or dwell cadence (~30 s); **explicitly allowed to be late** | Skip the invocation, set `CC_VF_ML_STALE`, and let `decision_age_ms` grow. Emit "undecided" — never a fabricated presence or rate. |
| **Best-effort** | Offline/heavy ML, visualisation, export, compression re-pack | none | Dropped freely; a lossy reader is *told* how many frames it missed. |

Nothing about flight safety appears in this table, and that is deliberate: the
radar subsystem's worst failure is a lost dwell.

---

## Part C — The compute budget, with the arithmetic

### C.1 Classic DSP is effectively free

Per frame, SCAN-12 (12 TX × 16 RX × 16 loops × 256 samples), FFT cost taken as
5·N·log₂N flops **[calc]**:

| Stage | Count × size | FLOP/frame | At 20 Hz |
|---|---|---|---|
| Range FFT | 3072 × 256-pt | 31.5 M | 629 MFLOP/s |
| Doppler FFT | 49 152 × 16-pt | 15.7 M | 315 MFLOP/s |
| Angle FFT | 4096 × 256-pt | 41.9 M | 839 MFLOP/s |
| **Total** | | **89.1 M** | **≈ 1.8 GFLOP/s** |

Orin Nano: 1024 Ampere CUDA cores, 32 tensor cores, 40 sparse / 20 dense INT8
TOPS, 68 GB/s LPDDR5, 6× Cortex-A78AE — and **no DLA, no PVA** **[prim,
DS-11105-001 v1.5: absent by product spec; the datasheet states "CUDA Core
Performance: 1.28 FP32 TFLOPs"]**, so the whole DSP chain is **~0.14 % of
peak**, or ~1–2 % at realistic FFT efficiency. JetPack 6's MAXN_SUPER operating
point raises the ceiling further (2.08 FP32 TFLOPs, 67 sparse TOPS, 102 GB/s)
at a thermal cost the flight enclosure must earn (M7) **[prim]**.

Memory traffic matters more than FLOPs: ~8 passes over a 3 MiB cube at 20 Hz is
≈ **0.6 GB/s against 68 GB/s ≈ 0.9 %** **[calc]**.

For the VITALS modes the same chain is 20–100× smaller again.

**Conclusion: the DSP is not the problem, and moving it to the radar board to
"save CPU" solves a problem that does not exist.**

### C.2 Ingest, compression and storage

| Item | SCAN-12 | VITALS-3 | Capability |
|---|---|---|---|
| Wire/ingest rate | 62.9 MB/s (503 Mbit/s) | 3.93 MB/s | ~110 MB/s practical on 1 GbE **[calc]** |
| Transform + zstd-1 | ~0.4 core (assuming ~165 MB/s/core on A78AE, from 380–400 MB/s measured on x86) | ~0.03 core | 6 cores **[meas-derived, must re-measure]** |
| Stored rate at ~2× lossless | ~32 MB/s | ~2 MB/s | NVMe sustained: plan at 200–350 MB/s floor; community fio spans ~100–800 MB/s by drive (Gen3 ×4 slot) — bench the chosen drive (M2) **[prim]** |
| Parquet index rows | 20/s | 20/s | trivial |
| HTE edges | 20/s | 20/s | trivial |
| Bracket IMU @1 kHz | ~32 kB/s | same | trivial (but needs a low-jitter reader) |

Headroom is 3–5× on every axis for SCAN-12 and ~50× for the VITALS modes. That is
the design target: **if a mode does not leave 3× headroom on all four axes, it is
not a supported mode.**

### C.3 ML in the pipeline: the cadence decides everything

ML is part of the pipeline — the presence decision, and therefore whether vital
signs are emitted at all, comes out of its end. The question is never "can the
Orin Nano run a model" but **"at what cadence"**. Same model, three cadences
**[calc]**:

| Model | every frame (20 Hz) | every 1 s | every 2 s | once per 30 s dwell |
|---|---|---|---|---|
| 2D net, range-Doppler (256 × 64), ~1–2 GFLOP | 20–40 GFLOP/s (~3 %) | ~2 GFLOP/s | ~1 GFLOP/s | negligible |
| 2D net over several beams, ~5–10 GFLOP | 100–200 GFLOP/s (~15 %) | ~10 GFLOP/s (~0.8 %) | ~5 GFLOP/s | negligible |
| 3D net on the full cube, ~100–500 GFLOP | **2–10 TFLOP/s — impossible** | 100–500 GFLOP/s (8–40 %) | 50–250 GFLOP/s | 3–17 GFLOP/s (~1 %) |

And in wall-clock duty-cycle terms, which is the honest unit for published models
**[prim, corrected against the primaries]**: DPDCNet takes **0.26 s per 10-s
sample on an RTX 3090** — 26 % of that GPU at 1 Hz, under 1 % once per dwell; on
this device expect a few× slower, still dwell-viable. The pipeline previously
misquoted as "~1.7 s on a 1080 Ti" actually reports 3.719 s per 10-s sample of
which 3.157 s is **CPU-bound RoI selection** (network inference 0.006 s), so the
earlier "8–17 s on this device" GPU-ratio extrapolation was invalid — the cost
scales with the A78AE cores, not the GPU. Either way the conclusion stands: a
decision that arrives a few seconds after a 30 s hover is operationally fine
*once per dwell*.

**So the architecture places ML at window and dwell cadence, and the deadline is
explicitly soft.** That is the whole reason it fits.

Rules the harness enforces rather than trusting to discipline:

* Every stage declares a **budget in milliseconds and a cadence**. The runtime
  measures actual cost, and when a stage runs long the *next* invocation is skipped
  and recorded — the report then carries `CC_VF_ML_STALE` and a growing
  `decision_age_ms` instead of a stale answer presented as fresh.
* ML runs in a **low-priority CUDA stream**; the frame-rate DSP runs high-priority.
  With no DLA and no PVA **[prim]**, stream priority is the only isolation the
  device offers.
* Online models are **quantised (INT8/FP16) and distilled**; the FP32 research model
  stays in the offline path, and both are recorded by model hash so a live decision
  is reproducible.
* A model process that dies, hangs or OOMs must be **invisible to the recorder** —
  hence out-of-process with a lossy reader (§D.2). The pipeline degrades to
  "undecided", which is a legitimate answer; it never degrades the record.

---

## Part D — Implementation architecture

### D.1 Process and thread layout

```
companiond (existing)                    ← FC link, mission log, health; untouched
radar-harness
  ├─ thread: control        (TCP:5001 session, dwell FSM, interlock)        low rate
  ├─ thread: sync           (HTE edge reader, IMU reader)         SCHED_FIFO, pinned
  ├─ thread: ingest         (transport → preallocated ring)       SCHED_FIFO, pinned
  ├─ thread: store          (transform → zstd → shard append → index)      pinned
  ├─ thread: live-dsp       (GPU: range/Doppler/angle, high-priority stream)
  └─ thread: report         (rate estimate → CC_VITALS_REPORT → companiond)
pipeline processes (0..N)  ← attach to the shm ring; lossy; killable; unprivileged
```

Why separate from `companiond`: a radar subsystem crash must not take down the
mission log or the FC link. The two communicate over a local socket, and the
report path is a small message — the same "no shared fate" principle the repo
applies between PX4 and the companion, one level down.

### D.2 The shared-memory frame ring (the pipeline bridge)

* Fixed-size slots sized to the largest supported mode; **preallocated at start**,
  never resized.
* Single writer, many readers. Each slot carries `(sequence, dwell_id,
  frame_index, cc_mono_ns, len, crc32)` and a **seqlock** so a reader can detect a
  torn read rather than consuming half a frame.
* Readers are **lossy by construction**: a reader that falls behind sees the
  sequence jump and is told how many frames it missed. No reader can ever apply
  back-pressure — the property that keeps a Python or MATLAB pipeline from
  endangering the record.
* The bytes in the ring are **exactly the bytes recorded** (post-transform,
  pre-compression), so an online result is reproducible offline by construction.

### D.3 Storage path

* Append-only shards, `O_DIRECT`-eligible or plain buffered writes with
  **periodic** fsync (never per frame — that is a throughput cliff).
* One writer thread; the index row is appended in the same critical section as the
  shard record so index and data can never disagree about what exists.
* Preallocate/fallocate shard files to avoid metadata churn mid-dwell.
* Seal at dwell boundaries and on a size cap; checksum at seal.

### D.4 Platform tuning (Jetson-specific, and each is measurable)

| Item | Action | Why |
|---|---|---|
| Power mode | `nvpmodel` to the highest mode the thermal design supports; `jetson_clocks` to pin clocks | Dynamic clocking is a latency and throughput variance source |
| Core isolation | `isolcpus`/cpuset for sync + ingest + store; leave general cores for pipelines | Keeps the hard class away from best-effort work |
| Scheduling | `SCHED_FIFO` at modest priority for sync/ingest, with RT throttling configured so a bug cannot wedge the box | 50 ms deadlines do not need aggressive priorities |
| IRQ affinity | NIC IRQs off the store core | Avoids interrupt/write contention |
| Network | Raise `rmem`, use `recvmmsg` (or `io_uring`/AF_XDP if 60 MB/s proves marginal), jumbo frames if the producer supports them | 500 Mbit/s of small packets is where naive sockets fail |
| Allocation | No allocation in steady state; all buffers preallocated | The repo's existing discipline, extended |
| GPU | Explicit pinned host buffers and copies; **avoid unified-memory page faults**; CUDA stream priorities | Page-fault stalls are the classic Jetson performance trap |
| Thermal | Record SoC temperature and clock state per dwell | A throttle event changes drop behaviour; without the record, a bad dwell is unexplainable |
| Watchdog | systemd `WatchdogSec` + an internal deadline monitor writing `rt_events.parquet` | Missed deadlines become data, not folklore |

### D.5 The degradation ladder (what gives way, in order)

1. Best-effort consumers (ML, visualisation) miss frames — recorded.
2. The live tier reduces rate (20 → 10 → 5 Hz of derived product) — recorded.
3. Lossy compression engages *if configured for this dwell* — recorded with its k.
4. The dwell is marked `coherence_broken` and the gap list stored.
5. **The recorder never silently stops.** If it cannot write, that is a FAULT,
   surfaced in `payload_state` and in the report.

Note what is *not* in the ladder: the record never gives way to keep a display
alive. That inverts the priority the live tier might suggest, and it is the correct
inversion — the dwell is unrepeatable, the display is not.

---

## Part E — Measurements to take on the actual Orin (nothing here is a guess)

| # | Measurement | Why it matters |
|---|---|---|
| M1 | zstd-1 throughput on the transform chain, per core, on A78AE | The 0.4-core figure is extrapolated from x86 **[meas]**; confirm it |
| M2 | Sustained NVMe write with the real record pattern, 30+ min | Vendor numbers hide sustained-write cliffs and thermal effects |
| M3 | UDP/TCP receive at 500 Mbit/s with the real frame size, loss counted | Decides whether plain sockets suffice or `io_uring`/AF_XDP is needed |
| M4 | cuFFT batched throughput for the three FFT stages, and end-to-end live-tier latency | Confirms the ~2 % figure and sets the live-tier budget |
| M5 | HTE edge capture reliability over 30 min (zero missed edges) and jitter | The time base's credibility |
| M6 | Bracket-IMU read jitter at 1 kHz | Determines whether the µs class is actually achieved |
| M7 | Thermal soak in the real enclosure at full load | Fanless flight enclosures throttle; a throttle changes everything above |
| M8 | Power draw per mode | Flight endurance is part of the design |
| M9 | End-to-end latency from frame to `CC_VITALS_REPORT` | Operator experience, and the ELRS budget |
| M10 | Worst-case latency with an ML pipeline attached and deliberately overloaded | Proves the isolation actually isolates |

Every one of these belongs in the phase's committed evidence, in the same style as
the existing phases' measured results — and each replaces a number in this document
that is currently arithmetic or extrapolation.
