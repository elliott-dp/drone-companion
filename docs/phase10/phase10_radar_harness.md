# Phase 10 (proposed) — the radar harness: MMWCAS ↔ Jetson link, and an environment any pipeline can be built in

> **Status: design proposal. No code has shipped for this phase.**
>
> **What Phase 10 delivers:** a *harness*, not an algorithm.
> 1. The radar↔Jetson link works, deterministically, with known timing.
> 2. Everything is recorded — compressed raw plus every synchronised context
>    channel — in an organised, documented, tool-agnostic dataset.
> 3. Any signal-processing or ML pipeline (Rust, Python, MATLAB) can be attached
>    to that data **offline or online, without touching the recorder**.
>
> **Explicitly out of scope:** implementing the vital-signs or detection
> algorithms. §D and the companion survey enumerate what they could be and what
> must be tested; choosing and building them is Phase 11+ work, done *against*
> this harness.

**Companion documents**

| Document | Covers |
|---|---|
| [`radar_transport_and_sync.md`](radar_transport_and_sync.md) | The four possible data paths off the RF-EVM, the control plane, and the timing/sync architecture with its measured-vs-assumed budget |
| [`radar_dataset_and_storage.md`](radar_dataset_and_storage.md) | The dataset layout, schemas, **measured compression results**, ground-truth channels, and readers for Rust / Python / MATLAB |
| [`radar_dsp_ml_survey.md`](radar_dsp_ml_survey.md) | The extensive algorithm survey: every stage, its alternatives, the ML landscape, and what should and should not be tested |
| [`radar_realtime_budget.md`](radar_realtime_budget.md) | Task classes and deadlines (including the deferred ML decision path), the compute budget with arithmetic, and the answer to "will the monitoring tier overload the CC?" |
| [`radar_fc_integration.md`](radar_fc_integration.md) | **The PX4-side specification**: dialect additions (54014–54016 + one extension field), uORB topics, the `cc_payload_bridge` module, stream classes, receiver-gauntlet entries, parameters, files touched, and the 868 MHz ELRS downlink path to the operator |
| [`radar_rbec_method.md`](radar_rbec_method.md) | **Method proposal (RBEC)**: reference-beam ego-motion cancellation — target + reference beams from one datacube, the anchor least-squares/ANC estimators, the shared-LO common-mode analysis, budgets, failure modes, and a six-rung validation ladder. The developed form of the survey's §B.10 "beam as second channel" idea |
| [`radar_rbec_validation.md`](radar_rbec_validation.md) | **RBEC numerical validation (V1 groundwork)**: seeded simulations ([`tools/phase10/rbec/`](../../tools/phase10/rbec/README.md)) showing the budget closes on paper, the chest-velocity unwrap requirement, the calibration-spur nuance, and the geometry results — adversarially reviewed, defects fixed |
| [`radar_primary_source_findings.md`](radar_primary_source_findings.md) | The 2026-08 primary-source verification record: per-claim verdicts, citations, corrections applied, and the answers to the survey's five open questions |
| [`radar_vitals_bank_validation.md`](radar_vitals_bank_validation.md) | **Vitals estimator bank, benchmarked on real clinical radar data** (Erlangen 24 GHz CW, ECG reference): confidence-gated HR MAE 3.05 BPM at 30 % coverage — the three-state doctrine validated at the estimator layer; code in `tools/phase10/vitals/` |

**Sourcing tags** used throughout: **[calc]** arithmetic done here · **[meas]**
measured on this machine · **[code]** read from source code or project
documentation on GitHub · **[corrob]** multiple independent search extracts,
primary document not read · **[unver]** single source or inference · **[prim]**
primary document read.
History: the first drafting session had `ti.com`, `arxiv.org`, `ieeexplore`,
`pmc.ncbi.nlm.nih.gov`, `ecfr.gov`, `docs.nvidia.com`, `expresslrs.org` and
`docs.px4.io` blocked, so §H listed what still had to be opened. The **2026-08
primary-source verification pass** opened them; the per-claim record — verdicts,
exact citations, and the corrections applied here — lives in
[`radar_primary_source_findings.md`](radar_primary_source_findings.md).

---

## Part A — Roles: who does what

