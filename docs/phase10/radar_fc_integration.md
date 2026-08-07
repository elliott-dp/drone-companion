# FC ↔ CC integration for the radar payload, and the operator downlink

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md). This is the
paragraph-by-paragraph specification for the **PX4-side** work: the dialect
additions, the uORB topics, the new module, the stream classes, the receiver
gauntlet entries, the parameters, and the path from the aircraft to the pilot's
RadioMaster over **868 MHz ELRS**.

Conventions here match the fork's existing Phase 2/3 patterns exactly (stream
classes gated on `#if defined(MAVLINK_MSG_ID_…)`, `update()`-only send semantics,
publisher decimation as the single rate authority, the §4.4 receiver gauntlet with
named counters in `ccfc_print_stats()`).

Tags: **[calc]** arithmetic here · **[code]** read from source/docs on GitHub ·
**[corrob]** search extracts · **[unver]** inference.

---

## Part A — The invariant that shapes everything

> PX4 is a **relay and a recorder** for radar content. It carries the pilot's
> start/stop intent up to the companion, and it carries one compact result back
> down to the operator. It never evaluates that result, never acts on it, and
> never routes it into `cc_safety_monitor`.

Structurally enforced: the receiver publishes the vitals topic **for logging and
re-streaming only**, and no subscriber to it exists inside the safety monitor. The
reason is concrete rather than stylistic — `cc_safety_monitor`'s policy table maps
severities onto flight actions (Hold/Land/RTL), so a "human detected" arriving on
that path could produce a Land recommendation *because a person was found*. The
two concerns must not share a channel.

---

## Part B — Dialect additions (`cc_dialect.xml`)

ID rules from the file's own header: private block 54000–54099, never reused or
renumbered, `54008` reserved for a future `CC_TELEMETRY_ESC`, `54009` a historical
gap. Next free is **54014**.

### B.1 New enums

```xml
<enum name="CC_PAYLOAD_CMD">
  <description>Payload (radar) command relayed by PX4 from the pilot's RC switch
    or a GCS. Advisory to the companion; PX4 takes no action itself.</description>
  <entry value="0" name="CC_PL_CMD_NONE"/>
  <entry value="1" name="CC_PL_CMD_START_MISSION"/>
  <entry value="2" name="CC_PL_CMD_STOP_MISSION"/>
  <entry value="3" name="CC_PL_CMD_START_DWELL"/>
  <entry value="4" name="CC_PL_CMD_STOP_DWELL"/>
  <entry value="5" name="CC_PL_CMD_ABORT"/>
</enum>

<enum name="CC_CMD_SOURCE">
  <entry value="0" name="CC_CMD_SRC_UNKNOWN"/>
  <entry value="1" name="CC_CMD_SRC_RC_SWITCH"/>
  <entry value="2" name="CC_CMD_SRC_GCS"/>
  <entry value="3" name="CC_CMD_SRC_AUTO"/>
</enum>

<enum name="CC_PAYLOAD_STATE">
  <entry value="0" name="CC_PL_STATE_IDLE"/>
  <entry value="1" name="CC_PL_STATE_CONFIGURING"/>
  <entry value="2" name="CC_PL_STATE_CAPTURING"/>
  <entry value="3" name="CC_PL_STATE_INHIBITED"/>   <!-- airborne w/o authorization -->
  <entry value="4" name="CC_PL_STATE_FAULT"/>
</enum>

<enum name="CC_VITALS_FLAGS" bitmask="true">
  <entry value="1"   name="CC_VF_WINDOW_INCOMPLETE"/>  <!-- <30 s coherent data yet -->
  <entry value="2"   name="CC_VF_MOTION_CORRUPTED"/>
  <entry value="4"   name="CC_VF_CLUTTER_DOMINATED"/>
  <entry value="8"   name="CC_VF_CLOCK_UNLOCKED"/>
  <entry value="16"  name="CC_VF_FRAME_GAPS"/>
  <entry value="32"  name="CC_VF_ML_STALE"/>           <!-- decision older than budget -->
  <entry value="64"  name="CC_VF_MULTI_SUBJECT"/>
  <entry value="128" name="CC_VF_LOSSY_CAPTURE"/>      <!-- controlled-loss mode active -->
</enum>
```

