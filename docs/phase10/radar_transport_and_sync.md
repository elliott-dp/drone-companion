# Radar transport and synchronisation

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md). This is the
engineering core of the harness: how bytes get off the RF-EVM, and how every byte
acquires a trustworthy timestamp.

Sourcing tags: **[calc]** arithmetic here · **[code]** read from source/docs on
GitHub · **[corrob]** multiple independent search extracts · **[unver]** single
source or inference · **[prim]** primary document read (2026-08 verification
pass — see [`radar_primary_source_findings.md`](radar_primary_source_findings.md)).

---

## Part A — The control plane (this part is solved)

The open-source Linux control tool for exactly this board pair is readable, and
its call sequence is the specification we should reimplement in Rust **[code,
`azinke/mmwave`]**. Transport: **TCP to `192.168.33.180:5001`**, Ethernet only.

```
per-device bring-up
  master (device 0, cascading=1):  DevicePowerUp → firmwareDownload → setDeviceCrcType
                                   → rfEnable → channelConfig → adcOutConfig
  slaves (devices 1-3, cascading=2): DevicePowerUp ×3, then the same four calls
                                   applied to the slave device map

global config (deviceMap = 0x0F)
  RFDeviceConfig → ldoBypassConfig → dataFmtConfig → lowPowerConfig
  → ApllSynthBwConfig → setMiscConfig → rfInit → dataPathConfig
  → hsiClockConfig → CSI2LaneConfig → profileConfig

MIMO chirps (one TX active per chirp, 12 chirp configs)
  dev0 → chirps 11,10,9 · dev1 → 8,7,6 · dev2 → 5,4,3 · dev3 → 2,1,0

frame config
  frameConfig(masterMap, …) then frameConfig(slavesMap, …)
  numLoops = 16, framePeriodicity = 20 000 000 ticks × 5 ns = 100 ms,
  numAdcSamples = 512 int16 = 256 complex samples

capture
  ArmingTDA(captureDir, timing, allocation)
  StartFrame(device 3 → 0), wait, StopFrame(device 3 → 0), DeArmingTDA()
```

Two facts in that listing matter far beyond the control plane:

* **`CSI2LaneConfig`** — the AWR2243 ships raw ADC out over **MIPI D-PHY /
  CSI-2**, not LVDS **[code + corrob]**. That is what makes Path C below
  conceivable at all.
* **Slaves are hardware-triggered, the master is software-triggered** **[prim,
  SWRA574B §5.3/Table 6; `rl_sensor.h` makes HW SYNC_IN triggering mandatory for
  `MULTICHIP_SLAVE`]** — the host triggers slaves first, master last, and the
  master emits a `DIG_SYNC_OUT` pulse **for every frame**, so a frame-start edge
  physically exists on the board and can be *observed* (§D.3). Two useful
  numbers from the same documents: the software-trigger *start* has "several
  tens of microseconds" of uncertainty but the frame cadence itself is
  crystal-scheduled (supports the per-dwell ppm-drift fit in D.4), and the
  inter-chip `DIG_SYNC_IN` imbalance budget is ns-class (~4 ns process/voltage
  + ~0.6 ns temperature + 0/0.55 ns synchronizer quantisation, SWRA574B
  Table 1) — orders of magnitude inside the µs coherence class.

Reference defaults from the same tool **[code]**: start 77 GHz, slope
15.0148 MHz/µs, idle 5 µs, ADC start 6 µs, ramp end 40 µs, 256 samples at
8 Msps, RX gain 48 dB, RX mask 0xF, TX mask 0x7, 16 loops, 100 ms period → 80 m
max range, 30 cm range resolution. An included example profile uses slope
79.0327 MHz/µs (short range). **These are third-party example configs, not TI
factory defaults** — do not cite them as TI's.

---

## Part B — Capture modes: the 100× lever nobody mentions

Before choosing a transport, choose the *configuration*, because the data rate
spans two orders of magnitude and **the vital-signs measurement does not need the
big aperture**. Vital signs need slow-time phase stability and a clean range bin;
the 192-channel aperture exists for localisation and separation.