```mermaid
flowchart LR
    OP(["👤 Operator<br/>RadioMaster TX"])
    subgraph AIR["Aircraft"]
      FC(["PX4 / CUAV V6X"])
      CC(["Jetson Orin Nano<br/>companiond"])
      RAD(["MMWCAS-RF-EVM + DSP-EVM"])
    end
    OP -->|"ELRS uplink: mission start/stop switch"| FC
    FC -->|"start/stop + flight state + airborne flag"| CC
    CC -->|"CC_VITALS_REPORT ≤1 Hz, ~32 B"| FC
    FC -->|"ELRS downlink telemetry"| OP
    CC <-->|"control TCP + data + SYNC edge"| RAD
    RAD -.->|"raw to on-board SSD, offloaded post-flight"| CC
```

| Component | Responsibility | Explicitly **not** its job |
|---|---|---|
| **Operator / RadioMaster** | Starts and stops the mission with a switch; reads presence + vital signs on the handset/GCS | — |
| **PX4 (FC)** | Relays the start/stop intent to the companion; supplies flight state and the airborne flag; forwards and logs `CC_VITALS_REPORT`; keeps flying | Any processing, any decision, any policy on radar content. Radar never enters `cc_safety_monitor`. |
| **Jetson (CC)** | Owns the radar: configure, trigger, capture, timestamp, compress, record, derive the live product, report | Commanding the aircraft |
| **Radar** | Chirps and produces raw ADC | Deciding anything |

### A.1 The FC contract, in full

**FC → CC, mission start/stop.** The pilot's switch travels RadioMaster → ELRS
uplink → PX4. Two routes, and the recommendation is to build both because they
cost almost nothing:

| Route | Mechanism | Verdict |
|---|---|---|
| **Primary** | PX4's onboard stream set already includes `RC_CHANNELS`; the companion reads an AUX channel and edge-detects it **[prim, `mavlink_main.cpp`: ONBOARD profile streams it at 20 Hz — as does ONBOARD_LOW_BANDWIDTH, so the route survives a profile downgrade]** | **Zero PX4 code.** Build this first. |
| Hardening | A `CC_PAYLOAD_COMMAND` in the CC dialect, emitted by the fork on an RC edge, carrying an explicit command + monotonic sequence | Better semantics (debounce and edge detection live where the RC data lives), but needs fork code. Phase 10.3. |

Failure semantics, which matter more than the mechanism:

* **RC lost ⇒ never start a capture.** An unknown switch position holds the
  current state; it does not begin one. Stopping on RC loss is a configuration
  choice (`on_rc_loss = hold | stop`), defaulting to `hold` so a brief dropout
  does not truncate a 30 s dwell.
* Every transition is recorded with its source (`rc_edge`, `dialect_cmd`,
  `operator_gcs`, `auto_timeout`) — so the dataset can always answer *why* a
  dwell started.
* The **airborne-transmit interlock overrides start** unconditionally (§A.2).

**CC → FC → operator, the report.** One compact, **self-contained** message at
≤ 1 Hz: presence (3-state: no / yes / *undecided*), track count, best range and
azimuth, respiration and heart rate with per-estimate confidences, elapsed coherent
dwell seconds, `decision_age_ms`, quality flags, payload state, and
`last_command_seq` as the command ack. 36 B payload → ~48 B on the wire **[calc]**.

Full field-by-field definition, the enums, the PX4-side plumbing and the reasons
behind each choice live in **[`radar_fc_integration.md`](radar_fc_integration.md)** —
that document is the single source of truth for the wire contract, so this one does
not duplicate the table.

Two design points worth stating here because they are easy to get wrong:
*presence is three-state* (collapsing "undecided" into "no" turns a
not-yet-answered question into a negative result — the failure mode that gets
people missed), and *a rate of 0 means "no estimate", never a guess.*

Sized against measured ELRS MAVLink-mode bandwidth **[prim, confirmed against
the `expresslrs.org/software/mavlink` throughput tables; every percentage
re-derived]** — MAVLink mode forces a 1:2 telemetry ratio:

| ELRS mode | Downlink | One 48 B report at 1 Hz costs |
|---|---|---|
| **868/900 MHz K1000 Full** (LR1121) | ~4420 B/s | 1.1 % |
| **868/900 MHz 200 Hz Full** | ~880 B/s | 5.5 % |
| 2.4 GHz F1000 | ~2375 B/s | 2.0 % |
| 2.4 GHz 333 Hz Full | ~1470 B/s | 3.3 % |
| 2.4 GHz 50 Hz | ~110 B/s | **44 %** |