### B.2 `54014 CC_PAYLOAD_COMMAND` (FC → CC)

| Field | Type | Notes |
|---|---|---|
| `fc_timestamp_us` | uint64 | FC monotonic |
| `sequence` | uint32 | monotonic per `px4_boot_id`; the companion echoes it as an ack |
| `param` | uint16 | capture-mode selector / requested dwell seconds (0 = default) |
| `command` | uint8 | `CC_PAYLOAD_CMD` |
| `source` | uint8 | `CC_CMD_SOURCE` |
| `rc_valid` | uint8 | 1 if the RC input backing this command was valid at emission |
| `schema_version` | uint8 | |

18 B payload → ~30 B on the wire **[calc]**. Event-driven, with a bounded repeat
until acked (§D.2).

### B.3 `54015 CC_VITALS_REPORT` (CC → FC)

Ordered size-descending, per the dialect's convention:

| Field | Type | Notes |
|---|---|---|
| `companion_timestamp_us` | uint64 | CC monotonic at report creation |
| `sequence` | uint32 | monotonic per `cc_boot_id` |
| `dwell_id` | uint32 | joins the downlink to the recorded dataset |
| `last_command_seq` | uint16 | **the ack** — mirrors `CC_SAFETY_STATUS.last_report_sequence` |
| `quality_flags` | uint16 | `CC_VITALS_FLAGS` |
| `best_range_dm` | uint16 | decimetres to the best track |
| `resp_rpm_x10` | uint16 | **0 = no estimate.** Never a fabricated value |
| `heart_bpm_x10` | uint16 | 0 = no estimate |
| `decision_age_ms` | uint16 | age of the presence decision (ML latency made visible) |
| `best_azimuth_cdeg` | int16 | centidegrees, body frame |
| `human_present` | uint8 | 0 = no / 1 = yes / 2 = undecided |
| `present_confidence` | uint8 | 0–100, from the classifier |
| `resp_confidence` | uint8 | 0–100 |
| `heart_confidence` | uint8 | 0–100 |
| `n_tracks` | uint8 | |
| `dwell_secs` | uint8 | elapsed coherent seconds — lets the operator watch the window fill |
| `payload_state` | uint8 | `CC_PAYLOAD_STATE` |
| `schema_version` | uint8 | |

36 B payload → ~48 B on the wire **[calc]**. Note the ordering benefit: MAVLink 2
zero-truncation drops trailing zero bytes, so a report with no estimate yet is
naturally shorter on the wire.

**`human_present` has three states on purpose.** "Undecided" is what the pipeline
says before the classifier has enough coherent data; collapsing it into "no" would
turn a not-yet-answered question into a negative result, which is the failure mode
that gets people missed.

### B.4 `54016 CC_PAYLOAD_STATUS` (CC → FC, optional, 0.2 Hz)

Payload self-health for ULog and the GCS — not for the operator's primary display:
`companion_timestamp_us`, `frames_captured`, `frames_dropped`, `disk_free_mib`,
`capture_mode`, `clock_quality`, `inhibit_reason`, `board_temp_c`, `schema_version`.
This is the message that makes a disappointing sortie diagnosable afterwards.

### B.5 One extension to `CC_TELEMETRY_STATE` — the interlock needs it

The airborne-transmit interlock (§A.2 of the harness document) must fail safe on
*certain* knowledge of being airborne, and the current Class A payload carries only
`nav_state` / `arming_state` / `failsafe_flags` — from which "airborne" can be
*inferred* but not *known*. Inference is not good enough for a legal interlock, so
add PX4's own land detector:

```xml
      <extensions/>
      <field type="uint8_t" name="landed_state">0 = unknown, 1 = landed,
        2 = in air, 3 = maybe-landed (vehicle_land_detected).</field>
```

Why this is cheap and safe: MAVLink 2 extension fields are **excluded from
CRC_EXTRA**, so old parsers on either side keep working, and zero-truncation means
the field costs nothing when unset **[std]**. `schema_version` does **not** bump —
no existing field changes meaning, which is exactly the criterion the cross-phase
rules state. The `dialect_hash` *does* change, so the build-time gate correctly
forces both sides to regenerate in one commit, and the golden vectors are
regenerated with it.

---

## Part C — uORB layer

