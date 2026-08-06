# Phase 10 (proposed) — TI MMWCAS cascaded radar: a human-detection & vital-signs sensing payload

> **Status: design proposal. No code has shipped for this phase.**
>
> **Purpose:** locate people and estimate respiration / heart rate from the air —
> a search-and-rescue and disaster-response **sensing payload**. It is *not* a
> navigation, obstacle-avoidance or collision-prevention sensor, and nothing it
> produces ever reaches PX4's flight logic.
>
> **Sourcing discipline:** every number below is tagged
> **[calc]** (arithmetic done here, independently reproducible),
> **[corrob]** (corroborated across multiple independent search extracts, but the
> primary document was **not** read), or **[unver]** (single source or inference —
> treat as a hypothesis). This session's egress policy blocked `ti.com`,
> `e2e.ti.com`, `arxiv.org`, `ieeexplore.ieee.org`, `pmc.ncbi.nlm.nih.gov`,
> `ecfr.gov` and `eur-lex.europa.eu`, so **no primary PDF was opened.**
> §H is the verification queue: the exact documents a human must read before any
> **[corrob]** or **[unver]** number becomes a design constant.

**Hardware in scope:** MMWCAS-RF-EVM (4 × AWR2243, 12 TX / 16 RX, 76–81 GHz) +
MMWCAS-DSP-EVM (TDA2Sx capture card, 1 GbE, on-board PCIe SSD), on the Jetson
Orin Nano that already runs `companiond`.

---

## Part 0 — Three findings that gate this phase (read these first)

### 0.1 In the US, airborne 76–81 GHz radar is prohibited by rule — and the rule mandates an interlock

47 CFR § 95.3333, quoted identically by two independent extracts of the CFR text
**[corrob]**:

> "Notwithstanding the provisions of § 95.3331, 76-81 GHz Band Radar Service is
> prohibited aboard aircraft in flight. Aircraft-mounted radars shall be equipped
> with a mechanism that will prevent operations once the aircraft becomes
> airborne."

§ 95.3331's permitted-use list is closed — vehicular radars, and fixed/mobile
radars in airport air operations areas "including … aircraft-mounted radars for
ground use only" **[corrob]**. A drone-mounted people-sensing radar is in none of
those categories. The 60 GHz escape hatch is also closed: § 15.255 excludes
operation on aircraft where there is little RF attenuation by the fuselage, and
reportedly names unmanned and model aircraft **[unver]**. In the EU the same
place is reached differently: the harmonised 76–81 GHz SRD designation
(ERC/REC 70-03; ETSI EN 301 091 for 76–77 GHz, EN 302 264 for 77–81 GHz) is
scoped to ground-based vehicle and infrastructure radar, so a UAV payload falls
*outside* the licence exemption rather than being banned by name **[unver]**.

Three consequences, and they restructure the whole phase:

1. **Lawful airborne operation needs an experimental/research authorization**
   (FCC Part 5 in the US; national administration authorization in the EU) — this
   is a licensing question, not a compliance-test question. It has a lead time and
   it belongs at the front of the schedule, not the end.
2. **Bench, ground, tripod and landed operation is where the engineering lives** —
   and, as §A shows, it is also where the physics is most likely to work. That is
   a happy alignment, not a consolation prize.
3. **The airborne-inhibit interlock is a first-class feature of this design**
   (§C.2), because the rule literally requires the mechanism. It fails safe:
   unknown airborne state ⇒ transmit inhibited.

### 0.2 76–81 GHz cannot find buried people. This is a surface / line-of-sight sensor.

Attenuation through rubble is prohibitive at mmWave — concrete alone is reported
at ~26 dB/m at 400 MHz rising to ~66 dB/m at 2.4 GHz, and it worsens with
frequency **[corrob]**. Every fielded rubble/avalanche victim-search system uses
150–650 MHz, L-band ~1.15 GHz, or 1–4 GHz UWB (e.g. NASA/DHS FINDER; airborne
avalanche GPR-SAR at 1–4 GHz; drone UWB through wet snow) **[corrob]**. At
76–81 GHz, clothing costs < ~6 dB one-way and light foliage ~0.4 dB/m one-way at
73 GHz **[unver]**.

So the honest capability statement is: **this payload finds people on or near the
surface — in the open, or under light vegetation, tarps and clothing. It cannot
find people under rubble or snowpack.** If buried-victim search is the mission
requirement, this is the wrong band and the wrong hardware, and no amount of
software fixes that.

### 0.3 Airborne vital signs at 79 GHz is unproven research; human *detection* is not

| Capability | Status |
|---|---|
| Detect a **moving** person at 5–30 m from the air | Well inside the hardware's means. At these ranges the system is not SNR-limited; the binding constraints are elevation resolution, ground clutter at high depression angles, and TDM velocity ambiguity (§A.7). |
| Estimate **respiration** on a stationary subject at short range, from a static mount | Established in the literature and in TI's own reference material — but at **0.3–1.5 m**, not tens of metres **[corrob]**. |
| Estimate **respiration** from a hovering UAV | Demonstrated at **7.3–24 GHz** (UWB / CW), not at 76–81 GHz. In the one airborne result found, breathing survived both hovering and moving flight; **heartbeat was recovered only while hovering** **[unver]**. |
| Estimate **heart rate** at 79 GHz from a hovering UAV at standoff | **No published demonstration found.** Treat as unproven research. The 79 GHz carrier makes identical platform motion produce **3.3–10.8×** more phase than the 7.3–24 GHz systems that did succeed **[calc]**. |

The design consequence is not "don't build it" — it is "**build the instrument
that can prove or disprove it from recorded data**", which is what §B is about.

---

## Part A — The measurement, in numbers

This is the budget every other decision follows from.

### A.1 Phase ↔ displacement