⇒ 1 Hz is comfortable on the realistic 868 MHz modes; the lever for constrained
links is the **rate** (`CC_PL_TEL_HZ` → 0.2 Hz), not a second message format —
MAVLink 2's zero-truncation already shrinks a report that has no estimate yet.

**The EU 868 constraint matters more than the byte count — but the mechanism is
not the one this document previously described.** The primary-source pass
refuted the LBT premise **[prim]**: ELRS implements Listen Before Talk only in
the 2.4 GHz CE build (`Regulatory_Domain_EU_CE_2400`); the EU868 build compiles
no-op stubs and no duty-cycle limiter — it is plain 13-channel FHSS across
863.275–869.575 MHz, with EN 300 220 compliance (25 mW e.r.p. in those
sub-bands; no 100 mW tier exists) resting on the operator. The real downlink
loss model in MAVLink mode is RF packet loss plus the TX module's 16-message
buffer shedding whole messages when oversubscribed, with a *stubborn sender*
retrying undelivered telemetry — so reports arrive **late rather than never**.
The design consequence is unchanged and now better-founded: the report is
**self-contained and idempotent** — no deltas, no implied state, absolute values
plus `decision_age_ms`, so any single packet that lands is fully interpretable
whether it was delayed, retried, or its neighbours were shed. Plan at ~50 % of
nominal, and never oversubscribe `MAV_x_RATE` (details in
[`radar_fc_integration.md`](radar_fc_integration.md) §G).

Operator display has two paths that differ a lot in effort — a GCS speaking the CC
dialect works with only the PX4 changes, while handset-only (EdgeTX) display needs
a custom CRSF frame plus a Lua widget, because ELRS converts MAVLink→CRSF through
a fixed `msgid` switch and a custom dialect message hits no case **[prim,
`MAVLink.cpp`]**. Two facts recovered from the source soften this: PX4 streams
its profile set from boot (no GCS needed for the mapped sensors to appear), and
`STATUSTEXT` is forwarded as Yaapu passthrough — a zero-display-code interim
alert channel. Both paths are specified in
[`radar_fc_integration.md`](radar_fc_integration.md) §G.3.

### A.2 The one hard gate: airborne transmit inhibit

Unchanged from the previous revision of this document, and it survives the
reframing intact, because it is a legal constraint rather than an engineering
one — now verified against the current eCFR text and the EU instruments
**[prim]**:

* **US, 76–81 GHz:** § 95.3333 prohibits the service aboard aircraft in flight
  and requires "a mechanism that will prevent operations once the aircraft
  becomes airborne" (quote matches the rule text verbatim). § 95.3331's
  permitted-use list is closed (vehicular; airport air-operations-area;
  aircraft-mounted for **ground use only**), and the FCC's stated rationale is
  Radio Astronomy Service protection — so a waiver is unlikely, and **FCC
  Part 5 experimental licensing is the only identified US airborne route**
  (Form 442 or STA; the grant must expressly authorise airborne operation;
  expect non-interference and possible RAS-coordination conditions). Ground
  and tripod work is licence-by-rule (§ 95.3305) at 50 dBm average / 55 dBm
  peak EIRP (§ 95.3367) — this payload sits far below the limit.
* **US, 60 GHz — the previous claim "closed for aircraft too" was wrong:**
  47 CFR § 15.255(b)(3) (added July 2023) permits field-disturbance-sensor/radar
  on **unmanned aircraft at 60–64 GHz**, unlicensed: ≤ 20 dBm peak EIRP,
  off-times ≥ 2 ms summing ≥ 16.5 ms per 33 ms (~50 % duty), ≤ 400 ft AGL.
* **EU, 76–81 GHz:** the exclusion is real but works by **closed scoping, not
  an explicit airborne prohibition**: 77–81 GHz is designated only for
  automotive SRR ("road vehicle based radar functions", Decision 2004/545/EC;
  EN 302 264 requires permanent fixed installation on a wheels/rails vehicle —
  aircraft count only while taxiing), and the sole airborne 76–77 GHz category
  is obstacle detection on **manned** (EASA CS-27/CS-29) rotorcraft. Airborne
  trials need an individual national experimental authorisation (the EU
  analogue of the Part 5 grant this document's `authorization` field already
  anticipates). ETSI TR 104 078 (2025) has formally asked CEPT/ECC to open
  76–77 GHz for onboard UAS radar — track it.