New `msg/` definitions, registered in `msg/CMakeLists.txt`, following the Phase 2
conventions (`uint64 timestamp` first, fields size-descending, one topic per
message):

| File | Direction | Publisher → Subscriber |
|---|---|---|
| `msg/CcPayloadCommand.msg` | FC → CC | `cc_payload_bridge` → `MavlinkStreamCcPayloadCommand` |
| `msg/CcVitalsReport.msg` | CC → FC | `mavlink_receiver` → logger + `MavlinkStreamCcVitalsReport` |
| `msg/CcPayloadStatus.msg` | CC → FC | `mavlink_receiver` → logger |

`CcTelemetryState.msg` gains `uint8 landed_state` to feed B.5, sourced from
`vehicle_land_detected` in `cc_telemetry_publisher`.

---

## Part D — The new PX4 module: `cc_payload_bridge`

A small module, in the style of `cc_telemetry_publisher` (ScheduledWorkItem on
`lp_default`, Kconfig entry, started from rcS, no heap after init). It exists so
that RC edge detection and command repetition live on the FC, where the RC data
actually is — and so the companion never has to interpret raw stick values.

### D.1 RC → command

* Subscribes `rc_channels` (or `manual_control_switches` where the mapping
  exists) and reads the AUX channel named by `CC_PL_RC_CH`.
* **Debounce and hysteresis**: a transition must persist for `CC_PL_RC_DEB` ms and
  cross `CC_PL_RC_THR` with a dead-band, so a noisy channel cannot toggle captures.
* On a confirmed edge, publish `cc_payload_command` with an incremented `sequence`,
  `source = RC_SWITCH`, and `rc_valid` from the RC-lost flag.
* **RC lost ⇒ never synthesise a start.** With `rc_valid = 0`, the module emits
  nothing new; the companion's own rule is that an unknown switch position holds
  the current state. Whether RC loss *stops* an in-progress capture is the
  companion's decision (`on_rc_loss = hold | stop`), because only the companion
  knows how much of a 30 s dwell is already banked.

### D.2 Repeat-until-acked

The uplink can lose a command, and losing a *stop* matters more than losing a
start. So the module re-publishes the current command at `CC_PL_REP_HZ` (default
2 Hz) until either `cc_vitals_report.last_command_seq` matches its `sequence`, or
`CC_PL_ACK_TO` ms elapse — then it falls back to a 0.2 Hz keep-alive of the same
command. This is deliberately the *same* ack idiom the safety loop already uses in
the other direction (`CC_SAFETY_STATUS.last_report_sequence` stopping the CRITICAL
repeat), so there is one pattern in the codebase, not two.

### D.3 Explicit non-wiring

`cc_payload_bridge` does **not** subscribe to anything that could influence flight,
does not publish `vehicle_command`, and does not contribute to arming checks. Its
only outputs are one uORB topic and its `print_status()`.

---

## Part E — MAVLink streams

Same gating and semantics as the existing eight **[code, Phase 3 doc]**:

| Stream | Source uORB | Instance | Rate |
|---|---|---|---|
| `CC_PAYLOAD_COMMAND` | `cc_payload_command` | the **CCFC companion** instance | `update()`-only; ceiling set by `mavlink stream -r`, ≥ `CC_PL_REP_HZ` |
| `CC_VITALS_REPORT` | `cc_vitals_report` | the **radio/telemetry** instance (ELRS) — and optionally the GCS instance | `CC_PL_TEL_HZ` (default 1 Hz, 0.2 Hz on constrained links) |
| `CC_PAYLOAD_STATUS` | `cc_payload_status` | radio instance | 0.2 Hz |

Registering the vitals stream on the *radio* instance rather than relying on
`MAV_x_FORWARD` is deliberate: it makes the downlink rate independently
controllable per link, which is exactly what the 868 MHz budget (§G) needs.

---

## Part F — Receiver gauntlet

`mavlink_receiver.{h,cpp}` gains two handlers, reusing the §4.4 gauntlet already
built for `CC_HEALTH_REPORT` — source (`sysid`/`compid = 191`), schema version,
field ranges/enums, sequence monotonicity, and the flood limit — each with a named
counter added to the existing `ccfc_print_stats()` line format:

```
CCFC rx … vitals +N bad_source +N bad_schema +N bad_range +N dup_seq +N flood +N
```