Round-trip phase: `φ = 4π·d/λ`, so `d = λ·Δφ/(4π)`. All **[calc]**, c = 299 792 458 m/s:

| f | λ | 4π/λ | µm per radian | wrap spacing λ/2 | one-sided unambiguous λ/4 |
|---|---|---|---|---|---|
| 76 GHz | 3.945 mm | 3.186 rad/mm | 314 µm | 1.972 mm | 0.986 mm |
| 77 GHz | 3.893 mm | 3.228 rad/mm | 310 µm | 1.947 mm | 0.973 mm |
| 79 GHz | 3.795 mm | 3.311 rad/mm | 302 µm | 1.897 mm | 0.949 mm |
| 81 GHz | 3.701 mm | 3.395 rad/mm | 295 µm | 1.851 mm | 0.925 mm |

The scale factor moves 6.6 % across the band, so **a recorded phase history
without its carrier frequency cannot be converted to millimetres**. Chirp start
frequency, slope, ADC window and the derived λ are per-capture metadata, not
documentation.

### A.2 What wraps, and what never does

| Signal | Displacement | Phase at 79 GHz **[calc]** | Wraps? |
|---|---|---|---|
| Respiration | 1–12 mm (typically 4–12) **[corrob]** | 3.3–39.7 rad | **Always** — up to ~6 wrap crossings per traverse of a 12 mm excursion |
| Cardiac (at the point of maximal impulse) | 0.1–0.5 mm **[corrob]** | 0.33–1.66 rad pk-pk (≤ 0.83 rad one-sided) | **Never** |

Two hard consequences:

* **Respiration requires unwrapping; cardiac is a sub-radian perturbation riding
  on top of it.** Worst case the two are only ~2× apart in amplitude (a 1 mm
  shallow breath against a 0.5 mm strong cardiac impulse), best case ~60×.
* **A single dropped or duplicated frame injects an unrecoverable 2πk ambiguity**
  that propagates through the rest of the unwrapped series. Unwrapping is valid
  only while true inter-frame motion stays under λ/4 — i.e. chest-wall velocity
  below `λ·f_frame/4` = **19 mm/s at 20 Hz** **[calc]**. So the recorder must make
  gaps *detectable* (§B.3) and the estimator must **segment on discontinuities
  rather than unwrap across them**.
* Therefore: **record complex I/Q, never a pre-wrapped or pre-unwrapped scalar
  phase.** Unwrapping decisions must be revisable offline.

### A.3 The noise floor is SNR-determined, not a sensor constant

`σ_d = (λ/4π)/√(2·SNR)` = `302 µm/√(2·SNR)` at 79 GHz **[calc]**:

| Per-look SNR | 10 dB | 20 dB | 30 dB | 41 dB |
|---|---|---|---|---|
| σ_d | 67.5 µm | 21.4 µm | 6.8 µm | 1.9 µm |

Because SNR falls as 1/R⁴, **σ_d grows as R²**. Anchored to a strong near-bench
target, the same hardware degrades to tens of µm at a few metres and ~180 µm at
8 m **[unver]** — i.e. at realistic standoff the *per-look* floor is comparable
to or larger than the cardiac displacement. What rescues it is slow-time coherent
integration: 30 s at 20 Hz is N = 600 samples = **27.8 dB** of processing gain
**[calc]**. That is precisely why the dwell in §A.4 is not negotiable, and why any
claim of "sub-micron sensitivity" as a hardware property is wrong.

### A.4 Window length sets rate resolution, and therefore dwell time

Plain-FFT resolution is `1/T`, i.e. `60/T` in BPM **[calc]**:

| Window T | 12.8 s | 16 s | 25.6 s | 30 s | 60 s |
|---|---|---|---|---|---|
| Resolution | 4.69 BPM | 3.75 BPM | 2.34 BPM | 2.00 BPM | 1.00 BPM |

TI's shipped vital-signs profile uses a 50 ms frame period (**20 Hz** slow-time),
a **256-sample (12.8 s) breathing window** and a **512-sample (25.6 s) heart-rate
window**, with a heart-rate band-pass of **0.8–2.0 Hz (48–120 BPM)** and a
configured range of 0.3–0.9 m **[corrob]**. 20 Hz is 3.3× the Nyquist minimum for
a 3 Hz top end **[calc]**; the margin buys harmonic headroom and impulse-noise
interpolation.

⇒ **A heart-rate estimate needs ~512–600 consecutive, gap-free, phase-coherent
frames — a 26–30 s dwell on one subject.** That forces **hover-and-stare, not
fly-by**, and it sets the search economics: at ~30 s per dwell plus repositioning,
a sortie yields tens of dwells. **This payload is a cued confirmer, not an area
scanner.** (An imaging/moving-target pass can do the cueing; the vital-signs mode
confirms.) Note also that TI's own band tops out at 120 BPM — a tachycardic
casualty at 130–160 BPM would fall outside it, so the band is a parameter to widen
deliberately, not a constant to inherit.

### A.5 The ego-motion budget — the dominant fact of this design

Everything is expressed against the cardiac target of **0.33 rad** (0.1 mm):

| Disturbance | Magnitude | Phase at 79 GHz **[calc]** | vs cardiac |
|---|---|---|---|
| Hover station-keeping, GNSS-only | ±1.5 m **[unver]** | 4967 rad (790 wraps) | ~15 000× |
| Hover station-keeping, RTK | ±0.1 m **[unver]** | 331 rad (53 wraps) | ~1 000× |
| 1 cm of residual drift | 10 mm | 33.1 rad (5.3 wraps) | ~100× |
| Airframe vibration @ 30 m/s², 100 Hz | 76 µm | 0.252 rad | ~0.8× |
| Airframe vibration @ 30 m/s², 220 Hz | 16 µm | 0.052 rad | ~0.16× |
| **Lever arm**: 20 cm phase-centre offset × 0.5° attitude jitter | 1.75 mm | **5.78 rad** | ~17× |