* **EU, 57–64 GHz:** generic SRD (100 mW e.i.r.p., EN 305 550) **already
  permits airborne use** — no individual licence needed.

⇒ Strategic consequence the earlier draft could not see: **60 GHz is the lawful
airborne band in *both* jurisdictions today.** The 77–81 GHz cascade remains
the ground/tripod instrument and the reason the harness is built first; if the
airborne phase stalls on authorisations, a 60–64 GHz TI sensor (IWR6843-class)
is the legal airborne fallback at the cost of aperture and range — and the
harness, dataset format and FC contract carry over unchanged.

```
permit_tx(airborne_state, rc_command, authorization, config) -> Permit | Inhibit(reason)
```

* `airborne_state` derives from PX4 (`arming_state` / `nav_state` / landed
  state) — **fail-safe: unknown or stale ⇒ Inhibit.**
* Airborne transmit requires a configured authorization identifier (e.g. an FCC
  Part 5 experimental grant reference), recorded into the manifest. Absent it,
  airborne ⇒ Inhibit, logged and surfaced in the report's `payload_state`.
* Exhaustive host-side truth table, in the style the `cc_policy_table` already
  established.

This gate is *why* the harness is worth building first: **ground, tripod and
tethered work is lawful and is where every algorithm gets developed** — and it is
also where the physics is easiest. The harness makes that phase produce data that
the airborne phase can be compared against.

---

## Part B — Architecture

### B.1 The two-path principle

The single most important architectural fact: **the reliable recording path and
the real-time path are different paths, and that is fine.**

| | Path R — bulk record | Path L — live tier |
|---|---|---|
| Producer | TDA2 capture to on-board SSD (TI-supported) | TDA2 Ethernet stream, or the Orin's own DSP over a reduced raw stream |
| Content | Full-rate raw ADC, every dwell | Reduced, *unselected* complex product + derived tracks/rates |
| Purpose | The dataset. ML training. Any future algorithm. | Operator display, `CC_VITALS_REPORT`, "is the payload alive" |
| Latency | Post-flight offload | ≤ ~200 ms |
| If it fails | Mission is scientifically dead → treat as a hard fault | Degrade silently; the record is unaffected |

They are joined by the **frame index and the SYNC edge ledger**, not by
timestamps alone (see [`radar_transport_and_sync.md`](radar_transport_and_sync.md)).
Whether the TDA2 can do both at once is an open bench test — and if it cannot,
the fallbacks are documented there, including capturing CSI-2 directly into the
Orin and dropping the TDA2 entirely.

### B.2 Crates (harness only — no algorithms)

```
crates/
  cc-radar/            control plane: TCP:5001 session, mmwavelink-equivalent
                       configure/arm/start/stop sequence, profile+calibration
                       hashing, dwell FSM, airborne-inhibit gate, *_idx.bin parser
  cc-radar-sync/       the timing domain: SYNC edge capture via Tegra HTE,
                       frame-index ledger, radar<->CC offset estimate + quality
  cc-radar-io/         frame ingest (Path L), bounded queues, zero-copy ring
  cc-radar-store/      the recorder: transform + zstd shards, Parquet index,
                       manifest, integrity, offload/verify
  cc-radar-pipe/       *** the extension point ***: PipelineStage trait, the
                       shared-memory/IPC bridge, and the offline driver
apps/
  radar-harness        the daemon-side binary (or a companiond subsystem)
  radar-inspect        dataset verification: index<->shard consistency, gap and
                       coherence reporting, checksum verify, dwell completeness
  radar-replay         feed any recorded dwell to a pipeline, at rate or as fast
                       as possible, live-identical
tools/phase10/
  fake_radar.py        control-plane test double + synthetic frame producer
  pipelines/           reference stubs: rust/, python/, matlab/ (no algorithms —
                       each just proves the contract end to end)
```

### B.3 The pipeline extension point (the part the user actually asked for)

The harness must let a pipeline be written in **Rust, Python or MATLAB**, run
**offline or online**, and be swapped without touching the recorder. Three
attachment modes, all fed by one canonical frame representation:

| Mode | Mechanism | For |
|---|---|---|
| **Offline** (default, always available) | `radar-replay` reads a recorded dwell and hands frames to a pipeline process; or the pipeline reads the dataset directly with the documented readers | Algorithm development, ML training, MATLAB prototyping, regression |
| **Online, out-of-process** | Frames published into a **shared-memory ring buffer** (`/dev/shm`, fixed slot size, sequence-numbered, single writer / many readers, readers may fall behind and are told so) + a small control socket | Python/MATLAB pipelines that must run live but must never be able to stall the recorder |
| **Online, in-process** | A Rust `PipelineStage` trait, same lifecycle as the offline driver | Low-latency Rust stages that feed `CC_VITALS_REPORT` |

The contract that makes all three equivalent:

```rust
pub trait PipelineStage: Send {
    fn on_frame(&mut self, f: &RadarFrameView) -> StageOutput;   // fold; must not block
    fn on_dwell_start(&mut self, d: &DwellMeta);
    fn on_dwell_end(&mut self, d: &DwellMeta) -> Option<DwellResult>;
}
```

Non-negotiable properties, each inherited from a lesson the repo already learned:

1. **A pipeline can never back-pressure the recorder.** Readers are lossy; a
   slow reader misses frames and is *told* how many (the `broadcast::Lagged`
   pattern already used for the mission log).
2. **Frames handed to a pipeline are exactly what was recorded**, bit for bit —
   so an online result is always reproducible offline. Any stage that wants to
   consume something else must record it first.
3. **The pipeline's identity is recorded**: name, version, git hash, config hash,
   and for ML the model hash. A `radar_vital_estimate` row without a pipeline
   identity is worthless six months later.
4. **Every output is timestamped in the recorded time base**, never in wall
   clock, so `radar-replay` reproduces it exactly.
5. **Language parity is proven by a test, not by intent**: `tools/phase10/pipelines/`
   holds a trivial stage in each of Rust, Python and MATLAB, and CI asserts all
   three produce byte-identical output on a committed fixture dwell.

---

## Part C — Answers to the four questions raised

### C.1 "Will the streamed pre-processed tier be expensive for CPU load? Won't my ML pipeline overload the CC?"

**Not from the DSP — and the numbers are not close.** Full arithmetic in
[`radar_realtime_budget.md`](radar_realtime_budget.md); the summary **[calc]**:

| Stage, per frame at 20 Hz (12 TX × 16 RX × 16 loops × 256 samples) | Cost |
|---|---|
| Range FFT (3072 × 256-pt) | 629 MFLOP/s |
| Doppler FFT (49 152 × 16-pt) | 315 MFLOP/s |
| Angle FFT (4096 × 256-pt) | 839 MFLOP/s |
| **Total classic DSP** | **≈ 1.8 GFLOP/s** |
| Orin Nano GPU FP32 peak | ≈ 1.3 TFLOP/s → DSP is **~0.14 % of peak**, ~1–2 % at realistic efficiency |
| Memory traffic (≈8 passes over 3 MiB at 20 Hz) | ≈ 0.6 GB/s of 68 GB/s ≈ **0.9 %** |
| Lossless compression of the raw stream | **< 0.5 CPU core** [meas-derived] |

So the monitoring tier is nearly free, and **the reason to keep it is not
convenience — it is that an operator with no live product cannot tell a working
payload from a dead one, and a 30 s dwell wasted on nothing is expensive.**

What *can* overload the Orin Nano — and the harness must budget each explicitly:

* **An unbudgeted ML model on the full cube.** A 2D model on a range-Doppler map
  (~1–2 GFLOP/inference) is ~2 % of FP32 peak at 20 Hz; a 3D model over
  256 × 64 × 192 cells is easily 100–500 GFLOP/inference = **2–10 TFLOP/s at
  20 Hz, i.e. beyond the device** [calc]. And published radar denoising networks
  are far from real time to begin with: 0.26 s per 10-s sample (RTX 3090) for the
  self-supervised denoise+classify network; the "hybrid ~1.7 s" figure previously
  cited traces to no paper — the real number is 3.719 s per 10-s sample on a
  1080 Ti, dominated by CPU-side RoI selection **[prim]**.
* **Naive ingest.** 63 MB/s through a socket with per-frame allocation and two
  copies costs more than the FFTs do.
