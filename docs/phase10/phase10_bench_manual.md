# Phase 10.0 bench manual — runnable protocols

> **Purpose:** make day one at the bench productive instead of exploratory. Every
> E-test from [`radar_transport_and_sync.md`](radar_transport_and_sync.md) §E is
> turned into a procedure with its exact configuration, its analysis, its
> pass/fail arithmetic, and the trap that would make a null result meaningless.
>
> **Status:** authored before the hardware arrives. The analysis half is written
> and self-tested ([`tools/phase10/bench/`](../../tools/phase10/bench/README.md));
> the numbers below are from source, not from a bench.

**Sourcing tags.** **[code]** read from a file fetched and quoted (the strongest
class available: `ti.com`, `e2e.ti.com`, `docs.nvidia.com` are blocked by egress
policy here) · **[calc]** arithmetic done here · **[meas]** measured on this
machine · **[prim]** primary document read in the 2026-08 pass · **[corrob]**
multiple independent snippets · **[unver]** single source or inference ·
**[DATASHEET]** must be confirmed from a vendor PDF before it is trusted.

Two sources did most of the work and are worth naming, because they replace
documents we cannot open:

* **The AWR2243 DFP-2G mmwaveLink headers** (`mmwaveDFP_2G`, vendored in
  `gaoweifan/pyRadar`) — every struct, field, LSB, bit position and doxygen note
  below is quoted from them.
* **The MMWCAS-RF-EVM Rev D ODB++ manufacturing output** (Altium, 2020-02-10),
  recovered from a public repo and parsed at pin-and-value level. The sync
  network is therefore *known*, not inferred — including two planning
  assumptions it overturns (§4).

Corrections this manual applies to the existing design docs are listed in §10.

---

## Part 0 — Before power

### 0.1 The legal frame (short version: the bench is fine)

Ground and bench operation of 76–81 GHz is the lawful regime — 47 CFR § 95.3331
permits fixed and mobile ground use, and § 95.3333 prohibits only *airborne*
operation **[prim]**. **The entire bench campaign is unaffected by the
authorisation question.** Nobody should stall waiting for a licence they do not
yet need; the licence gates flight (Phase 10.3+), not measurement.

### 0.2 RF safety

MPE distance is ~5 cm at ~25 dBm EIRP per TX and ~60 cm for a theoretical 12-TX
coherent worst case **[calc]**. The hazard geometry is a hand or face at the
aperture, not a subject at 1 m. Working rule: **nobody within 1 m of the antenna
face while transmitting, never eye-level inside 60 cm.** Post it on the bench.

### 0.3 Electrical safety and power-up order

| Item | Value | Source |
|---|---|---|
| RF board supply | 5 V, up to 8 A (~40 W worst case) via J4/J5 from the DSP-EVM, or bench 5 V on J6 with the Kelvin sense pair | **[prim]** |
| DSP-EVM supply | 12 V ≥ 3 A (12 V/5 A typical) | **[prim]** |
| RF board outline | **160.00 × 136.40 mm** (6.29921 × 5.37007 in, from the ODB++ board profile) | **[code]** |
| Sync IO domain | **3.3 V** — U8 VDD/VDDO on `LMK2_3V3`, and AWR ball R13 is the only 3.3 V ball per die. **No level translator anywhere on the sync path** | **[code]** |

Order: 12 V first → confirm the DSP-EVM boots and the SSD mounts → then the RF
board. **Never hot-plug the 60-pin host connector.** 40 W in a small board with no
airflow makes die temperature a *measurement input* (§2.4), so instrument it from
the first power-on rather than bolting it on later. Wrist strap and mat: the sync
tap involves probing a live 77 GHz front end.

### 0.4 Kit

Corner reflector (trihedral, ≥10 cm, on a surveyed tripod mount) · a second
tripod for the radar with a repeatable pointing reference · oscilloscope with
≥200 MHz bandwidth and a low-capacitance probe (for §4.4, non-negotiable) ·
soldering iron with a fine tip and 0402-capable tweezers (for §4.6 only) ·
a Polar-H10-class chest strap (D3+) · a mannequin or body-sized RCS surrogate ·
two SD cards, one per TDA2 firmware image (§6).

---

## Part 1 — Day-one order

Each step de-risks the next; nothing later depends on a step being skipped.

1. **Power and boot only, no RF.** DSP-EVM console, network, `/mnt/ssd` mounted.
2. **Control-plane reachability.** TCP:5001, firmware download to all four dies,
   read back device IDs and die temperatures (§2.4). Still nothing transmitting.
3. **One default capture** with the reference configuration, to SSD. Prove the
   file pair appears and that `tools/phase10/bench` parses it. *This is the first
   moment anything transmits — apply §0.2.*
4. **E1** control-plane parity (§7.1).
5. **E2** drop-rate sweep (§7.2) — cheap, and it decides which modes are usable.
   **E4** and **E5** fall out of the same captures at no extra bench cost.
6. **Sync tap**: build the harness, verify the edge **on a scope** (§4.4) before
   it ever touches the Jetson. Then **E6**, then **E7/E8** (§7.6–7.8).
7. **E10** and **D1** with the corner reflector (§7.10) — the two tests whose
   results change the algorithm design.
8. **E9** (§7.9), **E11** (§7.11).
9. **E3** (§7.3) and **E12** last: E3 is *expected to be impossible* on stock
   firmware and E12 is time-boxed, so neither may block anything.

---

## Part 2 — Control-plane configuration: freezing calibration and instrumenting it