Read that table carefully, because it inverts a natural assumption:

* **Vibration is second-order.** Displacement falls as 1/f², so the 100–600 Hz
  rotor band produces only ~16–120 µm even at ArduPilot's "acceptable" 30 m/s²
  ceiling **[calc from corrob VIBE levels]** — comparable to the cardiac signal,
  not dominant over it. (Aliasing still matters: an out-of-band vibration line
  lands at `f_vib mod f_frame`, so it must be *predicted*, which is why §B.3
  records rotor state.)
* **Low-frequency platform motion is the killer**, by three orders of magnitude.
* **Lever arm alone defeats pure IMU aiding.** Holding the lever-arm term under
  0.1 rad at a 20 cm offset needs attitude known to **0.0086°** **[calc]** —
  roughly 100–400× better than a multirotor EKF delivers. So:

> **Compensation must be scene-referenced first, IMU-aided second.** The phase
> reference has to come from the radar's own view of static clutter (a ground
> patch, a corner reflector, a structural return) with the IMU as a coarse aid
> and a validity check. Absolute position knowledge can never supply it.

### A.6 The timing budget — and why the MAVLink link cannot meet it

Cancelling a sinusoidal disturbance with a timing error τ leaves a residual of
≈ 2πfτ, so **[calc]**:

| Target cancellation of a 100 Hz line | −20 dB | −30 dB | −40 dB |
|---|---|---|---|
| Required radar↔IMU alignment | 159 µs | 50 µs | 16 µs |

Plus: keeping the translational residual under 50 µm at 0.2 m/s hover velocity
needs **250 µs** alignment **[calc]**; and **10 ppm** of uncorrected clock skew
accumulates **300 µs over a 30 s dwell** **[calc]** — by itself larger than the
whole budget.

Against that, MAVLink TIMESYNC is millisecond-class and `HIGHRES_IMU` tops out
around 180–200 Hz **[corrob]**. So the conclusion is structural:

> **This design has two time domains with different accuracy classes, and they
> must not be confused.**
>
> * **Context alignment (ms class)** — `cc-timesync` + `cc_receive_time_ns`, the
>   machinery that already exists. Sufficient for joining a dwell to flight state,
>   mission identity, position, and operator display.
> * **Compensation alignment (µs class)** — a *radar-local* subsystem: hardware
>   frame trigger, a PPS-referenced clock, a ≥1 kHz IMU mounted on the **radar
>   bracket** (not the FC), and per-frame edge counting latched against
>   `cc_receive_time_ns`. The FC link is not in this path and cannot be.

On the trigger: the AWR2243 supports per-frame hardware triggering
(`triggerSelect = 0x0002`, SYNC_IN rising edge, ~160 ns edge-to-air, 5 ns-LSB
programmable delay) and the cascade shares one 20 GHz LO and 40 MHz reference
with a sub-10 ns inter-device sync skew **[corrob]**. But the **MMWCAS-RF-EVM
exposes no trigger header** — the reported route is soldering to the sync-fanout
buffer **[unver]**. That makes "can we hardware-trigger this board" a
**bench task with a soldering iron**, and it is the highest-value unknown in the
whole design (§H).

### A.7 Sensing geometry (what the aperture actually buys)

**[corrob]** except where marked: 12 TX × 16 RX = 192 virtual channels, ~86
non-overlapping in azimuth; azimuth resolution ~1.35–1.4°; elevation ~19°.

| At range | Azimuth cell (1.35°) | Elevation cell (19°) |
|---|---|---|
| 5 m | 12 cm | 1.7 m |
| 10 m | 24 cm | 3.3 m |
| 30 m | 71 cm | 10 m |

Across the usable FoV that is ~89 azimuth beams against **fewer than 2 elevation
beams** **[calc]** — so instantaneously the cascade is **not a height
discriminator at all**. A supine casualty cannot be separated from the ground
plane in elevation; height must come from platform-motion synthetic aperture
(which needs cm-class pose per chirp, a much harder problem than per frame) or
from assuming targets lie on a known surface.

Also: 12-TX TDM at a 40 µs chirp gives a 480 µs effective PRI and an unambiguous
velocity of only **±2.0 m/s** **[calc]** — smaller than the drone's own ground
speed, so ego-motion aliases in Doppler and the MIMO scheme must be recorded per
capture. (DDMA buys ~10.8 dB at the *same* ambiguity, minus a small guard-band
penalty **[calc]** — it is a power trade, not an ambiguity fix.)

And a projection factor that must be recorded per dwell: **the sensed
displacement is the chest-normal motion projected on the line of sight**. A nadir
look favours a supine casualty and nulls a standing one; a face-down casualty
presents smaller, essentially uncharacterised back-wall motion. Geometry and
posture are part of the measurement, not context.

---

## Part B — What must be recorded (the irreversibility principle)

> **The governing rule: anything not recorded in flight cannot be recovered, and
> the algorithms that will consume this data do not exist yet. So the recording
> decision must assume the processing is wrong.**

### B.1 Raw-to-SSD per dwell is mandatory, not optional

This is the reverse of the conclusion you would reach for a navigation radar. The
vital-signs chain is unproven at this carrier and geometry (§0.3), so the raw ADC
capture is the **only** artifact that lets a future algorithm be tried at all.

Volumes, for the standard 12 TX × 16 RX × 16 loops × 256 samples complex-int16
frame **[calc, geometry corrob]**:

| Quantity | Value |
|---|---|
| Per device / per frame | 768 KiB |
| **Per frame (4 devices)** | **3.00 MiB** |
| At 10 Hz / 20 Hz | 31.5 MB/s / **62.9 MB/s** (503 Mbit/s) |
| 30 min at 20 Hz | ~113 GB |
| 512 GB SSD **[unver capacity]** | full in ~2.3 h at 20 Hz |
| Per-device file rollover | reported at 2047 MB ≈ 2729 frames ≈ 136 s at 20 Hz **[unver]** |

Frame loss is **normal, not exceptional** — reports include ~3.3 % loss on an
otherwise ordinary capture **[corrob]** — and the `*_idx.bin` index carries **no
frame sequence number**: a 24-byte header (`tag`, `version`, `flags`, `numIdx`,
`dataFileSize`) followed by **48-byte** per-frame records
(`tag u16`, `version u16`, `flags u32`, `width u16`, `height u16`,
`pitchOrMetaSize[4] u32`, `size u32`, `timestamp u64` ns, `offset u64`)
**[corrob; 48 B is arithmetic on those fields — an earlier draft of this document
said 56 B, which was wrong]**. So drops are inferable only from timestamp deltas,
and the four per-device files can de-align. Given §A.2, that makes gap detection a
correctness requirement, not a statistic.

### B.2 What "pre-processed" may and may not be trusted for

Two facts kill the idea that a streamed pre-processed product can be the
scientific record:

1. **The producer does not exist.** TI ships no firmware that computes and streams
   range-FFT / range-azimuth / selected-bin products from the cascade, and the
   TDA2 real-time cascade demo is EOL and not AWR2243-firmware-compatible
   **[corrob]**. The 1 GbE streaming path also has documented pathological
   behaviour (frame rate collapsing, majority-drop reports) **[unver]**.
2. **In-flight selection decisions are irreversible and will be wrong.** Choosing
   *which* range-angle cells to keep, and applying DC/clutter removal or
   beamforming weights, bakes in decisions made under exactly the ego-motion
   conditions (§A.5) that make them unreliable.

⇒ Therefore:

> **The streamed pre-processed tier is a MONITORING product** — "the payload is
> alive, it is pointed there, returns look like this" — for the operator and for
> flight-line confidence. **It is explicitly not the record.** If a pre-processed
> stream is to be scientifically usable, it must be *unselected*: all range bins
> over a coarse beam set, complex, e.g. 256 bins × 16 beams × complex f32 =
> **32 KiB/frame → 655 kB/s at 20 Hz → 2.4 GB/h** **[calc]** — cheap enough that
> selection is never worth the risk. For comparison, 8 selected bins × 192 virtual
> channels is 12 KiB/frame (246 kB/s), and a 256 × 128 magnitude heatmap is
> 128 KiB/frame (2.6 MB/s) — the magnitude map being useless for vital signs
> because it discards phase.

### B.3 The alignment scaffolding (the part that is easy to forget and fatal to omit)

Raw lands on the radar's SSD; products and flight state land in Jetson Parquet.
**Nothing joins those two domains unless it is recorded in flight.** Required, per
dwell:

| Item | Why |
|---|---|
| `capture_id` + per-device file names, sizes, checksums | binds SSD bytes to the mission record |
| **Counted SYNC edges latched on a Jetson GPIO against `cc_receive_time_ns`** | the only µs-class bridge between radar frames and CC time (§A.6) |
| Explicit dwell-start / dwell-end anchors (a commanded, recorded event) | bounds the coherent segment; survives index de-alignment |
| Radar-clock timestamp **and** monotonic frame index per frame | gap detection without assuming uniform sampling |
| Nominal frame period as an exact integer (5 ns ticks) | lets measured Δt be differenced against nominal |
| **Per-frame device state**: 4 × die temperature, RX gain, TX mask, MIMO scheme, calibration/monitoring async events, calibration-matrix hash | phase-vs-temperature drift and the reported ~1 Hz APLL/VCO recalibration are only removable offline if they sit beside the phase — and a 1 Hz artefact lands **inside the cardiac band** **[unver, and a priority bench test]** |
| **Bracket IMU at ≥1 kHz** + calibrated antenna-phase-centre-to-IMU extrinsic (<1 mm) | §A.5/§A.6; the FC's 50 Hz IMU is sub-Nyquist for the 80–250 Hz rotor band |
| Rotor state proxy: commanded `actuator_output` (20 Hz) + IMU vibration metrics | predicts the alias position `f_vib mod f_frame`. **Honest gap: this airframe has no ESC telemetry**, so true RPM is unavailable — the same reduced-observability caveat `motor_balance` already documents |
| Geometry + posture per dwell: standoff, depression angle, subject posture, surface type | the projection factor in §A.7 |

### B.4 Ground truth and negative controls — the most irreversible omission

A flight without these can neither confirm a heart rate nor bound a false-alarm
rate, and no amount of later processing fixes it:

* **A synchronized clinical reference** — chest belt and/or PPG, sampled onto the
  *same* clock as the radar frames (not "roughly the same time").
* **An in-scene corner reflector** for phase and amplitude calibration, and as the
  scene-referenced phase anchor of §A.5.
* **Empty dwells** — identical geometry, no human present. This is the direct
  analogue of the repo's existing "a benign flight must produce zero findings"
  rule, and it is what a false-alarm rate is computed from.
* **A landed / tripod control measurement in every experiment**, so airborne
  degradation is *measurable* rather than argued about.

### B.5 Where the determinism boundary goes

The existing stack proves byte-identical replay. Radar splits cleanly across that
line, and the split is better than it was for the navigation framing:

| Stage | Deterministic? | Treatment |
|---|---|---|
| Range FFT / beamforming (TDA2 or GPU) | No — not bit-reproducible across drivers, arch or library versions | **Recorded transform.** Outside the proof. |
| **Vital-sign estimation** (unwrap → clutter/DC removal → band separation → spectral estimation → confidence fusion) | **Yes, if written to the existing rules**: integer logical clock, fixed-order reductions, `libm` instead of host transcendentals | **Inside the proof.** Replayable from recorded phase history, and therefore FP-auditable exactly like the eight health algorithms. |