* **No priority separation.** The Orin Nano has **no DLA and no PVA** **[prim,
  DS-11105-001 v1.5 — absent by product spec, present in the AGX Orin
  datasheet]** — every model and every FFT contends for one 1024-core GPU.
  Without stream priorities and a drop policy, an ML experiment starves the
  live tier.

⇒ **Design rule: ML is a first-class pipeline stage, and its deadline is the
window or the dwell — not the frame.** That distinction is what makes it
affordable. Frame-rate inference at 20 Hz would be brutal; a decision every 1–2 s,
or once per dwell, is cheap:

| Cadence | A 10 GFLOP model | A 500 GFLOP model | A model taking 0.26 s wall time |
|---|---|---|---|
| every frame (20 Hz) | 200 GFLOP/s — tight | impossible | impossible |
| every 1 s (sliding window) | 10 GFLOP/s ≈ 0.8 % of peak | 500 GFLOP/s — too much | 26 % GPU duty |
| once per 30 s dwell | negligible | 16.7 GFLOP/s ≈ 1.3 % of peak | < 1 % GPU duty |

So the pipeline ends in ML by design: classical DSP folds every frame, a sliding
window feeds a classifier every 1–2 s, and the **presence decision plus the vital
signs are committed at window or dwell granularity** — carrying
`decision_age_ms` so the operator can see how fresh the answer is. What stays
non-negotiable is *independence*: the recorder never waits for the model, a model
that hangs or OOMs is invisible to the record, and every inference is stamped with
its model hash so the same decision can be reproduced offline (§B.3).

### C.2 "The idea of the radar pre-processing was to lower CC overhead"

That trade is real but it is **a link-bandwidth trade, not a CPU trade** — and
paying it in DSP-side firmware is expensive:

* The Orin has the compute headroom (C.1). It does not need the help.
* TI's cascade real-time demos are de-facto EOL (last SDK release Dec 2019, the
  demo team disbanded per TI staff) and Radar SDK 3.7/3.8 does not work with the
  AWR2243 cascade kit **[prim, TI FAQ + E2E staff answers]** — so TDA2-side
  processing means owning a firmware project on a dead SDK, in exchange for
  *losing* the ability to change the algorithm and losing raw for ML. TI's own
  supported evaluation flow for this EVM is raw capture to SSD plus offline
  MATLAB post-processing.
* What the TDA2 *is* genuinely needed for: **getting the data off the board at
  all.** 20 Hz × 3.00 MiB = **62.9 MB/s = 503 Mbit/s**, ~50 % of a 1 GbE link
  **[calc]**, on a path TI does not recommend for raw.

⇒ Keep the TDA2 doing what it is good at (reliable raw capture to its SSD), let
the Orin do the DSP, and use a *reduced* Ethernet tier for live-only purposes.
Reduce by decimation and coarse beamforming — **never by selecting range-angle
cells**, because in-flight selection is the one decision that cannot be undone
offline.

### C.3 "Data has to be saved — compressed raw radar would be amazing"

It is achievable, and here are **measured** numbers rather than hopes
(synthetic cascade frames with realistic strong static clutter, complex int16;
full method and tables in [`radar_dataset_and_storage.md`](radar_dataset_and_storage.md)) **[meas]**:

| Transform → codec | Full-scale capture | Typical (peak ~12 bits) | Quiet scene (~10 bits) |
|---|---|---|---|
| raw int16 → zstd-1 | 1.00× | 1.19× | 1.37× |
| byte-plane split → zstd-1 | 1.02× | 1.81× | 1.92× |
| slow-time delta → zstd-1 | 1.29× | 1.72× | 2.50× |
| **slow-time delta + byte-plane → zstd-1** | **1.46×** | **1.99×** | **2.67×** |

Three findings worth carrying into the design:

1. **Plain zstd on raw int16 is nearly worthless** (1.0–1.4×) — the data is
   noise-dominated. Ratio comes from the *transform*, not the codec.
2. **The best lossless transform is a slow-time (chirp-to-chirp) difference,
   which is also the classic static-clutter canceller.** The archive format and
   the first processing step are the same operation — a rare free alignment.