### 2.1 What is actually freezable (smaller than assumed)

The runtime calibration mask has **only four usable bits** **[code]**:

| Bit | Runtime calibration |
|---|---|
| 4 | LODIST |
| 8 | PD (peak detector) |
| 9 | TX power |
| 10 | RX gain |

HPF cutoff, LPF cutoff, RX ADC DC offset, TX phase and RX IQMM are **boot-only**
calibrations, selectable only in `rlRfInitCalConf_t.calibEnMask` (bits 5, 6, 7,
11, 12) **[code]**. So "freeze all host-disableable runtime calibrations" is
literally `periodicCalibEnMask = 0` — there is no other runtime knob. APLL and
synth-VCO run unconditionally, confirmed verbatim in the header **[code]**.

Identifier corrections against the names used in earlier drafts **[code]**:

| Wrong | Right |
|---|---|
| `rlRfInitCalibConf_t` | **`rlRfInitCalConf_t`** |
| `rlRfCalMonTimeUnitConf_t` | **`rlRfCalMonTimeUntConf_t`** ("Unt") — but the *function* is `rlRfSetCalMonTimeUnitConfig` |
| `oneTimeCalibEnableMask` / `periodicCalibEnableMask` | **`oneTimeCalibEnMask` / `periodicCalibEnMask`** |
| report carries an `errorCode` | **it does not** — no such field exists |

### 2.2 The configuration that both freezes and reports

There is a real tension: TI's cascade guidance is to set calibration periodicity
to 0 for phase synchronisation, but **`calibPeriodicity = 0` is precisely the
report-suppression setting** **[code]**. The resolution, and it is the single most
important configuration in this manual:

```c
/* Per device, single-bit deviceMap. Designated initializers are mandatory:
   the field order swaps under MMWL_BIG_ENDIAN. */
rlRunTimeCalibConf_t c = {0};
c.oneTimeCalibEnMask  = 0;   /* no one-time cals during the record            */
c.periodicCalibEnMask = 0;   /* TI: disable in cascade for phase sync         */
c.calibPeriodicity    = 4;   /* valid range is 0 or 4..100 — so 1, 2 and 3 are
                                ALL outside it, and 1 specifically "will cause
                                internal APLL and SYNTH calibrations to stop".
                                0 would suppress the reports we need.
                                1 LSB = 1 calibMonTimeUnit.                    */
c.reportEn            = 1;   /* bit0: emit a report per calibration event      */
rlRfRunTimeCalibConfig(deviceMap, &c);
```

A non-zero `calibPeriodicity` with `periodicCalibEnMask = 0` still runs **no user
calibrations** — the header states periodicity "is applicable only for those
calibrations which are enabled to be done periodically in the
`periodicCalibEnMask` field" **[code]**. So the cascade phase-sync recommendation
is preserved *and* the 1 Hz APLL/SYNTH events become observable. **Note the
status of that last sentence honestly:** it is assembled from two header quotes
in different places rather than stated as one fact, so the *composite* is an
inference **[unver]** — and since it is the single most important configuration
in this manual, confirm it in the first five bench minutes by checking that
`calibUpdateStatus` reports only APLL/SYNTH bits (1/2/3) and never the LODIST /
PD / TX-power / RX-gain bits. Timing: pair
`calibMonTimeUnit = 5` frames (250 ms at 20 Hz) with `calibPeriodicity = 4`
→ 1 s; or `calibMonTimeUnit = 1` (50 ms) with `calibPeriodicity = 20` → 1 s.
At a 50 ms frame only `calibMonTimeUnit` 1…5 stays inside the documented
40–250 ms window, while the **device default is 100** — so leaving the default is
out of range and must be set explicitly.

### 2.3 The report

`reportEn` bit 0 makes the device emit `RL_RF_AE_RUN_TIME_CALIB_REPORT_SB`
(**0x12**, inside `RL_RF_ASYNC_EVENT_MSG` 0x80 → unique sub-block **0x1012**),
payload `rlRfRunTimeCalibReport_t { calibErrorFlag, calibUpdateStatus,
temperature, reserved0, timeStamp, reserved1 }`
(`mmwaveDFP_2G/ti/control/mmwavelink/mmwavelink.h`, L2223–2274) **[code]**.

Two fields carry the experiment:

* **`calibUpdateStatus` bits 1/2/3 = APLL tuning / SYNTH VCO1 / SYNTH VCO2**
  **[code]** — per-event, whether RF was *actually* reconfigured. This is a
  **within-record negative control at identical times**: events where nothing was
  updated should show no step, and if they do, the step is not the calibration.