That is the payoff of recording complex I/Q rather than estimates: the estimator
becomes a *replayable, regression-testable* component, and `replay-mission` can
re-run every recorded dwell against a new estimator build and diff the results.

---

## Part C — Architecture on this codebase

### C.1 Crates

```
crates/
  cc-radar/          control-plane client (TCP:5001), dwell/session FSM, profile +
                     calibration hashing, *_idx.bin parser, capture reference,
                     airborne-inhibit interlock, radar shed ladder
  cc-radar-sync/     the µs-class domain: SYNC edge counting/latching, PPS, bracket
                     IMU ingest, radar↔CC offset estimate + quality
  cc-radar-ingest/   monitoring-tier frame contract: decode → validate → RadarEvent
                     fan-out → recording
  cc-vitals/         the deterministic offline/online estimator (phase → rates),
                     own registry, own FP audit          [after 10.2]
apps/
  radar-inspect/     verify radar streams, index↔blob consistency, offload checksums,
                     dwell completeness (gap-free coherent segments)
tools/phase10/
  fake_radar.py      control-plane test double + synthetic frame/phase producer
```

### C.2 The airborne-inhibit interlock (a policy gate, in the style of `cc_safety_monitor`)

Because § 95.3333 mandates a mechanism (§0.1), transmit permission is a small,
exhaustively-tested pure function — not an `if` buried in a task:

```
permit_tx(airborne_state, authorization, config) -> Permit | Inhibit(reason)
```

* `airborne_state` derives from `CC_TELEMETRY_STATE` (arming/nav/landed state).
* **Fail-safe: `Unknown` or *stale* FC state ⇒ Inhibit.** Missing data stays
  missing — the payload never assumes it is on the ground.
* Airborne transmit requires an explicitly configured authorization identifier
  (e.g. a Part 5 experimental grant reference) recorded into the manifest. Absent
  it, airborne ⇒ Inhibit, and the reason is logged and surfaced.
* Every transition is recorded in the mission's operational event log.

The gate depends on FC telemetry, which is fine: it is a *payload* gate, not a
flight function, and its failure direction is always "stop transmitting".

### C.3 Task graph

```
[radar subnet] ─ cc-radar control task ⇄ TDA2 :5001         (dwell FSM, low rate)
[GPIO/PPS]     ─ cc-radar-sync task    → SYNC edge ledger   (µs domain)
[bracket IMU]  ─ cc-radar-sync task    → 1 kHz motion ref
[radar subnet] ─ cc-radar-ingest task  → bounded mpsc → broadcast: RadarEvent
                                                   ├─→ radar log task (own volume)
                                                   └─→ operator downlink task
mission supervisor ── mission lifecycle + airborne state ──→ interlock + dwell FSM
```

Separate NIC and subnet from the FC link; separate bounded channels; separate disk
volume; and the radar control socket never shares `cc-link`'s priority TX queues.
A wedged radar board must be indistinguishable, to the safety loop, from an absent
one.

### C.4 On-disk schemas

**`envelope_fields()` cannot be reused** — it mandates `sequence`,
`fc_timestamp_us` and a `stream_id` drawn from the 8-value `cc_ingest::StreamId`,
none of which a radar frame has. Radar gets `radar_envelope_fields()`: the shared
identity prefix (`vehicle_id`, `mission_id`, `px4_boot_id`, `cc_boot_id`,
`schema_version`) plus radar-specific time columns. Likewise `fsl()` is
Float32-only, so **complex I/Q is stored as explicit `iq_re` / `iq_im` Float32
columns** (a documented layout decision, not an implied one).

| Stream | Grain | Key columns |
|---|---|---|
| `radar_session` | per capture session | `radar_session_id`, `capture_id`, `profile_hash`, `calib_hash`, `mimo_scheme`, `frame_period_ticks`, `trigger_source`, `f_start_hz`, `slope_hz_per_s`, `n_adc`, `opened/closed_cc_ns`, `close_reason`, `frames_reported`, `frames_dropped`, `authorization_id` |
| `radar_dwell` | per hover-and-stare dwell | `dwell_id`, `start/end_cc_ns`, `standoff_m`, `depression_deg`, `surface_type`, `subject_posture`, `gt_reference` (belt/PPG present), `control_kind` (airborne \| landed \| empty), `coherent_frames`, `max_gap_frames` |
| `radar_frame` | per frame | `frame_seq`, `radar_timestamp_ns`, `cc_receive_time_ns`, `sync_edge_index`, `gap_frames`, `age_ns` (nullable), `age_locked`, `die_temp_c[4]`, `rx_gain_db`, `tx_mask`, `cal_event_flags`, bulk ref (`shard`, `offset`, `len`, `dtype`, `dims`) |
| `radar_phase` | per frame × range bin × beam | `frame_seq`, `range_bin`, `beam_id`, `iq_re`, `iq_im`, `snr_db` — **unselected** (§B.2). The irreplaceable stream. |
| `radar_track` | per frame × track | `track_id`, `x`,`y`,`z`, `range_m`, `az_rad`, `el_rad`, `snr_db`, `rcs_dbsm` (nullable), `motion_state`, `age_frames` |
| `radar_vital_estimate` | per track × window | `window_start_cc_ns`, `window_len_ns`, `resp_rpm`, `resp_conf`, `heart_bpm`, `heart_conf`, `method_id`, `snr_db`, `ego_residual_rad`, `clutter_ref_quality`, `quality_flags` |
| `radar_motion_ref` | bracket IMU, ≥1 kHz | `t_cc_ns`, `accel[3]`, `gyro[3]`, `sync_edge_index` |