3. **Lossless realistically buys 1.5–2.7×.** Beyond that you need controlled
   loss, and it can be budgeted honestly: dropping k LSBs gives 1.75× at 2.3 µm
   of added displacement noise (k=4), 4.2× at 9.4 µm (k=6), 8.0× at 18.9 µm
   (k=7) — against a cardiac target of ~100 µm **[meas + calc]**. Lossy is
   therefore *defensible*, but it must be a recorded, per-dwell decision with the
   error budget stated, and the archival tier must stay lossless.

The resulting sortie economics are encouraging **[calc]**: a 30 s dwell is
1.9 GB raw, ~950 MB at 2× lossless; **30 dwells ≈ 57 GB raw / ~28 GB
compressed** — comfortably inside one NVMe. The write-rate floor: the devkit
M.2 is PCIe Gen3 ×4, and community measurements span ~100 MB/s (poor drive /
×2 slot) to ~700–800 MB/s (Gen3 ×4 TLC drive) sustained sequential write —
200–350 MB/s is a conservative planning floor, not a measured devkit property,
and even the worst observed case clears the ~32 MB/s stored rate with 3×
headroom. The chosen drive must be bench-verified post-SLC-cache (M2)
**[prim, carrier spec + community fio results]**.

### C.4 "No dataset like this exists on the internet"

Verified, and the gap is narrower and sharper than "nothing exists" **[code,
from the community dataset index + dataset READMEs]**:

| What exists | Examples |
|---|---|
| Cascade (12 TX/16 RX, TIDEP-01012) **raw ADC** datasets | Radatron (ECCV 2022: 152K frames / 4.2 h, stereo camera + 16K manually annotated); RaDelft (2024: lidar-supervised detector training on the same MMWCAS hardware); ColoRadar (AWR2243 cascade + AWR1843, pose-only ground truth); Gao's carry-object and automotive sets **[prim, the datasets' own papers]** |
| mmWave **vital-sign** datasets | 60 GHz child vital signs (raw ADC), 24 GHz GUARDIAN (IQ), the Twente 10-participant FMCW set with Polar H10 ground truth (40–160 cm standoff only), the clinical Erlangen sets (Task Force Monitor reference), and a 2026 age-balanced referenced set. The MMWCAS cascade itself has produced SCG-validated chest measurements at 3–4 m (arXiv:2411.09201) — but no public dataset |
| **Drone-mounted** radar datasets | ODA (24 GHz, obstacle avoidance, processed only) |
| **What does not exist** | A **drone-borne, cascade-MIMO, raw-ADC, human-and-vital-sign dataset with synchronised flight state, pose, and clinical ground truth.** No overlap of those five properties was found anywhere — and the 2026 literature pass reinforced it: no public mmWave vitals dataset approaches the 5–30 m UAV geometry. |

That is a real, publishable contribution — and it raises the bar on the harness,
because a dataset is only valuable if it is *interpretable by strangers*. Hence
the format work: documented schemas, hashed configuration, per-frame provenance,
a dataset card, and readers in three languages. Adopting the community's
ground-truth convention (a Polar-H10-class chest strap) is a deliberate choice so
the data is comparable to what already exists.

---

## Part D — Deliverables, phases, exit criteria

Phase 10 is done when a stranger can attach a pipeline and trust the data.

| Step | Deliverable | Exit criterion |
|---|---|---|
| **10.0** | Bench feasibility: which data path works (§transport §A), whether SSD capture and Ethernet streaming coexist, HTE edge timestamping proven, drop-rate measured, the APLL/recalibration phase-step test, coherence across stop/start. `fake_radar.py`. | A written go/no-go per data path, backed by measurements — before integration is committed. |
| **10.1** | `cc-radar` control plane + dwell FSM + **airborne-inhibit interlock** + `cc-radar-sync` + the manifest/reference. Landed/tripod only. | Companiond starts and stops a real capture from the RC switch; the interlock's truth table is 100 % host-covered; every fault drill passes; dataset opens Clean. |
| **10.2** | `cc-radar-store`: compressed raw shards + Parquet index + all context channels + ground-truth ingest. `radar-inspect`. | A 30-minute session of dwells recorded with **zero unexplained frame gaps**, measured compression ratio, verified checksums, and a reported coherent-segment length per dwell. |
| **10.3** | `cc-radar-pipe` + `radar-replay` + the three language stubs + the live tier + `CC_VITALS_REPORT` + PX4 relay. | The same trivial pipeline produces byte-identical output in Rust, Python and MATLAB, offline and online, on a committed fixture; the report reaches the handset/GCS within the ELRS budget. |
| **10.4** | Dataset v1 publication package: dataset card, format spec, readers, a baseline (not necessarily good) reference pipeline, privacy review. | A stranger reproduces a figure from the dataset using only the published documents. |
| **11+** | Algorithms and ML, developed against the harness. | Out of scope here — see the survey. |