All rows: 16 RX (4 devices), 256 complex samples, complex int16, 20 Hz frames **[calc]**

| Mode | TX × loops | Chirps/frame | Bytes/frame | Rate | Per 30 s dwell | Buys you |
|---|---|---|---|---|---|---|
| **VITALS-1** | 1 TX × 2 | 2 | 32 KiB | **0.66 MB/s** | 20 MB | Phase on one beam. No angular separation. |
| **VITALS-3** | 3 TX × 4 | 12 | 192 KiB | **3.93 MB/s** | 118 MB | Coarse azimuth; can reject a clutter direction. |
| **VITALS-12** | 12 TX × 4 | 48 | 768 KiB | 15.7 MB/s | 472 MB | Full aperture at reduced Doppler depth. |
| **SCAN-12** | 12 TX × 16 | 192 | 3.00 MiB | **62.9 MB/s** | 1.9 GB | Full imaging cube; the cueing mode. |
| Research reference | 12 TX × 61 | 732 | 11.4 MiB | 240 MB/s | 7.2 GB | (the config used by the published cascade datasets **[code]**) |

Consequences, and they are large:

1. **A VITALS-1/3 dwell streams live with room to spare** (0.7–4 MB/s against
   the TDA2 producer's real ~40–68 MB/s TFDTP ceiling — see Path B). The entire
   "can we get real-time data to the Jetson" problem *disappears* for the
   vital-signs use case. Only the imaging mode is hard.
2. **Storage stops being scary**: 30 dwells of VITALS-3 ≈ 3.5 GB before
   compression **[calc]**.
3. The operating concept follows the data: **SCAN to cue, VITALS to confirm.**
4. Open question that must be answered on the bench: **does re-configuring
   between modes break phase coherence?** Almost certainly yes across a
   `StopFrame`/`StartFrame`, so a dwell is one mode, start to finish, and mode
   switches happen *between* dwells. Record the mode per dwell.

---

## Part C — The four possible data paths

### Path A — TDA2 captures to its own SSD (the reliable record)

TI-supported, and the only path TI recommends for raw **[prim: SPRUIS6 §1;
mmWave Studio Cascade UG §8; TI staff on E2E 953863 "we don't recommend"
Ethernet for raw, and E2E 1174366 confirming the shipped firmware writes raw
only to the SSD]**. Files land in `/mnt/ssd/<capture>/` as `*_data.bin` +
`*_idx.bin` per device, retrieved afterwards over the network **[code]**. A
12-bit packed capture mode exists (`*_packed_data.bin`) — a free 25 %
bandwidth/storage reduction worth evaluating for VITALS dwells **[prim]**.

The index is what makes the path scientifically usable **[code, `mmwave-repack`]**:

```
header (24 B):    tag u32 · version u32 · flags u32 · numIdx u32 · dataFileSize u64
per frame (48 B): tag u16 · version u16 · flags u32 · width u16 · height u16 ·
                  pitchOrMetaSize[4] u32 · size u32 · timestamp u64 (ns) · offset u64
```

* Per-frame **nanosecond timestamp, byte offset and size** — enough to rebuild the
  slow-time axis without assuming uniformity.
* **No frame sequence number.** Drops are inferable only from timestamp deltas,
  and the four per-device files can de-align. The drop rate is **strongly
  frame-periodicity dependent** **[prim, E2E 934840 with TI engaged]**: ~1.8–3.3 %
  at a 4 ms frame period, falling to **~0.05–0.1 % at 50–100 ms periods** —
  i.e. the 20 Hz VITALS regime sits in the sub-0.1 % band, and only SCAN-like
  fast framing sees percent-level loss. TI never specifies a "normal" rate; it
  treats drops as a throughput symptom and ships the idx file to detect them.
  There is also a known last-frame-lost issue in finite framing mode. Given that
  a single dropped frame injects a 2πk ambiguity into respiration unwrapping,
  gap detection is a correctness requirement — hence §D, and E2 should sweep
  periodicity.
* `flags` semantics are undocumented to us; whether they carry overflow/error
  status is a bench question worth answering early because it would make drop
  detection trivial.

**Verdict: build on this first.** It is the record.

### Path B — TDA2 streams over 1 GbE (the live tier)

The DSP-EVM's GbE (PHY DP83867) streams captured data to a host, and the stock
`cascade_radar_capture_only` usecase **already ships with `Network_tx`** — TFDTP
or TCP/IP selectable at the serial console, received by the shipped
`network_rx.exe`, one radar frame split into 4 TFDTP frames by channel id
(Master, Slave1–3) — no code modification needed **[prim, VSDK 3.08 usecase
docs + TI on E2E 953863]**. TFDTP is supported *only* on IPU1_1 (its EDMA
PaRAMs are statically allocated there), which is why `NDK_PROC_TO_USE=ipu1_1`
owns the Ethernet port. Against that: TI does not recommend raw over network,
and the SDK is de-facto EOL (last release Dec 2019, demo teams disbanded) and
not AWR2243-firmware-compatible — Radar SDK 3.7/3.8 does not work with the
AWR2243 cascade kit, and the published AWR2243 patch has open user-reported
failures **[prim, TI FAQ E2E 925288 item 13]**.

**The doc's earlier "~110 MB/s practical on 1 GbE" ceiling was wrong for this
producer** **[prim, VSDK NetworkTools UG perf tables]**: TFDTP TX on the IPU1_1
M4 does ~320 Mbit/s at 42 % CPU (~550 Mbit/s observed in TI's radar datasheet),
plain NDK TCP on the M4 collapses to ~2 MB/s, and NDK on the A15 manages
~60 MB/s. Budget the TDA2 producer at **~40–68 MB/s best case, TFDTP-only,
lossy-UDP semantics** — fine for VITALS-1/3 (0.7–4 MB/s), marginal for
VITALS-12, out of the question for SCAN-12.

**Verdict: viable *for the reduced modes* (§B), which is all the live tier needs.**
The unknown that must be tested first: **can the TDA2 capture to SSD and stream
simultaneously?** The primary sources say *probably not without custom
firmware*: the SSD DAQ path is a Linux-SDK application while the TFDTP streaming
usecase is a separate BIOS-build image — the two were never shipped combined,
and TI states live Ethernet output otherwise means writing your own TDA2
application **[prim, E2E 1174366]**. E3 stands as the falsification test, but
plan for the fallback: the live tier runs only in modes where the Orin's own
copy is sufficient, or Path C is needed.

### Path C — CSI-2 straight into the Jetson (bypass the TDA2)

The AWR2243 emits raw ADC on MIPI CSI-2, and TI's own guidance for CSI-2 capture
is "connect an external processor (TDAxx or FPGA or other) and write your own
application" **[corrob]**. The Orin Nano is such a processor **[prim,
DS-11105-001 v1.5 + DG-10931-001 v1.5]**: the module has 8 D-PHY 2.1 lanes as
**four independent 2-lane CSI ports** (CSI0–3, each with its own clock;
combinable into two 4-lane groups CSI0+1 / CSI2+3), 2.5 Gbit/s per lane, up to
16 virtual channels, RAW6–RAW16 input formats — exactly four ports for four
devices *at module level*. The devkit carrier, however, exposes **only two** of
them (J20 fixed 2-lane, J21 2- or 4-lane), so four coherent radar streams need
a custom or third-party carrier. Control without the TDA2 is *already proven*
in open source — `pyRadar` configures an AWR2243 over SPI via an FTDI USB
bridge, no mmWave Studio **[code]**.

What makes it hard, honestly:

| Obstacle | Reality |
|---|---|
| NVCSI/VI expects a camera | A radar frame must be declared as a synthetic RAW "image" with a stub V4L2 subdev and a custom device tree. This is far stronger than a generic community pattern: **RidgeRun ships a working AWR2243-over-CSI-2 V4L2 driver for Jetson** (Xavier NX, JP5.0.2 — each chirp declared RAW8 2048×1, `v4l2-ctl` streaming, up to 2 radar devices, SPI/I2C control), and an AGX Orin r36.4.4 bring-up exists on the NVIDIA forums with NVIDIA staff assisting **[prim]**. The 10.0 spike starts from known-good device trees, not from zero. NVIDIA's own camera bring-up chapter documents the DT/V4L2 path but covers cameras only — community-proven, not NVIDIA-supported. |
| Four devices | Four CSI-2 sources need four ports/lane groups and must be captured coherently; lane budget and per-port deskew need checking against the module's limits. |
| Control path | SPI + FTDI (proven), or SPI from Jetson GPIO. Firmware download per device still required. |
| Sync | You inherit the master/slave SYNC fan-out — an advantage: the frame edge is right there. |
| Risk | Highest of the four. A driver/device-tree project with real chance of failure. |
| Payoff | Also the highest: the Orin owns raw *directly*, at full rate, with no EOL SDK, no SSD offload step, and one clock domain. It would make the harness dramatically simpler. |

**Verdict: the strategic option.** Do not gate Phase 10 on it. Do spend a
time-boxed feasibility spike in 10.0, because if it works the rest gets easier
forever.

### Path D — DCA1000EVM

Wrong tool: DCA1000 captures **LVDS** from single-chip EVMs, while the cascade
board is CSI-2 and four-chip-synchronised **[corrob]**. Four DCA1000s would not
share a coherent trigger. **Dismissed**, recorded here so nobody re-proposes it.

---

## Part D — Synchronisation: the actual hard part

Three clocks exist — PX4's, the Jetson's, and the radar/TDA2's — and the harness
must relate them at two *different* accuracy classes.

### D.1 The two accuracy classes (do not conflate them)

| Class | Purpose | Requirement | Mechanism |
|---|---|---|---|
| **Context** (ms) | Join a dwell to flight state, position, mission identity, operator display | ~1–10 ms | `cc-timesync` + `cc_receive_time_ns`, already shipped |
| **Coherence** (µs) | Relate radar phase to platform motion; make the slow-time axis exact | ~10–250 µs | Hardware edge timestamping (§D.3) |

Why the second class is not optional: cancelling a 100 Hz disturbance to −20 dB
needs 159 µs of alignment, −30 dB needs 50 µs, −40 dB needs 16 µs; holding
translational residual under 50 µm at 0.2 m/s needs 250 µs; and 10 ppm of
uncorrected clock skew accumulates **300 µs over a 30 s dwell** **[calc]**. MAVLink
TIMESYNC is millisecond-class, so the FC link cannot serve this class at all.

### D.2 PTP is not available on this module

The Ethernet PHY on the DSP-EVM side supports IEEE 1588 — SPRUIS6 §2.5.3
confirms the DP83867 provides 1588 start-of-frame detection and a synchronous
Ethernet clock output **[prim]** — but **PTP is supported on Jetson AGX Orin
and *not* on Orin NX / Orin Nano** **[prim: NVIDIA staff statement on the
developer forum; `ethtool -T` on Orin Nano returns "PTP Hardware Clock: none"]**.
The precise reason: the GbE controller on Orin NX/Nano is an on-**module**
Realtek RTL8111H without hardware timestamping, so no carrier using the module's
Ethernet can ever add PTP — the only retrofit is a separate PCIe/M.2 NIC
(i210-class). So the elegant "PTP over the radar LAN" answer is closed on this
hardware. If the platform ever moves to AGX Orin (integrated MGBE, hardware
PTP proven), revisit — it would be the cleanest solution.

### D.3 What replaces it: hardware edge timestamping via Tegra HTE

Jetson has a **Hardware Timestamp Engine** (HTE; from JetPack 6.0 it replaces the
older GTE and it explicitly covers AGX Orin, Orin NX **and Orin Nano**), which
timestamps state changes on AON-domain GPIOs and LIC interrupt lines *in
hardware* **[prim, Jetson Linux Developer Guide GTE chapter (r36.4/r38.2.1) +
upstream `tegra-hte` docs]**. That is precisely the primitive needed. Details
that were unknown at design time and now aren't **[prim]**:

* Timestamps come from the **TSC system counter at 31.25 MHz → 32 ns
  resolution**, converted to nanoseconds by the driver — a hardware timebase
  separate from `CLOCK_MONOTONIC`; correlating the two domains is
  `cc-radar-sync`'s job and belongs in the ledger.
* Userspace path: GPIO v2 character-device ioctls with
  `GPIO_V2_LINE_FLAG_EVENT_CLOCK_HTE`. No NVIDIA sample app exists; NVIDIA's own
  guidance is "write a custom application".
* **Devkit pin reality:** only two AON-domain pins reach the 40-pin header —
  pin 27 (`I2C0_SDA`, PDD.00) and pin 28 (`I2C0_SCL`, PCC.07) — and both carry
  1.5 kΩ pull-ups to 3.3 V as I²C lines, which the tap buffering must drive
  against. The module's other AON GPIOs (GPIO03–06, GPIO10) do not reach the
  devkit header; a custom carrier would free them.

The key simplification — and it removes the scariest hardware risk:

> **For timestamping you only need to *observe* the frame edge, not drive it.**
> The cascade already generates a master→slave SYNC fan-out through the U8
> LMK00804B 1:4 buffer (one output loops back to the master's own `DIG_SYNCIN`
> for delay matching). Tapping that signal to a Jetson AON GPIO is a wire plus
> level-shifting/buffering, and it changes no radar behaviour. *Driving* an
> external trigger turns out to be **provisioned by TI, not a rework**: host
> connector **J4 pin 64 `EXT_DIG_SYNC` is a documented alternative input to the
> U8 fan-out** (SWRU553A Table 2), so platform-triggered framing is an input-path
> selection, not soldering at the buffer **[prim]**. A candidate observation tap
> without touching U8 also exists (J4 pin 83 `AWR_1_SOP1_SYNCOUT`, muxed with
> SOP1 — the UG's pin-direction annotations are inconsistent, so bench-verify
> direction). Observe in Phase 10; consider driving in Phase 11.

Both must be bench-verified: which net to tap, its logic level and drive
strength, whether buffering is needed, and which Jetson pins are HTE-capable AON
GPIOs.

### D.4 The three-way frame ledger

Recording all three and reconciling them offline is what makes the dataset
trustworthy:

| Source | Gives | Weakness |
|---|---|---|
| **HTE edge ledger** — `(edge_index, cc_mono_ns)` | Exact frame-start times in the Jetson clock, sub-µs | Says nothing about whether the frame's *data* survived |
| **`*_idx.bin`** — `(frame_index, tda_ts_ns, offset, size)` | What was actually written, in TDA time | No sequence number; per-device files can de-align |
| **Live tier** — `(frame_seq, cc_receive_time_ns)` | Arrival, in CC time | ms-class jitter; may be shed |

Reconciliation rules (implement in `cc-radar-sync`, and record the outcome, not
just the inputs):

1. Edge count between dwell markers is the **authoritative frame count**.
2. `idx` entries are matched to edges by monotonic order; a missing entry is a
   **capture drop**; an edge with no entry and no successor is a **truncation**.
3. Per-device disagreement in count is a **de-alignment fault**, recorded
   per device, never silently repaired.
4. `tda_ts_ns` versus edge times yields the radar↔CC **offset and drift (ppm)**
   per dwell — fit it, record the residual, and expose a quality state
   (Locked / Degraded / Unlocked) with the same discipline `cc-timesync` uses.
5. **Never interpolate a missing frame.** Mark the gap; downstream segments on it.

Fallback when no edge wire exists: estimate the alignment from `idx` deltas plus
live-tier arrivals, and stamp the dwell `coherence = estimated`. Usable for
context, explicitly not for phase compensation — and the dataset must say so.

### D.5 Platform-motion reference

Independent of radar, and on the *radar bracket* rather than the FC (the lever arm
between the antenna phase centre and the IMU is what defeats naive attitude
compensation — 20 cm × 0.5° = 1.75 mm = 5.8 rad at 79 GHz **[calc]**):

* A **≥1 kHz IMU on the bracket**, timestamped in the same HTE-anchored domain.
  The FC's ~50 Hz vibration data is sub-Nyquist for the 80–250 Hz rotor band.
* The antenna-phase-centre→IMU extrinsic, calibrated to <1 mm, recorded as
  dataset metadata.
* Rotor state proxy: commanded `actuator_output` (20 Hz) + FC vibration metrics.
  **Honest gap: this airframe has no ESC telemetry**, so true RPM is unavailable —
  the same reduced-observability caveat `motor_balance` already documents. Alias
  position is `f_vib mod f_frame`, so *predicting* it needs rotor state.
* A **static scene anchor** (corner reflector, or a designated clutter cell) — per
  the survey, the scene is a better phase reference than the IMU.

---

## Part E — Bench tests that gate the design (Phase 10.0)

Each has a pass/fail criterion, and each can invalidate a path. Run them before
writing integration code.

| # | Test | Pass criterion |
|---|---|---|
| E1 | Control plane in Rust reaches parity with the reference tool | Configure + capture + stop, byte-identical register writes to a captured trace of the reference tool |
| E2 | SSD capture drop rate, per mode, 30 s dwells ×20 | Measured and recorded; VITALS modes ideally 0 drops; SCAN-12 characterised |
| E3 | **Simultaneous SSD capture + Ethernet stream** | Either works (record the sustained rate) or is proven impossible — this decides the live tier's producer |
| E4 | `*_idx.bin` `flags` semantics under induced drops | Determine empirically whether drops are flagged |
| E5 | Are the four devices' `idx` timestamps from one TDA2 clock? | If yes, cross-device alignment is a lookup; if no, it is an estimation problem |
| E6 | SYNC edge tap → Jetson AON GPIO → HTE | Edge timestamps captured for a full 30 s dwell with **zero missed edges**; jitter reported |
| E7 | Edge count vs `idx` count vs live-tier count, with induced drops | Reconciliation detects every induced drop and no false ones |
| E8 | Radar↔CC drift over 30 s and over 30 min | ppm fitted and stable; residual reported |
| E9 | Phase coherence across `StopFrame`/`StartFrame` | Determine whether two captures can be concatenated coherently — decides whether a dwell may straddle a capture |
| E10 | **1 Hz APLL/VCO recalibration phase step** — static corner reflector, ≥60 s at 20 Hz | The mechanism is now documented rather than suspected **[prim, SPRACF4C §3.1/3.2/6; ICD Table 5.36]**: APLL + synth-VCO runtime cals run at 1 s periodicity in inter-frame idle, are always-on and **cannot be disabled** (`CALIBRATION_PERIODICITY=1` is an invalid "stop" value — do not use as a freeze); all *other* runtime cals (LODIST/PD/TX-power/RX-gain — the documented source of abrupt gain/phase jumps, and a 2 dB step per RX gain code) **are host-disableable**, and the ICD explicitly recommends disabling them in cascade for phase synchronisation. TI never quantifies the APLL/VCO step, so this test measures the one undocumented residual. Method upgrade: set `ENABLE_CAL_REPORT` so every 1 s cal event arrives as a timestamped async report (with die temperature and whether hardware was updated) — correlate phase steps to logged events instead of blind 1 Hz spectral search, and record the reports in `device_state` **[prim, priority test]** |
| E11 | Mode-switch behaviour and reconfigure latency | Time to switch SCAN→VITALS; whether it is safe within a mission |
| E12 | Path C spike (time-boxed) | A single AWR2243's CSI-2 frames captured into the Orin, or a written negative result with the specific blocker |

---

## Part F — Failure modes and their required behaviour

| Failure | Required behaviour |
|---|---|
| Radar board absent / TCP refused at start | Mission logging unaffected; payload state FAULT; report still sent with `payload_state=FAULT` |
| Board reboot mid-dwell | Seal the dwell `radar_reboot`; do not silently continue |
| SSD full on the board | Seal, surface, stop starting new dwells; keep the live tier |
| Network partition mid-dwell | Live tier degrades; SSD capture continues; reconciliation records the missing live rows |
| Frame drops above threshold | Dwell marked `coherence_broken` with the gap list; **not** discarded — a partially usable dwell is still data |
| Profile or calibration hash mismatch | Refuse to capture; record the reason |
| HTE edges missing | Dwell `coherence = estimated`; a loud warning; still recorded |
| RC lost | Never start; hold or stop per config |
| Airborne without authorization | Transmit inhibited, logged, reported |