Detections and estimates are **flat rows** (variable counts per frame, columnar
compression, no `FixedSizeList` lying about a width that varies). Bulk tensors go
to length-prefixed `radar_bulk/NNNNNN.bin` shards with index columns in
`radar_frame` — reusing the proven `raw_mavlink.bin` pattern, including its
"a torn trailing record after `kill -9` is expected and detectable" property.

Per §A.5's *record RCS/SNR, not booleans*: a person's return varies by ~20 dB with
aspect **[corrob]**, so a detector tuned to a single threshold will flicker;
detection must be treated as probabilistic across aspect, and the recorded row
must carry the evidence.

### C.5 Shedding — inverted from the navigation framing

`radar_phase` and `radar_motion_ref` are **never-shed** classes: they are small
(§B.2) and irreplaceable. Bulk tensors shed first, then the monitoring tier. As
established for the mission log, `ShedStage` discriminants are **never
renumbered** (they are on disk as `shed_stage: UInt8` in every `events/` part) —
radar gets its own ladder over its own volume, ordered against the mission-log
ladder by config validation.

### C.6 Config sketch

```toml
[radar]
enable = false
control_addr = "192.168.33.180:5001"
profile = "…/vitals_20hz.toml"          # hashed; mismatch ⇒ refuse to capture
calibration = "…/calib_2026-05.json"    # hashed
radar_root = "/var/lib/companiond/radar"
airborne_tx = false                      # requires authorization_id to be true
authorization_id = ""                    # e.g. FCC Part 5 grant reference

[radar.vitals]
frame_hz = 20.0                          # contract, not a tunable (§A.4)
resp_window_secs = 12.8
heart_window_secs = 25.6
resp_band_hz  = [0.1, 0.6]
heart_band_hz = [0.8, 3.0]               # widened past TI's 2.0 Hz deliberately
min_dwell_secs = 30
max_gap_frames = 0                       # a gap segments the window, never bridges it

[radar.downlink]
enable = false
endpoint = "…"
hz = 1.0
```

### C.7 Operator downlink — locate, don't diagnose

Second leg of "stream pre-processed data", and it does not exist in the stack
today. Tracks + rate estimates + confidence, ~1 Hz, tens of bytes per track,
sized for an LTE/WiFi/900 MHz payload radio. Rules:

* Separate socket, task and bounded queue. **The FC link is never a transport for
  payload products** (§13 envelope).
* Downlink loss degrades to "recorded but not shown" — never affects recording,
  never affects flight.
* **Wording is a compliance boundary, not a UX choice.** The medical-device line
  is drawn by *claimed intended purpose*, and contactless radar vital-sign
  monitors do hold real regulatory clearances **[corrob]**. So the product says
  *"possible living person at this location; non-clinical respiration estimate
  N rpm, confidence C"* — and never a triage category, a clinical threshold, or an
  "abnormal" label.

### C.8 What never reaches PX4

Radar's only channel into `CC_HEALTH_REPORT` is payload **self**-health
(`CC_SUBSYS_RADAR`, `CC_HF_RADAR` — both enum values free today, and both must
land in the PX4 fork's range-validation gauntlet *before* the companion emits
them, or they are rejected as `CC_REJECT_BAD_RANGE`).

This separation is structural, not stylistic: `cc-ai-health::merge()` maps
findings onto **flight actions** (Hold / Land / RTL). A detection entering that
path could produce a Land recommendation *because a person was found*. Detections
and vital estimates therefore never construct a `HealthFinding`.

---

## Part D — Test & verification

### D.1 Bench experiments that must precede any flight

Each of these can invalidate the concept, so they come first and they are cheap:

1. **Line-of-sight range PSD of the actual airframe in hover, 0.05–500 Hz,
   measured at the antenna phase centre** (bracket IMU, or a landed radar staring
   at the hovering aircraft). This single measurement decides feasibility, and it
   appears to be unpublished for a payload of this class.
2. **The ~1 Hz APLL/VCO recalibration test**: static corner reflector, ≥60 s at
   20 Hz, look for 1 Hz-periodic phase steps and correlate with die temperature.
   A step here lands *inside the cardiac band*.
3. **Coherence across `stop`/`start`**: can two consecutive captures be
   concatenated into one coherent slow-time record, or must each be an independent
   coherent block? This decides whether a 26 s window may straddle a capture
   boundary.
4. **Hardware trigger feasibility** on the RF-EVM sync net (§A.6) — including
   whether the master's SYNC_OUT can be cleanly isolated from an injected pulse.
5. **Sustained-capture behaviour** at 20 Hz for a full dwell: measured drop rate,
   `flags` semantics in `*_idx.bin`, and whether the four devices' timestamps come
   from one TDA2 clock (alignment = lookup) or several (alignment = estimation).
6. **Downwash test**: how much does rotor wash move a clothed mannequin, a
   blanket, dust and nearby vegetation in the 0.1–3 Hz band, versus standoff? If
   the answer exceeds ~0.5 mm at 5 m, the airborne concept fails regardless of
   compensation quality.

### D.2 The false-positive catalogue (what will fake a heartbeat)

| Mechanism | Why it is dangerous |
|---|---|
| **Two-ray multipath fading** | λ/4 = 0.95 mm of platform motion sweeps a null; the resulting amplitude/phase fading is indistinguishable from breathing |
| Wind-swayed vegetation, tarps, water surfaces | sit squarely in the 0.1–0.5 Hz respiration band |
| Rotor downwash on fabric/foliage near the subject | ditto, and it is *correlated with the platform*, so naive common-mode rejection will not remove it |
| The aircraft's own props, gear and structure | occupy the near range bins |
| Aliased vibration lines | land at `f_vib mod f_frame` |