Range checks worth spelling out, because they are the difference between a relay
and a rumour mill: `human_present ≤ 2`, confidences `≤ 100`, `payload_state ≤ 4`,
`resp_rpm_x10 ≤ 900` (90 rpm), `heart_bpm_x10 ≤ 3000` (300 BPM), and
`quality_flags` within the defined mask. A message failing any check is dropped and
counted — it is never published, so it never reaches the log or the downlink.

On success: publish `cc_vitals_report` / `cc_payload_status`. **No other
subscriber.** Add both to `ROMFS/.../logger_topics.txt` at interval `0` (full
rate), matching how `cc_health_report` and `cc_safety_status` are already treated —
they are low-rate and evidentiary.

---

## Part G — The operator link: 868 MHz ELRS

### G.1 Budget

ELRS MAVLink mode forces a 1:2 telemetry ratio, and the documented downlink
figures for the 900 MHz family — which is the 868 MHz hardware family in the EU
domain — are ~880 B/s at 200 Hz Full and ~4420 B/s at K1000 Full (LR1121)
**[code, ExpressLRS docs]**. Against that, a 48 B report **[calc]**:

| Rate | Cost at 880 B/s | Cost at 4420 B/s |
|---|---|---|
| 1 Hz | 5.5 % | 1.1 % |
| 0.2 Hz | 1.1 % | 0.2 % |

Affordable — but the budget is shared with all of PX4's own telemetry, so
`CC_PL_TEL_HZ` exists to tune it per airframe rather than assuming.

### G.2 The EU 868 constraint that changes the design

The EU868 domain uses **Listen Before Talk**: before each transmission ELRS
measures channel activity, and **if the channel is busy that packet interval is
aborted** and the system moves to the next hop **[corrob]**. This is required
because >1 % duty cycle is needed for low latency, and LBT/frequency hopping
provides the polite-spectrum-access route under ETSI EN 300 220 **[corrob]**.
Certified power with LBT is up to 100 mW, with users often at 25 mW **[corrob]**.

Two design consequences, both real:

1. **The downlink loses slots non-deterministically**, so the effective bandwidth
   is below the table and varies with the RF environment. Plan at ~50 % of nominal.
2. **Every report must be self-contained and idempotent.** No deltas, no implied
   state, no "field X refers to the previous message". Each report carries its own
   `dwell_id`, `sequence`, absolute values and `decision_age_ms`, so any single
   packet that arrives is fully interpretable and any number of lost packets costs
   only freshness. The message in §B.3 is designed that way; this is *why*.

### G.3 Getting it in front of the pilot — two paths, and they differ a lot

| Path | How | Verdict |
|---|---|---|
| **GCS over MAVLink-on-ELRS** | QGroundControl/MP on a tablet with the CC dialect loaded; the custom message decodes and can be displayed/logged | **Works with only the PX4 work above.** Recommended for 10.3. Caveat: in MAVLink mode the FC waits to be asked for streams, so **EdgeTX can show nothing until a GCS connects** **[code, ELRS discussion]** |
| **Handset (EdgeTX) only** | ELRS converts MAVLink telemetry into CRSF sensors, and EdgeTX discovers those — but only for the mapped sensor set, so **a custom dialect message will not render** **[corrob]**. PX4's CRSF telemetry additionally requires custom firmware including `crsf_rc` in place of `rc_input`, and supports a fixed set (mode, battery, GPS, RSSI, speed, altitude) **[corrob]** | Needs either a **custom CRSF sensor frame** added to PX4's CRSF telemetry plus an **EdgeTX Lua widget** to display it, or mapping 2–3 scalars into standard sensor slots (which is abuse and will confuse the GCS). Real work — schedule for 10.4 |

Recommended sequencing: build the GCS path first because it needs no new display
code; add the CRSF frame + Lua widget when handset-only operation is actually
required. And decide deliberately what the *minimum* handset display is — realistic
answer: presence (a 3-state icon), respiration, and confidence. Heart rate is the
number most likely to be absent or wrong, so it should never be the headline field.

---

## Part H — Parameters