---

## Part E — Decisions

| # | Decision |
|---|---|
| **R1** | Phase 10 delivers a harness. No vital-signs or detection algorithm is part of it. |
| **R2** | Two data paths by design: bulk raw for the record, a reduced tier for live use. They are joined by frame index + SYNC ledger, never by timestamps alone. |
| **R3** | The live tier stays — it is ~2 % of the GPU and it is the only way an operator knows the payload works. But it is *derived*, never the record. |
| **R4** | Reduce the live tier by decimation and coarse beamforming, never by selecting range-angle cells. |
| **R5** | ML is a pipeline stage, not an afterthought: the pipeline **ends** in a presence decision (classifier or classical, declared per pipeline), and vital signs are emitted only when presence is asserted. Its deadline is the window/dwell, and it reports `decision_age_ms` rather than blocking. The recorder never waits for it. |
| **R6** | Raw is stored compressed, losslessly by default (transform + zstd), with an optional per-dwell controlled-loss mode whose error is recorded in micrometres. |
| **R7** | A pipeline can never back-pressure the recorder, and must be attachable in Rust, Python or MATLAB with proven output parity. |
| **R8** | The FC relays only: start/stop in, one compact report out. Radar content never enters `cc_safety_monitor`. |
| **R9** | RC loss never starts a capture; the airborne-transmit interlock overrides every start path and fails safe to Inhibit. |
| **R10** | The report degrades to an 8-byte summary on slow ELRS modes rather than competing with flight telemetry. |
| **R11** | Every recorded row carries the provenance needed to reproduce it: profile hash, calibration hash, pipeline name/version/hash, model hash. |
| **R12** | Dataset format is tool-agnostic (flat shards + Parquet index), documented, and validated by readers in three languages. |

---

## Part H — What has been read, and what must still be measured

**Documents — all opened in the 2026-08 primary-source pass** (per-claim record
in [`radar_primary_source_findings.md`](radar_primary_source_findings.md)):
SWRU553A + SWRA574B (antennas, sync net, U8 fan-out, `EXT_DIG_SYNC`; the UG has
**no** regulatory notice) · SPRUIS6 (512 GB NVMe SSD, DP83867 GbE, 12 V/5 A;
predates the AWR2243 board revision) · AWR2243 datasheet SWRS223D + SPRACV2 +
SPRACF4C + the mmWaveLink ICD (phase-vs-temperature magnitudes, the always-on
1 s APLL/VCO cal, the cascade freeze-and-anchor recipe) · mmWave Studio Cascade
UG + `rl_sensor.h` (frame constraints; 20 Hz is TI's own worked example) ·
47 CFR Part 95 Subpart M + 15.255 + Part 5 (verbatim; the 60 GHz UAV carve-out)
· EN 302 264 + EN 300 220 + ERC 70-03 + (EU) 2025/105 (closed scoping; 25 mW
reality; the manned-rotorcraft-only airborne category) · Jetson Linux Developer
Guide + Orin datasheets (HTE/TSC, PTP, CSI, DLA/PVA) · the ELRS MAVLink
throughput tables and firmware (numbers confirmed; the LBT premise corrected) ·
plus the four literature sweeps that answered the survey's Part H questions
(range frontier 7 m; the Fraunhofer 77 GHz airborne line; the W-band clutter
corpus; the airborne-compensation portability evidence).

**Bench measurements that gate design choices — unchanged, and now sharper:**
the transport go/no-go (§transport §E, with E3 expected to fail on stock
firmware and E10 instrumented via calibration reports), the timing budget
(§transport §D.5), compression on *real* captures (§dataset §C.5), the
compute/latency budget on the actual Orin (§realtime §E) — and three
measurements the literature pass showed **nobody has published**, which this
harness can produce: rubble/debris σ⁰ at W-band, elevated-aspect human RCS at
76–81 GHz, and downwash-induced motion of clothing on a chest surrogate (D8).