Guards, structural rather than threshold-based: a **static-clutter phase
reference** for common-mode cancellation; a **spatial-coherence requirement** (the
vital signature must be localized to the target cell and *absent* from
neighbouring clutter cells); a **cross-band consistency** check (a genuine
cardiac line should be present in the harmonically-consistent way a fading null
is not); and an explicit **false-alarm-rate target measured on empty dwells**
(§B.4).

### D.3 Software test layers

| Layer | Tests |
|---|---|
| Golden vectors | a committed synthetic `*_idx.bin` (24 B header + 48 B records) parsed field-exact — the index parser gets the same treatment as the wire format |
| Unit | interlock truth table (exhaustive, host-run, in the `cc_policy_table` spirit); dwell FSM; profile/calibration hash gate; gap detection and window segmentation; frame contract decode + fuzz; radar ladder; complex-I/Q schema round-trip |
| Determinism | the `cc-vitals` estimator: same recorded phase history ⇒ byte-identical estimates, x86-64 vs aarch64; and the existing 8-algorithm hash **unchanged** by radar's presence |
| FP audit | empty dwells produce zero candidate detections and zero vital estimates; a mannequin produces detection but no vitals; platform-motion-only dwells produce no respiration |
| Fault drills | board absent · board reboot mid-dwell · SSD full · network partition · frame drops (index gaps) · hash mismatch refusal · **airborne-inhibit engaged and logged** · offload interrupted then resumed |
| Soak | a full sortie's worth of dwells: measured drop rate, gap-free coherent segment lengths, disk/CPU/network numbers written back into the budget table as *measured* |

---

## Part E — Phases and exit criteria

| Step | Content | Exit criterion |
|---|---|---|
| **10.0** | **Regulatory + bench feasibility.** Start the authorization conversation (§0.1). Run every D.1 experiment. Build `fake_radar.py`. | A written go/no-go on airborne operation *and* on hardware triggering, backed by measured bench data — before any integration work is committed. |
| **10.1** | `cc-radar` control plane + dwell FSM + **airborne-inhibit interlock** + capture reference + `radar_session`/`radar_dwell` streams. Landed/tripod captures only. | Landed dwells captured under companiond control; mission dataset `log-inspect`-Clean; offload checksum-verified; interlock truth table 100 % host-covered; all fault drills pass. |
| **10.2** | `cc-radar-sync` (µs domain) + `radar_phase`/`radar_motion_ref` recording + `cc-vitals` **offline** estimator, validated against belt/PPG ground truth on landed dwells. | Respiration and heart rate reproduced within a stated error against ground truth on landed dwells; estimator byte-identical on replay; FP rate on empty dwells measured and documented. |
| **10.3** | Tethered / low-hover dwells under authorization; ego-motion compensation (scene-referenced primary, IMU-aided secondary). | Quantified degradation from landed → hovering, with the landed control in every experiment; honest statement of what survives the platform. |
| **10.4** | Monitoring-tier stream + operator downlink; online estimation only if 10.2/10.3 succeeded. | Operator display useful and correctly worded (§C.7); recording unaffected by downlink loss. |
| **—** | *Deferred by design:* any radar → PX4 path. | Out of scope. Would need its own spec change, policy table and audit. |

---

## Part F — Risk register

| Risk | Assessment |
|---|---|
| **Regulatory (highest)** | Airborne 76–81 GHz is prohibited in the US absent an experimental authorization (§0.1). Bench/landed work is unaffected — hence the phase order. |
| **Airborne heart rate may be infeasible at 79 GHz** | Unproven, and §A.5 explains why. Mitigation: the instrument is designed to *measure* that, with landed controls and ground truth. Respiration is the more defensible objective; heartbeat is a research goal. |
| **Buried victims are out of reach** | Physics, not engineering (§0.2). Must be stated to any stakeholder before expectations form. |
| **Hardware trigger may require a board modification** | §A.6. Without it, compensation alignment must be estimated statistically — workable but weaker, and it must be characterised rather than assumed. |
| **Payload mass / power / cooling** | The DSP-EVM is 160 × 136 mm needing 12 V ≥ 3 A, and must supply conditioned 5 V to the RF board **[corrob]**. Added mass changes the hover micro-motion the entire concept depends on — so the airframe study is *part of the measurement chain*, not logistics. The 2-chip AWR2243-2X-CAS-EVM is the fallback. |
| **Search economics** | ~30 s per dwell ⇒ tens of dwells per sortie. Radar is a cued confirmer; something else must cue it. |
| **Data protection** | Respiration/heart-rate estimates of identifiable people are health-adjacent personal data (GDPR Art. 9 territory), and radar cardiac/gait signatures are person-distinguishing — so the dataset is pseudonymous at best. Practical obligations: encryption at rest, an explicit retention clock, access control, and budgeted offload + secure-erase turnaround **[corrob]**. |
| **Medical-device boundary** | Set by claimed purpose (§C.7). Locate, don't diagnose. |
| **RF exposure** | Not a standoff problem: MPE distance is ~5 cm at ~25 dBm EIRP per TX and ~60 cm for a theoretical 12-TX coherent worst case **[calc on unver inputs]**. The real hazard geometry is a ground crew member near the radiating face. |
| **TI software support** | Processor SDK Radar cascade demos are EOL and not AWR2243-compatible **[corrob]**. Own the contract; treat producers as replaceable. |

---

## Part G — Decision log

