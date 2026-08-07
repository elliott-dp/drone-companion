# Real-time architecture and compute budget

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md). This document
answers, with arithmetic: **what has to be real-time, what does not, what it
costs, and what could actually overload the companion computer.**

Tags: **[calc]** arithmetic here · **[meas]** measured on this machine (x86
container — indicative only for the Orin) · **[corrob]** search extracts ·
**[unver]** inference.

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
| **Soft** | Live-tier DSP → tracks → rate estimate → `CC_VITALS_REPORT`; operator display | ~200 ms; report at ≤1 Hz | Skip the tick. The report carries `quality_flags` saying the estimate is stale. |
| **Best-effort** | ML inference, visualisation, offline export, compression re-pack | none | Dropped freely; a lossy reader is *told* how many frames it missed. |

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

Orin Nano: 1024 Ampere CUDA cores, 32 tensor cores, ~40 TOPS INT8, 68 GB/s
LPDDR5, 6× Cortex-A78AE — and **no DLA, no PVA** **[corrob]**. FP32 peak is
roughly 1.3 TFLOP/s **[calc]**, so the whole DSP chain is **~0.14 % of peak**, or
~1–2 % at realistic FFT efficiency.

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
| Stored rate at ~2× lossless | ~32 MB/s | ~2 MB/s | 200–350 MB/s NVMe sustained **[corrob]** |
| Parquet index rows | 20/s | 20/s | trivial |
| HTE edges | 20/s | 20/s | trivial |
| Bracket IMU @1 kHz | ~32 kB/s | same | trivial (but needs a low-jitter reader) |

Headroom is 3–5× on every axis for SCAN-12 and ~50× for the VITALS modes. That is
the design target: **if a mode does not leave 3× headroom on all four axes, it is
not a supported mode.**

### C.3 ML is the only thing that can plausibly saturate the device

| Model shape | Cost/inference | At 20 Hz | Verdict |
|---|---|---|---|
| 2D net on a range-Doppler map (256 × 64) | ~1–2 GFLOP | 20–40 GFLOP/s | ~2–3 % of FP32 peak — **fine** |
| 2D net on range-azimuth, several beams | ~5–10 GFLOP | 100–200 GFLOP/s | ~8–15 % — **fine with a budget** |
| 3D net on the full cube (256 × 64 × 192) | ~100–500 GFLOP | **2–10 TFLOP/s** | **beyond the device** |
| Published radar denoise+classify network | 0.26 s/sample **[corrob]** | ≈ 4 Hz | offline only |
| Published hybrid method on a desktop 1080 Ti | ~1.7 s **[corrob]** | ≈ 0.6 Hz | offline only |

Rules that follow, and they are enforced by the harness rather than by discipline:

* Every online model declares a **frame budget in milliseconds**; the runtime
  measures actual cost and **skips inference when late**, recording the skip.
* Online models run in a **low-priority CUDA stream**; the live tier runs in a high-
  priority stream. On a single-GPU device with no DLA/PVA, stream priority is the
  only isolation available.
* Models are **quantised (INT8/FP16) and distilled** for online use; the FP32
  research model stays offline.
* An ML process that dies, hangs or OOMs must be invisible to the recorder —
  hence out-of-process (§D.2).

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