* **`timeStamp`**, 1 LSB = 1 ms, rolls over on its bit width **[code]**. Its epoch
  is *not* documented (unlike `rlRfTempData_t.time`, which says "from device
  powerup"), so treat the shared-epoch assumption as **[unver]** and cross-check
  it once on the bench. A 30-minute record needs rollover unwrapping.

### 2.4 Die temperature — do not use the report's field

The report's `temperature` "is updated only when a run-time calibration is
executed due to a change in temperature by more than 10 °C" **[code]** — it is
**not a per-second thermometer**. Take true die temperature from
`rlRfGetTemperatureReport` → `rlRfTempData_t`, and mind three traps **[code]**:

1. **Single-bit `deviceMap` only.** `rlDriverCmdInvoke` (`rl_driver.c`:2754–2843)
   loops over every set bit into the *one* caller-supplied buffer, so
   `deviceMap = 0x0F` silently returns only the highest-index die's data **with no
   error**. Poll `RL_DEVICE_MAP_CASCADED_1..4` (1U/2U/4U/8U) separately — four
   round trips per sample, four independent power-up epochs.
2. **Nine usable sensors on AWR2243**, not ten: 4× RX + 3× TX + `tmpPmSens` +
   `tmpDig0Sens`. `tmpDig1Sens` is documented "applicable only in
   xWR1642/xWR6843/xWR1843" (`rl_sensor.h`:1800–1804).
3. **Naming trap:** `rlRfTempData_t.tmpDig0Sens`/`tmpDig1Sens` correspond to the
   monitoring report's `TEMP_DIG1`/`TEMP_DIG2` — off by one between two TI
   structs.

### 2.5 What TI's own cascade example does *not* do

TI's 4-chip example (`Dongwoo-K/mmwavelink_cascade/mmw_example.c`) never calls
`rlRfRunTimeCalibConfig` or `rlRfSetCalMonTimeUnitConfig` at all, issues every API
on a **single-device `deviceMap` from a per-device thread**, and keeps strictly
per-die calibration blobs (`CalibrationData_0..3.txt`) **[code]**. Adopt the
per-device-thread pattern; it is also what makes trap 1 above harmless.

---

## Part 3 — One profile, three modes

### 3.1 The shared low-IF profile

Vitals want low IF, because the 0/1.1 ns bimodal chirp-start jitter costs phase in
proportion to beat frequency. One profile serves all three modes **[code + calc]**:

| Field | Value | Meaning |
|---|---|---|
| `freqSlopeConst` | 1657 | 80.0 MHz/µs |
| `digOutSampleRate` | 6250 | ksps, Complex1x |
| `numAdcSamples` | 256 | |
| `adcStartTimeConst` | 400 | 4.0 µs |
| `rampEndTime` | 4700 | 47.0 µs |
| `idleTimeConst` | 1000 | 10.0 µs |

Derived, with **c = 299 792 458 m/s** (an earlier draft used 3e8, a uniform
+0.11 % bias) **[calc]**:

* valid sweep `B = |S| · numAdcSamples / Fs` → range resolution **4.575 cm**
* `R_max = c·Fs / (2S)` → **11.711 m** unambiguous
* IF slope **0.5337 MHz per metre** of range
* chirp cycle `Tc = (idleTimeConst + rampEndTime + 1) × 10 ns` = **57.01 µs**
* λ at centre = 3.8006 mm → **5.2786 µm per degree** of phase

At 1.5 m the beat frequency is 0.8 MHz, so 1.1 ns of chirp-start jitter is 0.32°
= **1.7 µm** of apparent chest motion **[calc]** — two orders below the cardiac
signal, which is the point of choosing a low IF.

Two caveats. **Usable IF bandwidth is 0.9·Fs for Complex1x** **[corrob]**, so
treat the top ~10 % of range bins as anti-alias roll-off. And `digOutSampleRate =
6250` may be rounded by the DFE decimator with no API reporting the achieved rate
**[open]** — fall back to 8000 (used by `azinke/mmwave`) or 10000 (TI's cascade
example) if a rounded rate proves unacceptable, at the cost of raising IF per
metre.

**RX gain:** all three reference configs leave bits 7:6 = `0b00`, which selects
the 30 dB RF gain target; TI's own DFP example uses `rxGain = 30` **[code]**.
Choose deliberately rather than copying `48` from a third-party config, and record
it — RX gain is also a *storage* parameter (it sets how much of int16 the capture
uses, hence the compression ratio).

### 3.2 Frame configuration per mode

Fixed facts **[code]**: `numLoops` valid **1…255** (legacy; 1…32768 only with
`miscCtl` b1 `ADVANCE_CHIRP_CONFIG_EN`) · `framePeriodicity` 1 LSB = **5 ns**,
range 300 µs…1.342 s · `triggerSelect` `RL_FRAMESTRT_API_TRIGGER = 0x1`,
`RL_FRAMESTRT_SYNCIN_TRIGGER = 0x2` · **`frameTriggerDelay` is "applicable only in
SINGLECHIP" so it must be 0 in cascade.**

| Mode | TX × loops | Chirps | Active time | Idle at 20 Hz | Bytes/frame |
|---|---|---|---|---|---|
| VITALS-1 | 1 × 2 | 2 | 0.11 ms | **49.9 ms** | 32 KiB |
| VITALS-3 | 3 × 4 | 12 | 0.68 ms | **49.3 ms** | 192 KiB |
| SCAN-12 | 12 × 16 | 192 | 10.9 ms | **39.1 ms** | 3.00 MiB |

All three leave **39–49 ms of contiguous inter-frame idle**, vastly more than the
~500 µs that APLL + SYNTH + apply need, so all three are compatible with
`calibMonTimeUnit` 1…5 **[calc]**. Trigger: `0x1` (API/SW) on the master
(`deviceMap == 1`), `0x2` (SYNCIN/HW) on all slaves — both reference drivers do
exactly this **[code]**.

**The 12-bit packing saving is not an mmwaveLink field** — it is a TDA2-side
command **[code]**. Whether it truncates or requires `rlDevDataFmtCfg_t.adcBits =
0`, and whether the unpacked value is sign-extended, is **[open]**.

---

## Part 4 — The sync tap and the external trigger

This section is the reason the ODB++ recovery mattered: the sync network is now
known at pin-and-value level, and it overturns two assumptions.

### 4.1 The network as built **[code]**

```
AWR1 (U1_1) ball P11  [net DIG_SYNCOUT_RS_1]
   → R1_1  33.2 Ω series
   → [net AWR_1_DIG_SYNCOUT]
   → R143  0 Ω (ERJ-2GE0R00X)          ← master's drive into the fan-out
   → [net DIG_SYNC_SOURCE]  ──────────→ U8 pin 8 (LVCMOS_CLK)
                                ↑
   J4 pin 64 [EXT_DIG_SYNC] → R142 0 Ω ┘   ← the provisioned external input,
                                            1.04 mm from R143, bottom side
```

U8 = **LMK00804BPW**, a 1:4 buffer. Full pin functions: GND 1, OE 2, VDD 3,
CLK_EN 4, CLK 5, nCLK 6, CLK_SEL 7, **LVCMOS_CLK 8**, GND 9, Q3 10, VDDO 11,
Q2 12, GND 13, Q1 14, VDDO 15, Q0 16 — so **Q0 (pin 16) → AWR_1** (the master's
own loopback for delay matching), Q1 → AWR_2, Q2 → AWR_3, Q3 → AWR_4 **[code]**.

Configuration straps, all 10.0 kΩ and all fitted **[code]**: R139 CLK_SEL→GND
(**hard low**), R141 CLK→3V3 and R144 nCLK→GND (the unused differential pair
DC-parked), R137 CLK_EN→3V3, R138 OE→3V3. TI's own schematic text states
*"CLK_SEL low selects LVCMOS_CLK path for DIG_SYNC_SOURCE"* **[code]**. Power pins
are named **VDD/VDDO** on TI's schematic, not VCC/VCCO.

### 4.2 Does a clock buffer pass a 20 Hz aperiodic pulse? Yes.

This was the question most likely to sink the plan, and **the board itself is the
existence proof**: the master's per-frame `DIG_SYNC_OUT` pulse reaches U8 pin 8
DC-coupled through a 0 Ω jumper, with the differential input parked and CLK_SEL
strapped to select the LVCMOS path. There is no AC coupling and no minimum
frequency in play **[code]**.

**The constraint is slew rate, not frequency.** TI's LMK00804B documentation
expresses the input requirement as slew (optimal ≥ 3 V/ns; below roughly
0.05 V/ns the output can chatter from input-stage feedback), with AC parameters
specified to the 350 MHz maximum **[corrob — paraphrase, not quotation:
SNAS642 is not readable from this network]**. Chatter here means **spurious frame
triggers on all four dies simultaneously**, so a slow or noisy edge is not a
degraded measurement, it is a corrupted capture.

### 4.3 Where to tap — and where not to

**Do not tap J4 pin 83.** It reaches `AWR_1_DIG_SYNCOUT` through **R148 = 10 kΩ**,
and the pulse is only **15–20 ns** wide and cannot be widened; the RC integrates
it to 0.7–0.9 V at best and ~0.16 V with any real wire — below any buffer
threshold **[code]**. This kills the "candidate observation tap" the transport
document listed.

**Tap `TP1_1 … TP1_4`** instead: 1 mm SMD test pads sitting **directly on each
die's `DIG_SYNC_OUT`** **[code]**. One pad per die also means the tap can verify
inter-die skew, which no other access point offers.

### 4.4 The observation harness

Requirements, in order of importance:

1. **Do not load the net.** The pulse is 15–20 ns into a 33.2 Ω series resistor
   feeding a buffer input; a long unterminated stub will both distort the edge the
   *radar* needs internally and fail to reach the Jetson cleanly. Put a buffer
   **at the radar end**, within a few centimetres of the pad, and drive a
   terminated line to the Jetson.
2. **3.3 V throughout, no translation** — the sync domain is 3.3 V **[code]**.
3. **Single-gate Schmitt buffer** (SN74LVC1G17-class) at the pad; its hysteresis
   is what makes the far end immune to the cable. Series ~33 Ω at the driver,
   receiver end unterminated (the Jetson pin has a pull-up, see §5.2).
4. **Widen the pulse if the far end needs it.** 15–20 ns is comfortable for HTE
   (32 ns quantisation, see §5.3 — one quantum, so a pulse this short is *at* the
   resolution limit). A monostable (SN74LVC1G123-class) stretching it to ~1 µs
   removes all doubt about edge capture without changing its *timing*, since HTE
   stamps the rising edge.
5. **Galvanic isolation is affordable.** Two separately-powered boards share a
   ground through this wire; a digital isolator (ISO7721-class) removes that loop.
   Its 1–2 ns of propagation jitter is **irrelevant against a 32 ns quantum**
   **[calc]** — so take the isolation. Its fixed propagation *delay* is a constant
   in the ledger and must be characterised once (§7.8).

**Before connecting to the Jetson**, put the scope on the harness output and
confirm: amplitude, rise time, pulse width, one pulse per frame at exactly the
frame rate, and — critically — **that slave triggering still works with the tap
connected** (§7.6's null-invalidator).

### 4.5 Driving `EXT_DIG_SYNC` (Phase 11, but plan it now)

J4 pin 64 is the provisioned input **[prim]**, and contention is a **single 0402
removal**: R143 (0 Ω, master's drive) and R142 (0 Ω, EXT_DIG_SYNC) feed the same
node 1.04 mm apart on the bottom side **[code]**. Remove R143 to hand the
fan-out to an external trigger; refit it to restore stock behaviour. Reversible,
and no work at the buffer itself.

The driver must meet the slew requirement of §4.2 into a 3.3 V LVCMOS input — so
drive it from a real buffer, not a microcontroller GPIO through a long wire.

### 4.6 [DATASHEET] Confirm before building

1. **LMK00804B pin-8 VIH/VIL, hysteresis (if any), absolute maximum input
   voltage, and the formal minimum slew rate.** Two independent reports put the
   real switching point near 0.45 V rather than mid-rail; if true, the noise
   margin is asymmetric and the buffer choice in §4.4 becomes mandatory rather
   than advisable. **The entire external-drive margin rests on this number.**
2. LMK00804B recommended VDD range (a distributor lists 3.135–3.465 V for the
   non-Q1 part while a Q1 title says "1.5 V to 3.3 V" — resolve which is VDD and
   which VDDO).
3. AWR2243 `DIG_SYNC_OUT` drive strength and `DIG_SYNC_IN` input thresholds.

---

## Part 5 — The HTE path on the Orin Nano

### 5.1 It needs no kernel work

Stock JetPack 6 already ships it: `CONFIG_HTE=y` and `CONFIG_HTE_TEGRA194=y` in
the L4T R36.4.4 arm64 defconfig, and `hardware-timestamp@c1e0000` is
`status = "okay"` in the devkit device tree **[code]**. **No kernel rebuild, no
MB1 pinmux reflash.** The path is:

```
open /dev/gpiochipN   where the chip label is "tegra234-gpio-aon"   (32 lines)
ioctl GPIO_V2_GET_LINE_IOCTL  with flags INPUT | EDGE_RISING | EVENT_CLOCK_HTE
read  struct gpio_v2_line_event                      /* .timestamp_ns, .line_seqno */
```

`EVENT_CLOCK_HTE` is flag **bit 12** **[code]**.

### 5.2 Which pin — correction to the transport document

The devkit 40-pin header exposes **four** AON-domain pins, not two, and **all
four are HTE-capable** (every one of the 32 T234 AON lines has a valid slice-2
mapping) **[code]**:

| Header pin | Line | Bus | On-board clients | Verdict |
|---|---|---|---|---|
| **3** | PDD.02 (offset 22) | GEN8_I2C `i2c@c250000`, `hdr40_i2c1` | **none** | **use this** |
| **5** | PDD.01 (offset 21) | GEN8_I2C | **none** | **or this** |
| 27 | PDD.00 | GEN2_I2C `i2c@c240000` | yes — an on-board device sits on this bus | avoid |
| 28 | PCC.07 | GEN2_I2C | yes | avoid |

The two dimensions disagreed on *which* on-board device sits on `i2c@c240000`
(a Type-C role controller vs a power monitor) **[unver]** — but agreed that it has
one and that pins 3/5 do not. So the earlier plan to use pins 27/28 would have
contended with an on-board peripheral. **Use pin 3 or pin 5.** Free them by
disabling the `i2c@c250000` node (a device-tree overlay suffices; no kernel
rebuild), and confirm the board pull-up value on that bus — the 1.5 kΩ figure was
established for pins 27/28 and pins 3/5 will differ **[open]**.

### 5.3 The clock correlation is far easier than feared

The three-way ledger's hardest-looking term evaporates **[code]**:

* `gpio_v2_line_event.timestamp_ns` under HTE is literally **(TSC ticks << 5)**.
* `__init_el2_timers` writes `cntvoff_el2 = 0`, so **`CNTVCT_EL0` == the TSC
  exactly**.
* 1e9 / 31 250 000 = **32 exactly**, so `clocks_calc_mult_shift` yields an exact
  multiplier and `CLOCK_MONOTONIC_RAW` is exactly 32 ns per tick with **zero rate
  drift** versus HTE.

Therefore the HTE↔`CLOCK_MONOTONIC_RAW` mapping is a **pure constant offset**
(stepping only across suspend/resume), recoverable to sub-microsecond with a
sandwich read of `CNTVCT_EL0` around `clock_gettime`. There is no drift to track
on this side — all the drift in the ledger belongs to the *radar* clock, which is
exactly what `ledger.py` fits.

### 5.4 What this path cannot do

20 Hz is trivially safe. A **20 kHz chirp-rate tap is not feasible**: the
hardirq→workqueue handoff stores only one scalar (`line->timestamp_ns`), so bursts
silently coalesce — visible only as gaps in `line_seqno` **[code]**. Any future
chirp-granularity timestamping needs a different mechanism, and the `line_seqno`
gap is the detector that proves coalescing happened.

---

## Part 6 — TDA2 firmware: two images, and why E3 is impossible

### 6.1 The two images are two build configurations **[code]**

From the Vision SDK sources (a complete public mirror was cloned and read):

| | SSD capture | Ethernet stream |
|---|---|---|
| Config | `apps/configs/tda2xx_cascade_linux_radar/cfg.mk` | `apps/configs/tda2xx_cascade_bios_radar/cfg.mk` |
| A15 | `A15_TARGET_OS=Linux`, `IPU_PRIMARY_CORE=ipu2` | RTOS; `PROC_A15_0_INCLUDE=no` |
| Network | **`NDK_PROC_TO_USE=none`**, no TFDTP | `NDK_PROC_TO_USE=ipu1_1`, `NSP_TFDTP_INCLUDE=yes` |
| Usecase | `cascade_radar_datacollect` = Capture → Sync → DataCollect (A15) → `/mnt/ssd` | `cascade_radar_capture_only` = Capture → NetworkTx (IPU1_1) |

> **E3 (simultaneous SSD capture + Ethernet stream) is impossible by
> construction.** The streaming image has no A15/Linux, hence no NVMe; the SSD
> image has no NDK/TFDTP stack and no M4 core to host it. These are two SD-card
> images, so **E2 and E3 are two bench sessions**, and E3's honest outcome is a
> written negative result plus a scoped estimate of the firmware work.

(Aside worth knowing when reading those files: `cfg.mk` line 107 contains a
malformed `ifeq (RADAR_ONLY,yes)` missing its `$(...)`, so that conditional never
fires; the effective `A15_TARGET_OS=Bios` comes from `apps/configs/defaults.mk`
line 34 **[code]**. Do not "fix" it while chasing something else.)

### 6.2 Driving the app from a script **[code]**

Both usecases are compiled into their image and selected from a runtime menu, so
switching within an image needs no rebuild. Radar usecases sit behind top-level
menu **`6`**; SSD capture is radar submenu **`2`**; the Ethernet-streaming image
offers **`9`** (stream) and **`a`** (capture + DSP object detect) and has **no
SSD-writing usecase at all**.

The two menu layers read stdin differently: the top level uses plain `getchar()`
(`chains_main.c`:180 — send a bare character, no newline), submenus use
`Chains_readChar()` (`chains_common.c`:197–207), which takes a whole line and
accepts only exactly `"<c>\n"`. Both read ordinary stdin, so the app is
scriptable from a pipe. Exiting takes **two** `x`. Confirm the executable name
with `ls /opt/vision_sdk/*.out` (likely `apps.out`).

### 6.3 The `flags` question is settled — E4 is already answered

`diskBucket.cpp`:179 **[code]**:

```c
idx->flags = (dataFormat & 0xFF) | ((chId & 0xFF) << 8);
```

**`flags` encodes data format and channel id, never drop or overflow status.** So
drop detection must come from `Info.numIdx` plus timestamp gaps — which is exactly
what `capture.py` implements. E4 collapses from an experiment to a one-line
confirmation: dump the first record's `flags` and check the low byte is a format
code and the high byte the channel.

### 6.4 Two capture limits that change the protocols **[code]**

* `DATA_COLLECT_BUCKET_MAX_BYTES = 2047 × MB`, and the data file is
  `fallocate`d at open with **no `ftruncate` at close** — so **file size is
  meaningless; only the index file counts.** Any analysis keying off `.bin` size
  is wrong by construction.
* **`DISK_BUCKET_MAX_INDEX = 16 × 1024` = 16 384 frames per file = 13.65 min at
  20 Hz.** A 30-minute capture therefore *rolls over* into a second file pair.
  E8's 30-minute drift run and any long soak must handle rollover — this was not
  known when the transport document was written.

---

## Part 7 — The protocols

Each: setup → procedure → what to log → analysis → pass/fail → **the trap that
makes a null result meaningless**.

### 7.1 E1 — control-plane parity

**Setup** reference tool and the Rust client, same config file.
**Procedure** capture the reference tool's SPI/register trace, then the Rust
client's, on identical configuration.
**Analysis** diff the register-write sequences.
**Pass** byte-identical writes in the same order, or every difference explained
and justified in writing.
**Trap** the reference tool issues APIs on single-device `deviceMap` from
per-device threads (§2.5); a Rust client using a cascaded `deviceMap` may look
equivalent and silently hit the buffer-reuse trap of §2.4.

### 7.2 E2 — drop-rate sweep

**Setup** SSD image (§6.1). No target needed; this is a plumbing test.
**Procedure** for each mode (VITALS-1, VITALS-3, SCAN-12) × each periodicity
(4 ms, 10 ms, 25 ms, **50 ms**, 100 ms), 20 × 30 s captures.
**Log** the four `*_idx.bin` per capture, plus `calibUpdateStatus` reports and die
temperatures for context.
**Analysis** `analyse_capture(dir, nominal_period_ns)` → per-device drop rate,
gap list, anomaly list, last-frame-lost flag; `sweep_table()` for the matrix.
**Pass** VITALS modes at 50 ms: **0 drops** across all 20 runs. SCAN-12:
characterised, and compared against the 0.05–0.1 % TI reports at 50–100 ms
**[prim]**. Any *anomaly* (non-integer delta) is a finding regardless of rate.
**Trap** a mode whose active time leaves no inter-frame idle drops frames for a
different reason than the one under test — §3.2 shows all three modes leave
39–49 ms, so compute duty before blaming the capture path.

### 7.3 E3 — simultaneity

**Expected outcome: impossible on stock firmware** (§6.1). Run it only to
document the negative, then scope the modification: hosting TFDTP on a core the
Linux image does not include is not a patch, it is a firmware project.
**Pass** either a sustained streaming rate measured *while* writing to SSD, or a
written negative result naming the missing core and stack.

### 7.4 E4 — `flags` semantics

Already answered in source (§6.3). **Procedure** dump the first `BuffIdx` of
`master_0000_idx.bin`; confirm low byte = format, high byte = channel id.
**Pass** the source claim reproduces. **Trap** none — but if it *doesn't*
reproduce, the SDK version differs from the mirror and every §6 claim needs
re-checking against the shipping 03_08 firmware **[open]**.

### 7.5 E5 — one TDA2 clock or several?

**Analysis** `analyse_capture(...)` reports `cross_device_max_skew_ns` and a
`one_clock` verdict at a 1 µs tolerance.
**Pass** either verdict is a result: one clock makes cross-device alignment a
lookup, several make it an estimation problem.
**Trap** comparing like-*indexed* frames is only valid if no device dropped a
frame — read the drop report first, and if counts disagree, align on the ledger
(§7.7) before drawing a skew conclusion.

### 7.6 E6 — SYNC edge → HTE

**Setup** harness of §4.4 verified on the scope; pin 3 or 5 (§5.2).
**Procedure** 30 s dwell at 20 Hz; then a 30-minute run for M5.
**Log** `(line_seqno, timestamp_ns)` for every event; the `CNTVCT_EL0` sandwich
reads; the capture's index files.
**Analysis** expected edges = duration × frame rate; check `line_seqno`
contiguity (a gap means coalescing, §5.4); jitter = the deviation of edge
intervals from the nominal period.
**Pass** **zero missed edges** over 30 s and over 30 min, with jitter reported.
**Trap** *the tap can break what it measures* — a loaded net can suppress the
edge the slaves need. Confirm slave triggering still works with the tap connected
by checking the capture itself still contains four devices' frames.

### 7.7 E7 — ledger reconciliation

**Procedure** run E6 and E2 simultaneously, then deliberately induce drops
(the 4 ms periodicity case reliably produces them).
**Analysis** `reconcile(edges_ns, idx_ts_ns, T, live_ns=...)`.
**Pass** every induced drop appears as a `capture_drop` and **no false
positives**; missed edges (if any) are reported separately, never as capture
drops.
**Trap** if the two series share no majority of slots (e.g. the capture started
long after edge logging), slot-shift estimation degrades — bound both with
explicit dwell markers.

### 7.8 E8 — radar↔CC drift

**Procedure** a 30 s and a 30-minute run. **Mind the 13.65-minute index rollover**
(§6.4).
**Analysis** `reconcile(...)` → `DriftFit`: ppm, offset, residual RMS, quality.
Subtract the isolator's fixed propagation delay (§4.4) as a constant.
**Pass** ppm fitted and stable between the two durations; residual within
**LOCKED (≤50 µs)**; the isolator delay characterised once.
**Trap** fitting drift over a span shorter than the drift's own timescale — hence
both durations, and hence reporting the residual rather than only the slope.

### 7.9 E9 — coherence across stop/start

**This is an equivalence test, not a null-rejection test** — "we failed to detect
a discontinuity" is not the same claim as "captures may be concatenated".
**Setup** static corner reflector at ~1.5 m.
**Procedure** N ≥ 20 pairs of back-to-back captures with a stop/start between,
interleaved with N continuous captures of the same total length as controls.
**Analysis** phase discontinuity at the seam versus the within-capture
sample-to-sample distribution; declare equivalence only if the seam distribution
lies inside a pre-registered margin derived from the cardiac budget
(≤32 mrad impulse, §7.10).
**Pass** equivalence demonstrated within the stated margin, or a measured seam
step — either answer decides whether a 26 s window may straddle a boundary.
**Trap** a single pair proves nothing; and the LO may re-tune identically by luck
at one temperature but not another, so run across a temperature range.

### 7.10 E10 — the APLL/VCO calibration step *(the priority test)*

The mechanism is documented; the magnitude is the one number TI never publishes.

**Setup** static corner reflector at ~1.5 m, low-IF profile (§3.1), VITALS-1,
`calibPeriodicity = 4`, `reportEn = 1`, `periodicCalibEnMask = 0` (§2.2).

**Procedure — and here the research changed the experiment.** The header states
that with reports on and no one-time cals enabled, **re-issuing
`rlRfRunTimeCalibConfig` makes the firmware attempt an APLL+SYNTH calibration and
send the report immediately** **[code]**. So the host can *command* calibration
events at instants it chooses. That converts E10 from an observational 1 Hz search
into a **randomised-stimulus experiment**:

1. Baseline: 5 minutes of free-running 1 Hz events (≈300 epochs).
2. Randomised: 5 minutes issuing commanded calibrations at **uniformly random
   intervals** (2–7 s), so events are decorrelated from both the 1 Hz grid and
   the frame grid.
3. Positive control: inject a *known* phase step and confirm the pipeline sees
   it — `startFreqVar` at 53.644 Hz/LSB gives 1.26 mrad at 3 m, and the TX phase
   shifter at 5.625°/LSB gives 29.6 µm **[calc]**.
4. Negative control: events whose `calibUpdateStatus` shows **no** RF update, at
   identical times (§2.3).

**Log** the phase record, every report (`timeStamp`, `calibUpdateStatus`,
`calibErrorFlag`), per-die temperatures from `rlRfGetTemperatureReport` (§2.4),
and bench accelerometer data if available.

**Analysis** `fold_at_events(phase, fs, event_times)` → step, SEM, SNR, and the
same in µm. Cross-check with `page_hinkley` on the differenced phase for
unmodelled steps. **Never** rely on `fold_at_period`: the self-test proves a 1 Hz
mechanical artefact fools it.

**Pass** an *upper bound*, not a detection. Acceptance lines from band-leakage
arithmetic: **impulse ≤ 32 mrad** and **staircase σ ≤ 15 mrad** keep cardiac
contamination under 1 % **[calc]**. Detection power: MDS = 4.902·σ/√N **[meas]**,
so 300 epochs resolve ~1 µm across the plausible noise range — 10–100× margin
against the acceptance lines. **E10's product is therefore a certificate, and a
30 s record already suffices for the headline; the 5-minute runs buy the
confound rejection.**

**Traps** (all three are real): a bench that vibrates near 1 Hz — broken by
randomised stimulus; a reflector strong enough that ADC quantisation dominates the
phase noise — check the noise floor first with D1; and folding blind instead of on
reported timestamps.

**Open** whether the 0/1.1 ns chirp-start jitter is per-chirp or per-frame changes
this test's noise floor by ~8× at 64 loops **[open]** — settle it from the same
record by comparing within-frame chirp-to-chirp variance against frame-to-frame
variance of the frame-mean phase, looking for the bimodal component in one and
not the other.

### 7.11 E11 — mode-switch latency

**Procedure** time SCAN-12 → VITALS-3 → SCAN-12 reconfiguration, 20 repeats.
**Pass** worst-case latency recorded; a switch is "safe within a mission" only if
it is bounded *and* §7.9 says a dwell need not straddle it.
**Trap** measuring only the API return time rather than time-to-first-valid-frame.

### 7.12 D1 — noise floor and drift *(bench, no aircraft)*

**Procedure** static corner reflector, 60 s and 30 min, from a cold start so
thermal drift is included.
**Analysis** `band_report(phase, fs)` → band-limited RMS in the respiration and
cardiac bands, in rad **and µm** — the project's existing convention, so the
numbers are directly comparable to the RBEC budget.
**Pass** σ_d reported per band; the 30-minute run additionally gives the thermal
drift term that §7.10's staircase line is measured against.
**Trap** a reflector so strong the IF chain compresses; a session too short for
thermal drift to start; and the 13.65-minute rollover (§6.4).

---

## Part 8 — The canonical bench data product

Every protocol above emits the *same* thing, because E10, E9 and D1 all reduce to
"fold the slow-time phase on an event ledger":

| Table | Contents |
|---|---|
| `bench_session.parquet` | session id, date, operator, board serials, ambient temp, profile hash, calibration hash, firmware image id, harness git hash |
| `bench_dwell.parquet` | one row per capture: test id (E2/E6/E10/D1…), mode, nominal period, duration, target description, pass/fail, free-text note |
| `cal_events.parquet` | per report: `timeStamp`, `calibUpdateStatus` bits, `calibErrorFlag`, commanded-vs-spontaneous, plus the die temperatures sampled around it |
| `frame_index`, `edge_ledger`, `device_state`, `radar_phase` | **unchanged from the flight schema** |

That last row is the point: a bench dwell *is* a dwell. Resisting throwaway
per-test scripts once, on day one, is what makes bench evidence directly
comparable to flight evidence later — the same argument the project already made
for the mission dataset.

---

## Part 9 — Open questions the bench must close

1. **[DATASHEET]** LMK00804B pin-8 thresholds and minimum slew (§4.6) — the
   external-drive margin rests on it.
2. Board pull-up values on header pins 3/5 (§5.2).
3. Whether `digOutSampleRate = 6250` is realised exactly or decimator-rounded
   (§3.1).
4. Whether the shipping 03_08 firmware still uses
   `DISK_BUCKET_MAX_INDEX = 16384` and four buckets rather than one multiplexed
   bucket (§6.4) — first bench action is to dump a `BuffIdx`.
5. Whether the cal-report `timeStamp` shares an epoch with `rlRfTempData_t.time`
   (§2.3) — one cross-check settles it.
6. Whether chirp-start jitter is per-chirp or per-frame (§7.10).
7. Whether `reportEn = 1` permanently costs any inter-frame time at 20 Hz — the
   async event is ~20 bytes at ~1 Hz, so it should be free; confirm no frame is
   dropped when it lands.
8. Whether `calibMonTimeUnit` left at its default of 100 (= 5 s at 20 Hz) raises
   `RL_RF_AE_MON_TIMING_FAIL_REPORT_SB` or is silently accepted.

---

## Part 10 — Corrections this manual applies to the other documents

| Document | Claim | Correction |
|---|---|---|
| `radar_transport_and_sync.md` §D.3 | "candidate observation tap J4 pin 83" | **Unusable** — 10 kΩ series R148 against a 15–20 ns pulse. Use `TP1_1..TP1_4` (§4.3) |
| `radar_transport_and_sync.md` §D.3 | devkit exposes two AON pins (27/28), both with 1.5 kΩ pull-ups | **Four** AON pins; 27/28 have an on-board I²C client and should be avoided; use pin 3 or 5 (§5.2) |
| `radar_transport_and_sync.md` §D.3 | "Driving an external trigger is an input-path selection" | Correct, and now specific: remove R143, refit to revert (§4.5) |
| `radar_transport_and_sync.md` §E, E3 | "expected to fail on stock firmware" | **Impossible by construction** — the two images cannot coexist (§6.1) |
| `radar_transport_and_sync.md` §E, E4 | open question | **Answered in source**: `flags` = format + channel id (§6.3) |
| `radar_transport_and_sync.md` §D.4 | radar↔CC drift fitting | The *Jetson* side contributes **zero** rate drift; all drift is the radar's (§5.3) |
| `radar_dsp_ml_survey.md` §B.1 | "`CALIBRATION_PERIODICITY=1` is invalid — never use as a freeze" | Still true, and add: **`0` suppresses the reports E10 needs**; use 4…100 with `periodicCalibEnMask = 0` (§2.2) |
| `radar_dataset_and_storage.md` `device_state` | die temperature per frame | Must come from `rlRfGetTemperatureReport` per *single-bit* deviceMap, not the cal report's stale field (§2.4) |
| all | `c = 3e8` | Use **299 792 458**; the earlier value carried a uniform +0.11 % bias (§3.1) |
| new | 30-minute captures | Roll over at 13.65 min (§6.4) — affects E8 and every soak |