| # | Decision |
|---|---|
| **R1** | Radar is a sensing payload. Detections and vital estimates never construct a `HealthFinding` and never reach PX4; radar's only report channel is payload self-health. |
| **R2** | The airborne-transmit interlock is a pure, exhaustively-tested gate that fails safe to Inhibit on unknown/stale airborne state, and requires a configured authorization for airborne TX. |
| **R3** | Raw-to-SSD per dwell is mandatory. The processing is unproven, so the recording must assume it is wrong. |
| **R4** | Record complex I/Q, unselected over a coarse beam set — never pre-wrapped/unwrapped phase, never magnitude-only, never in-flight-selected cells. |
| **R5** | The streamed pre-processed tier is a monitoring product, not the record. |
| **R6** | Two time domains, explicitly separated: ms-class context alignment via `cc-timesync`; µs-class compensation alignment via a radar-local trigger/PPS/bracket-IMU subsystem. The FC link is not in the µs path. |
| **R7** | Ego-motion compensation is scene-referenced first, IMU-aided second — lever arm alone defeats pure IMU aiding. |
| **R8** | Gaps segment a coherent window; they are never bridged by interpolation. `max_gap_frames = 0` by default. |
| **R9** | Ground truth and negative controls (belt/PPG on the same clock, corner reflector, empty dwells, landed control) are part of the flight plan, not a later addition. |
| **R10** | The vital-sign estimator is deterministic and replayable (integer clock, fixed-order reductions, `libm`); beamforming/range-FFT is a recorded transform outside the proof. |
| **R11** | `envelope_fields()` is not reused; radar gets its own envelope. Complex I/Q is explicit `iq_re`/`iq_im` Float32 columns. |
| **R12** | `radar_phase` and `radar_motion_ref` are never-shed classes; bulk tensors shed first; `ShedStage` discriminants are never renumbered. |
| **R13** | Operator-facing wording is a compliance boundary: locate, don't diagnose. |
| **R14** | Phase 10.0 (regulatory + bench feasibility) gates all integration work. |

---

## Part H — Verification queue (before any [corrob] / [unver] number becomes a constant)

Nothing below was read in this session; each is the authoritative source for a
number used above.

1. **47 CFR §§ 95.3331, 95.3333** and the Subpart M power section (eCFR) — the
   airborne prohibition, the permitted-use list, and the EIRP limits (the section
   number for the limits is itself uncertain: 95.3345 vs 95.3367).
2. **47 CFR § 15.255** — the 60 GHz airborne exclusion wording.
3. **ETSI EN 302 264 clause 1 (Scope) + power table; ERC/REC 70-03 annex** for
   76–81 GHz — note the mean-power regime for 77–81 GHz is a power *density*
   limit, not the 76–77 GHz +50 dBm mean figure.
4. **FCC Part 5 experimental licensing** — whether airborne 76–81 GHz grants have
   precedent, on what terms, and whether NTIA coordination is triggered.
5. **SWRU553A** (MMWCAS-RF-EVM UG) §§ 2.6.2.x — antenna gain and elevation
   aperture (the largest single uncertainty in any link budget), plus the sync net
   schematic and the regulatory notice.
6. **SPRUIS6** (MMWCAS-DSP-EVM UG) — SSD capacity, capture limits, power.
7. **TI vital-signs lab guides** (`vitalSignsLab_xwr1443_DevelopersGuide.pdf`,
   `vitalSigns_lab_user_guide`) — frame rate, window lengths, the actual biquad
   passbands, FFT length and how much quoted resolution is zero-padding.
8. **SPRACV2 / SPRACF4C / SWRA574B** — AWR2243 phase-vs-temperature tables, cascade
   calibration procedure, and the APLL/VCO recalibration cadence.
9. **mmWave Studio Cascade UG** + `rl_sensor.h` frame-config constraints — the real
   minimum frame periodicity and inter-frame blank time.
10. **Human RCS at 76–81 GHz from above** (standing / supine / prone) and **σ⁰ for
    grass, soil, gravel, rubble at 20–90° depression** — both appear unmeasured in
    the accessible literature, and the near-nadir quasi-specular case is exactly
    the geometry this payload flies.
11. **Published UAV-borne vital-sign work** (7.3 GHz XeThru, 24 GHz dual-radar,
    9.2 GHz noise radar) — the compensation methods to port, and their honest
    limits.

---

## Sources

Platform and tooling facts:
[MMWCAS-DSP-EVM UG (SPRUIS6)](https://www.ti.com/lit/ug/spruis6/spruis6.pdf) ·
[MMWCAS-RF-EVM UG](https://manuals.plus/m/6ba4804c928286c03a74aaec1357d3ae037d7d14a929514c18393e8d7160ed68) ·
[E2E: no live raw streaming](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/881233/mmwcas-rf-evm-how-to-stream-via-mmwcas-dsp-evm-raw-data-to-a-desktop-computer-and-process-in-soft-real-time) ·
[E2E: continuous streaming support](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/854512/mmwcas-rf-evm-continuous-streaming-for-the-evm-support) ·
[E2E: TDA2x real-time demo (EOL)](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/1200296/mmwcas-dsp-evm-ros-gui-in-processor-sdk-radar-for-real-time-demo-with-ti-jacinto-tda2x-processing) ·
[E2E: cascade object detection use case](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/946616/mmwcas-dsp-evm-radar-sdk---cascade-object-detection-sample-usecase) ·
[azinke/mmwave](https://github.com/azinke/mmwave) (Ethernet control, `192.168.33.180:5001`, captures to `/mnt/ssd/`) ·
[azinke/mmwave-repack](https://github.com/azinke/mmwave-repack/blob/main/repack.py) (the `*_idx.bin` layout and frame geometry quoted in §B.1) ·
[E2E: master/slave `.bin` format](https://e2e.ti.com/support/sensors-group/sensors/f/sensors-forum/901469/mmwcas-dsp-evm-what-is-data-format-of-master-slave-bin-files-from-linux-cascade-radar-board)

Regulatory, physics and prior-art claims were gathered via search extracts of the
documents listed in Part H; the primary documents were **not** accessible from
this session and are queued for verification there.