| Param | Meaning | Default |
|---|---|---|
| `CC_PL_EN` | enable the payload bridge | 0 |
| `CC_PL_RC_CH` | AUX channel carrying start/stop | 0 (disabled) |
| `CC_PL_RC_THR` | threshold (normalised) | 0.5 |
| `CC_PL_RC_DEB` | debounce, ms | 300 |
| `CC_PL_REP_HZ` | command repeat rate until acked | 2 |
| `CC_PL_ACK_TO` | ack timeout, ms | 3000 |
| `CC_PL_TEL_HZ` | vitals downlink rate on the radio instance | 1.0 |
| `CC_PL_MODE` | default capture-mode selector passed in `param` | 0 |

All runtime-updatable, none safety-relevant — consistent with §12's rule that
behaviour is parameterised, never hardcoded.

---

## Part I — Files touched (fork), in the Phase 3 table style

| Path | Change |
|---|---|
| `cc-dialect/cc_dialect.xml` | 4 enums, 3 messages (54014–54016), 1 extension field on 54000 |
| `cc-dialect/golden/` | regenerate golden frames in the **same commit** |
| `msg/CcPayloadCommand.msg`, `msg/CcVitalsReport.msg`, `msg/CcPayloadStatus.msg` | new |
| `msg/CcTelemetryState.msg` | `+ uint8 landed_state` |
| `msg/CMakeLists.txt` | register the three new messages |
| `src/modules/cc_telemetry_publisher/` | subscribe `vehicle_land_detected`, fill `landed_state` |
| `src/modules/cc_payload_bridge/{CcPayloadBridge.hpp,.cpp,params.c,Kconfig,CMakeLists.txt}` | new module |
| `src/modules/mavlink/streams/CC_PAYLOAD_COMMAND.hpp`, `CC_VITALS_REPORT.hpp`, `CC_PAYLOAD_STATUS.hpp` | new stream classes |
| `src/modules/mavlink/mavlink_messages.cpp` | include + register, gated on `MAVLINK_MSG_ID_…` |
| `src/modules/mavlink/mavlink_receiver.{h,cpp}` | two handlers + gauntlet + counters |
| `ROMFS/px4fmu_common/init.d-posix/px4-rc.mavlink` | add the streams to the CCFC and radio instances |
| `ROMFS/px4fmu_common/init.d/logger_topics.txt` | `cc_vitals_report`, `cc_payload_status`, `cc_payload_command` at interval 0 |
| `boards/…/*.px4board` | `CONFIG_MODULES_CC_PAYLOAD_BRIDGE=y` |

**Contract rule (cross-phase):** the XML edit, both generated binding sets, and the
golden vectors land in **one commit**. The `dialect_hash` changes, and the
build-time SHA gate then refuses to build either side against a stale copy — which
is the intended behaviour, not an obstacle.

---

## Part J — Tests

| # | Test | Pass criterion |
|---|---|---|
| J1 | Golden round-trip for the three new messages + the extended 54000 | Field-exact and byte-identical in both C and Rust bindings |
| J2 | Extension compatibility | A pre-extension parser decodes the new `CC_TELEMETRY_STATE` without error and ignores the new field |
| J3 | SITL: RC switch → `cc_payload_command` | `listener cc_payload_command` shows one command per edge, sequence monotonic, no chatter on a noisy channel |
| J4 | SITL: repeat-until-ack | Command repeats at `CC_PL_REP_HZ`; the harness echoes `last_command_seq`; repetition stops |
| J5 | RC-loss drill | No command emitted with `rc_valid = 0`; no capture starts |
| J6 | Receiver gauntlet on vitals | Each invalid class increments exactly its counter and publishes nothing (mirroring the 50/50 Phase 3 result) |
| J7 | Downlink budget | 1 Hz report measured on the radio instance; verify against §G.1 and against a 868 MHz link with LBT active |
| J8 | Cross-log join | `cc_vitals_report` in ULog joins to `radar_vital_estimate` rows in the mission dataset on `(dwell_id, sequence)` — the Phase 6.4 join, extended to the payload |
| J9 | Interlock source | With `landed_state` present, the companion's interlock uses it; with the field absent (old FC), the interlock falls back to **Inhibit**, not to inference |
| J10 | Isolation | Injecting `human_present = 1, confidence = 100` never changes `cc_safety_status`, never issues a `vehicle_command` — asserted, not assumed |

J10 is the one that matters most. It is the mechanical proof of Part A.
