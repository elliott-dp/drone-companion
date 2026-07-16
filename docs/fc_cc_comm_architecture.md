# FC ↔ Companion Computer Communication Architecture Specification

**System:** CUAV V6X (PX4, C++) ↔ Jetson Orin Nano (Rust)
**Transport:** MAVLink 2 custom dialect over TELEM3 UART (Jetson 40-pin header) or Ethernet
**Document version:** 0.1 — working draft
**Status:** Design specification / Interface Control Document (ICD)
**PX4 base:** `PX4-Autopilot-CCFC` pinned to release tag **v1.17.0**
(commit `d6f12ad1c4`; latest stable per dev plan Phase 0.1 — recorded here
2026-07-15, rationale in `phase2_px4_telemetry.md` §A.1)

---

## 0. System Invariants (non-negotiable rules)

These hold in every mode, every state, every failure case. Everything else in this
document is subordinate to them.

1. **PX4 is the sole flight authority.** The Jetson never commands actuators, never
   runs control loops, and never holds state PX4 depends on to fly. The Jetson
   *recommends*; PX4 *decides* via `cc_safety_monitor`'s deterministic policy table.
2. **PX4 must fly correctly with the Jetson absent, crashed, rebooting, or lying.**
   No PX4 code path may block, delay, or degrade the control loop because of
   companion link state. Loss of the Jetson may *restrict capability* (block
   Offboard, trigger a configured failsafe) but never *destabilize flight*.
3. **PX4 code is deterministic:** no dynamic allocation after init, no file I/O in
   the telemetry path, no unbounded loops over inbound messages, bounded CPU per
   cycle, fixed-size messages only.
4. **Safety traffic outranks data traffic.** Heartbeats, TIMESYNC, and health
   reports are never queued behind telemetry or bulk transfer, on either side.
5. **PX4 keeps a local ULog blackbox in every mode.** A Jetson failure must never
   mean "no flight log exists."
6. **Every record is attributable and mergeable:** every message and every log row
   carries identity (vehicle, mission, boot), ordering (sequence), and time
   (FC timestamp + CC receive timestamp + schema version).
7. **Missing data is missing.** No side ever fabricates, interpolates, or
   substitutes values for a stale or absent stream in a way that hides the gap.

---

## 1. Top-Level Architecture

```
             ┌────────────────────────────────────────────┐
             │              Jetson Orin Nano              │
             │                    Rust                    │
             │                                            │
             │  cc-link        cc-timesync                │
             │  cc-ingest      cc-mission-log             │
             │  cc-ai-health   cc-health-tx               │
             │  cc-replay      cc-config                  │
             └───────────────▲────────────────────────────┘
                             │
                             │ MAVLink 2 custom dialect
                             │ HEARTBEAT + TIMESYNC + telemetry
                             │ + health reports + safety status
                             ▼
             ┌────────────────────────────────────────────┐
             │              CUAV V6X / PX4                │
             │                    C++                     │
             │                                            │
             │  mavlink_receiver.cpp (extended)           │
             │  MavlinkStreamCcTelemetry* classes         │
             │  cc_telemetry_publisher (uORB → uORB)      │
             │  cc_safety_monitor (policy, deterministic) │
             │  custom uORB topics (msg/Cc*.msg)          │
             │  PX4 logger / ULog (blackbox policy)       │
             └────────────────────────────────────────────┘
```

**Division of labor (fixed):**

| Concern | PX4 / V6X (C++) | Jetson Orin Nano (Rust) |
|---|---|---|
| Flight control, estimation, failsafes | ✅ owns | ❌ never |
| Telemetry curation & rate control | ✅ `cc_telemetry_publisher` | ❌ |
| Wire protocol encode/decode | ✅ mavlink module | ✅ `cc-protocol` / `cc-link` |
| Health inference / AI | ❌ never | ✅ `cc-ai-health` |
| Policy reaction to health reports | ✅ `cc_safety_monitor` | ❌ (proposes only) |
| Mission dataset (high-rate, columnar) | ❌ | ✅ `cc-mission-log` |
| Safety blackbox (crash/debug) | ✅ ULog | ❌ (references it only) |
| Time sync master clock | FC monotonic µs since boot | CC monotonic ns + UTC wall clock |

**Log ownership (the dedup rule in one line each):**
- PX4 owns: safety blackbox, estimator/control debugging, crash reconstruction,
  arming/failsafe evidence.
- Jetson owns: mission dataset, AI training/inference logs, long-horizon telemetry,
  mission indexing, health findings, replayable autonomy context.
- Both share: `px4_boot_id`, `mission_id`, per-stream `sequence`, timestamps —
  so the two logs can be joined offline without guessing.

---

## 2. Physical & Transport Layer

### 2.1 Primary link: TELEM3 UART → Jetson 40-pin header

- **FC port:** V6X TELEM3 (JST-GH). Pinout: VCC, TX, RX, GND (verify whether
  your V6X carrier exposes CTS/RTS on TELEM3 — many carriers route flow-control
  lines only on TELEM1/TELEM2; the design below assumes **no flow control**).
- **CC port:** Jetson Orin Nano Developer Kit 40-pin header, UART1:
  - pin 8 = UART1_TXD, pin 10 = UART1_RXD, pin 6 (or any GND pin) = GND
  - optional flow control if TELEM3 provides it: pin 11 = UART1_RTS,
    pin 36 = UART1_CTS
  - Linux device: `/dev/ttyTHS0` on Orin Nano with recent JetPack — confirm
    with `ls /dev/ttyTHS*` on your image and pin it in `cc-config`.
  - **Mandatory Jetson prep:** disable the serial console on that UART or the
    kernel will inject login prompts into the MAVLink stream
    (`systemctl stop nvgetty; systemctl disable nvgetty` and/or
    `systemctl disable serial-getty@ttyTHS0.service`, then reboot), and add the
    service user to the `dialout` group.
- **Wiring:** FC TX → Jetson RX (pin 10), FC RX → Jetson TX (pin 8), GND ↔ GND.
  Cross-check with a scope/loopback before first connect; a swapped pair is
  harmless but the most common "no heartbeat" cause.
- **Electrical:** both sides are 3.3 V TTL — direct connection allowed,
  **common ground mandatory**. Do not power the Jetson from TELEM3 VCC; the
  Jetson has its own regulated supply. Only TX/RX/(CTS/RTS)/GND cross the
  boundary.
- **Baud:** 921600 as the working default; 1,500,000 or 3,000,000 only after a
  bench soak test at full telemetry load with zero CRC errors for ≥30 min.
- **Framing:** 8N1.
- **Flow control:** default **off** (TELEM3 + 40-pin header rarely both wire
  RTS/CTS). Without flow control the rate budget in §8 must leave ≥40%
  headroom — the mission profile sits near 9%, so this is comfortably met.
  If both ends do expose RTS/CTS, enabling it is a free robustness win.
- **Cabling:** twisted pair for TX/RX with ground, kept away from ESC power runs;
  shielded if the run passes near motors. Any CRC error rate above ~1 in 10⁵
  frames on the bench is a wiring problem to fix, not a software problem to
  tolerate.
- **PX4 configuration:** dedicate a MAVLink instance to TELEM3 in
  **Onboard mode** (`MAV_1_CONFIG = TELEM3` (value 103), `MAV_1_MODE = Onboard`,
  baud via `SER_TEL3_BAUD`, `MAV_1_RATE` sized per §8, `MAV_1_FORWARD = 0`
  unless the Jetson must relay to a GCS). Onboard mode matters: it selects a
  companion-appropriate default stream set and heartbeat expectations.
  Verify the NuttX device node TELEM3 maps to on the V6X board config
  (`boards/cuav/v6x/default.px4board`) before hardcoding any `/dev/ttySx` in
  rcS stream commands.

### 2.2 Alternative link: Ethernet (MAVLink over UDP)

- V6X exposes 100 Mbps Ethernet; Jetson native GbE. Static IPs on an isolated
  vehicle subnet (e.g., FC `10.41.0.2`, CC `10.41.0.1`, /24, no DHCP, no gateway).
- PX4 MAVLink instance on UDP (e.g., `MAV_2_CONFIG` → network port, remote port
  14550-range but use a dedicated pair, e.g., FC:14540 ↔ CC:14541) in Onboard mode.
- UDP is lossy by design — that is acceptable because the protocol already
  assumes loss (sequence numbers, staleness timeouts, no retransmission for
  real-time streams).
- **Design rule:** the architecture must remain correct on UART. Ethernet only
  raises rate ceilings (§9); it changes no message, no state machine, no policy.

### 2.3 Transport selection & fallback

- Link type is a deployment configuration (`cc-config` on Jetson, `MAV_x_CONFIG`
  on PX4), not a runtime negotiation.
- No automatic UART↔Ethernet failover in v1. If dual links are ever added, they
  must be two PX4 MAVLink instances with the Jetson deduplicating by
  `(stream_id, sequence)` — explicitly out of scope for now.

### 2.4 Power & boot independence

- FC and Jetson power up independently; **neither side blocks boot on the other**.
- FC typically ready in ~2–5 s; Jetson in ~30–60 s. PX4 runs normally (streams
  idle or unconsumed) until the first Jetson HEARTBEAT arrives.
- Jetson brownout/reboot mid-flight = "link lost" case (§11); nothing more.
- Hot-plug of the UART must not wedge either parser: both resynchronize by
  scanning for the MAVLink 2 STX byte (0xFD) and re-validating CRC.

---

## 3. Protocol Layer: MAVLink 2 Custom Dialect

### 3.1 Dialect definition

- One XML file is the **single source of truth**: `cc_dialect.xml`, including
  `common.xml` (so HEARTBEAT, TIMESYNC, COMMAND_LONG/ACK, STATUSTEXT remain
  available).
- Generated artifacts:
  - **PX4/C:** mavgen C headers, vendored into the PX4 mavlink module the same
    way PX4 handles `development.xml`/custom dialects.
  - **Jetson/Rust:** `rust-mavlink` with the dialect XML fed to its build-time
    generator (`mavlink` crate custom dialect feature) — `cc-protocol` wraps
    the generated types; no other crate touches raw generated code.
- **CI rule:** both bindings are regenerated from the same XML at the same commit;
  a hash of the XML is embedded as `schema_version`'s companion
  (`dialect_hash`) in the mission manifest. If PX4 and Jetson disagree on
  `CRC_EXTRA` for any message, that message silently fails CRC — this is why
  generation must never be done by hand on one side only.

### 3.2 Message ID allocation

- MAVLink 2 has 24-bit message IDs. Reserve a private block far above every
  public dialect: **54000–54099**.

| ID | Message | Direction | Class |
|---|---|---|---|
| 54000 | `CC_TELEMETRY_STATE` | FC → CC | A |
| 54001 | `CC_TELEMETRY_IMU` | FC → CC | B |
| 54002 | `CC_TELEMETRY_POWER` | FC → CC | C |
| 54003 | `CC_TELEMETRY_GPS` | FC → CC | D |
| 54004 | `CC_TELEMETRY_ESTIMATOR` | FC → CC | E |
| 54005 | `CC_TELEMETRY_ACTUATOR` | FC → CC | F |
| 54006 | `CC_EVENT` | FC → CC | G |
| 54007 | `CC_SAFETY_STATUS` | FC → CC | ack/status |
| 54008 | *(reserved: `CC_TELEMETRY_ESC`)* | FC → CC | future — only if telemetry-capable ESCs are ever fitted |
| 54010 | `CC_HEALTH_REPORT` | CC → FC | health |
| 54011 | `CC_AI_DIAGNOSTIC` | CC → FC | low-rate detail |
| 54012 | `CC_MISSION_CONTEXT` | CC → FC | session |
| 54013 | `CC_LOG_CONTROL` | CC → FC | optional |

- IDs are **never reused or renumbered**. Deprecated messages keep their ID
  forever; new fields go at the end of a message (MAVLink 2 zero-truncation
  keeps old parsers compatible) or into a new message with a new ID.

### 3.3 Addressing

- Same MAVLink **system ID** for both (they are one vehicle): `sysid = 1`
  (or `MAV_SYS_ID`).
- Component IDs: PX4 autopilot = `MAV_COMP_ID_AUTOPILOT1` (1); Jetson =
  `MAV_COMP_ID_ONBOARD_COMPUTER` (191).
- Every Jetson-originated message sets `source_component = 191`; PX4's receiver
  discards CC_* command/health messages from any other component ID
  (`reject_reason = BAD_SOURCE`).

### 3.4 Common-envelope fields (in every custom payload)

Per the identity contract, every CC_* message carries:

```
schema_version   uint8    bumped on any field-semantics change
sequence         uint32   per-stream monotonic counter, wraps at 2^32
fc_timestamp_us  uint64   FC monotonic time (µs since PX4 boot) — FC-originated msgs
```

and every Jetson log row additionally records on receipt:

```
cc_receive_time_ns   CC CLOCK_MONOTONIC at frame receipt
vehicle_id, mission_id, px4_boot_id, cc_boot_id, stream_id
estimated_offset_us, timesync_rtt_us, timesync_quality (from cc-timesync)
```

Raw FC timestamps are **never overwritten** by corrected ones — both are stored.

### 3.5 Heartbeat contract

- **PX4 → CC:** standard `HEARTBEAT` at 1 Hz (already emitted by the mavlink
  instance). Carries `custom_mode`/`base_mode` → nav/arming context redundancy.
- **CC → PX4:** standard `HEARTBEAT` at 1 Hz, `type = MAV_TYPE_ONBOARD_CONTROLLER`,
  `autopilot = MAV_AUTOPILOT_INVALID`.
- `cc_safety_monitor` staleness clock (§6) is driven by **CC_HEALTH_REPORT**
  arrivals, not raw heartbeats — a Jetson whose `companiond` health pipeline is
  dead but whose link task still heartbeats must still be judged STALE.
  Heartbeat presence is tracked separately as `link_alive` for diagnostics.

### 3.6 What is deliberately NOT in the protocol (v1)

- No setpoint streaming / Offboard control messages (the monitor only *gates*
  Offboard; nothing here *drives* it).
- No parameter write from CC → FC (parameter *snapshot read* only, via standard
  PARAM protocol, at mission start).
- No arming/disarming commands from the CC — ever. `cc_safety_monitor` may issue
  Hold/Land/RTL `vehicle_command`s per its policy table; the CC itself may not.
- No file transfer (FTP), no camera/tensor/JSON payloads, no variable-length
  diagnostics toward PX4. Anything ≥ the MAVLink 2 payload cap (253 B) toward
  PX4 is a design error by definition.

---

## 4. PX4-Side C++ Modules

### 4.1 Source layout

```
src/modules/mavlink/
  mavlink_receiver.cpp        (extended: CC_HEALTH_REPORT, CC_MISSION_CONTEXT, ...)
  mavlink_messages.cpp        (stream registration)
  streams/CC_TELEMETRY_STATE.hpp
  streams/CC_TELEMETRY_IMU.hpp
  streams/CC_TELEMETRY_POWER.hpp
  streams/CC_TELEMETRY_GPS.hpp
  streams/CC_TELEMETRY_ESTIMATOR.hpp
  streams/CC_TELEMETRY_ACTUATOR.hpp
  streams/CC_EVENT.hpp
  streams/CC_SAFETY_STATUS.hpp
src/modules/cc_telemetry_publisher/
  CcTelemetryPublisher.{hpp,cpp}
  CMakeLists.txt
  Kconfig
src/modules/cc_safety_monitor/
  CcSafetyMonitor.{hpp,cpp}
  cc_policy_table.hpp
  CMakeLists.txt
  Kconfig
msg/
  CcTelemetryState.msg  CcTelemetryImu.msg  CcTelemetryPower.msg
  CcTelemetryGps.msg    CcTelemetryActuator.msg  CcTelemetryEstimator.msg
  CcHealthReport.msg    CcSafetyStatus.msg
  (all registered in msg/CMakeLists.txt)
```

Both modules start from the airframe rcS extras, run on the PX4 work queue
(`ScheduledWorkItem` on `wq:lp_default` for the publisher, `wq:nav_and_controllers`
or `lp_default` for the monitor — the monitor is not latency-critical at
sub-10 ms scale), and register `print_status()` for `cc_telemetry_publisher status`
style introspection.

### 4.2 `cc_telemetry_publisher` — exact behavior

**One job:** deterministic field selection + rate control from existing uORB
topics into compact `cc_telemetry_*` uORB topics. Nothing else.

- **Scheduling:** runs at the highest output rate needed (50 Hz base tick for
  UART profile; 200 Hz tick permitted only in `AI_ETH` profile). Each output
  stream keeps its own decimation counter derived from the `CC_TEL_*` params /
  `CC_TEL_PROFILE`.
- **Sampling rule:** on each tick, `copy()` the newest sample of each input
  topic (uORB latest-value semantics); never queue history; never block.
- **Per-stream sequence:** each output topic has its own `uint32 sequence`,
  incremented on publish, never reset except on FC reboot (new `px4_boot_id`).
- **`px4_boot_id`:** generated once at module start (e.g., from RTC seconds at
  boot XOR a persisted counter, or simply a random 32-bit drawn at init and
  logged); constant for the boot; included in `CC_TELEMETRY_STATE` and the
  mission handshake so both logs can key on it.
- **Prohibitions (enforced in review):** no heap allocation after `init()`,
  no file I/O, no float→string formatting, no per-message conditionals that
  depend on Jetson state (PX4 publishes whether or not anyone listens; the
  MAVLink stream layer already handles "no subscriber" cheaply).
- **CPU budget:** target < 1% of one core at UART profile; measured via
  `top`/`perf counters` in SITL and on hardware before merge.

### 4.3 `MavlinkStreamCcTelemetry*` classes

- Standard PX4 pattern: each class subscribes to one `cc_telemetry_*` topic,
  `get_size()` returns 0 when no fresh data (so idle streams cost ~nothing),
  `send()` copies uORB → mavlink struct → `mavlink_msg_*_send_struct()`.
- Stream rates are configured on the TELEM2 instance at startup
  (`mavlink stream -d /dev/ttyS2 -s CC_TELEMETRY_STATE -r 25`, etc., emitted
  from rcS according to `CC_TEL_PROFILE`), so rate lives in **one** place
  (publisher decimation defines the ceiling; mavlink stream rate ≤ that ceiling).
- `CC_EVENT` and `CC_SAFETY_STATUS` are event-driven: `get_size()` nonzero only
  when a new uORB sample exists.

### 4.4 `mavlink_receiver.cpp` extension — inbound validation

Handles, from component 191 only:

| Inbound MAVLink | Action |
|---|---|
| `CC_HEALTH_REPORT` | validate → publish `cc_health_report` uORB |
| `CC_AI_DIAGNOSTIC` | validate → log-only uORB (never drives policy) |
| `CC_MISSION_CONTEXT` | store mission_id/cc_boot_id → publish for logger + echo into `CC_TELEMETRY_STATE` context |
| `CC_LOG_CONTROL` (optional) | request logger profile change (dev only, param-gated) |

**Validation gauntlet, in order, before any uORB publish:**
1. MAVLink CRC (implicit — parser drops bad frames; counted).
2. `source_component == 191`, `sysid` matches own.
3. `schema_version` supported (else count + drop, and raise one throttled
   `events` warning per boot, not per message).
4. Payload range checks: severity ≤ 3, recommended_action ≤ 5,
   confidence ≤ 100, enum fields within definition.
5. Sequence check: new sequence must be > last (mod 2³²) within a window;
   duplicates dropped; gaps counted into `missed_reports`.
6. Rate check: > 20 CC_HEALTH_REPORT/s → treat as flooding; drop excess, count,
   raise throttled warning (a compromised/buggy Jetson must not be able to spam
   the work queue).

Failing any step increments a named counter surfaced by `print_status()` and,
where relevant, `CcSafetyStatus.reject_reason`.

### 4.5 `cc_safety_monitor` — the deterministic policy core

**Inputs:** `cc_health_report`, `vehicle_status`, `vehicle_control_mode`,
`estimator_status_flags`, `battery_status`, parameters (§12).
**Outputs:** `cc_safety_status` (uORB → MAVLink echo to Jetson), optional
`vehicle_command` (Hold/Land/RTL), PX4 `events` warnings.

**Companion state machine:**

```
                 first valid report
   UNKNOWN ────────────────────────────► (OK|WARN|CRITICAL per report)
      ▲                                          │
      │ FC reboot                                │ report age > CC_MON_TIMEOUT_MS
      │                                          ▼
      └──────────────                          STALE
                                                 │ CC_MON_OK_COUNT consecutive
                                                 │ fresh OK reports
                                                 ▼
                                                OK
```

- **UNKNOWN:** boot state. If `CC_MON_REQ_OFFB=1`, Offboard is blocked.
- **OK / WARN / CRITICAL:** taken directly from the latest *valid* report's
  severity, with hysteresis: escalation is immediate; de-escalation from
  CRITICAL/WARN→OK requires `CC_MON_OK_COUNT` consecutive OK reports.
- **STALE:** entered when no valid report for `CC_MON_TIMEOUT_MS`
  (default 3000 ms). Exiting STALE requires `CC_MON_OK_COUNT` fresh OKs.
- **Report acknowledgement:** `CC_SAFETY_STATUS.last_report_sequence` echoes the
  highest accepted report sequence — this *is* the ack that stops the Jetson's
  5 Hz CRITICAL repeat.

**Policy table (`cc_policy_table.hpp`), pure function
`(companion_state, recommended_action, flight_context, params) → action`:**

| Companion state | Recommended action | In Offboard? | Armed & airborne? | Monitor action |
|---|---|---|---|---|
| CRITICAL | LAND / RTL / HOLD | any | yes | execute `CC_MON_CRIT_ACT` (param wins over recommendation; recommendation logged) |
| CRITICAL | any | any | no (on ground) | block arming into autonomy modes*, warn |
| WARN | any | any | any | warn (events + STATUSTEXT), never auto-act |
| STALE | n/a | yes | yes | execute `CC_MON_STALE_ACT` (default: exit Offboard → Hold) |
| STALE | n/a | no | any | block Offboard entry if `CC_MON_REQ_OFFB`, otherwise no action |
| OK | BLOCK_OFFBOARD | any | any | honor: block/exit Offboard, warn |
| UNKNOWN | n/a | — | — | block Offboard entry if `CC_MON_REQ_OFFB` |

\* "block arming into autonomy modes" means: the monitor contributes a health
component flag consumed by PX4's arming checks (registered like other
health/arming check items), **only** gating autonomy-dependent arming, never
manual-mode arming — the pilot can always fly manually.

**Hard rules encoded in the monitor:**
- The monitor may only ever move the vehicle toward *more conservative* states
  (Hold, Land, RTL, block/exit Offboard). It has no code path to arm, take off,
  switch into Offboard, or increase authority.
- One action per state transition (edge-triggered), not per report
  (level-triggered) — a CRITICAL repeated at 5 Hz triggers Land once, not 5×/s.
- Pilot RC mode change always overrides: if the pilot switches modes after a
  monitor action, the monitor does not re-command until a *new* state
  transition occurs.
- All eight parameters (§12) re-read on param update; no reboot needed.

### 4.6 PX4 logger (ULog blackbox) policy

- Profiles selected by `CC_TEL_PROFILE` + `SDLOG_PROFILE`:
  - **Development:** default/full ULog (plus high-rate as needed for a test).
  - **Mission AI:** `logger_topics.txt` (SD card) minimal blackbox:

    ```
    vehicle_status 100
    vehicle_attitude 50
    vehicle_local_position 100
    battery_status 200
    actuator_outputs 100
    estimator_status 200
    failsafe_flags 0
    cc_health_report 0
    cc_safety_status 0
    ```

    (interval = min ms between logged samples; 0 = full rate — health/safety
    topics are low-rate and evidentiary, so full rate is correct for them).
- ULog also captures the parameter set and `events` automatically — that is the
  arming/failsafe evidence record.
- After each flight, the Jetson stores a `px4_ulog_reference.json` (ULog file
  name, size, hash if transferred, `px4_boot_id`) in the mission directory so
  the two records stay joinable even if the .ulg stays on the SD card.

---

## 5. Jetson-Side Rust Architecture

### 5.1 Workspace layout

```
drone-companion/
  Cargo.toml                  (workspace)
  dialect/cc_dialect.xml      (single source of truth, shared with PX4 repo via
                               submodule or vendored copy pinned by hash)
  crates/
    cc-protocol/   generated dialect bindings wrapper, envelope types, validation
    cc-link/       transport, framing, priority TX queues, link stats
    cc-timesync/   MAVLink TIMESYNC client, offset/RTT estimator
    cc-ingest/     decode → validate → normalize → TelemetryEvent fan-out
    cc-mission-log/ mission directories, Parquet writers, manifest, rotation
    cc-ai-health/  health algorithms → HealthFinding
    cc-health-tx/  finding aggregation, hysteresis, report rate policy
    cc-replay/     offline replay harness
    cc-config/     layered config (file + env + CLI), profiles
  apps/
    companiond/    the runtime daemon (tokio)
    log-inspect/   CLI: open a mission, print stats, verify integrity
    replay-mission/ CLI: replay a mission through cc-ai-health
```

### 5.2 Runtime model (`companiond`)

- **Async runtime:** tokio, multi-threaded. Task graph:

```
[serial/udp RX] → cc-link RX task → bounded mpsc → cc-ingest task
                                              ├─→ broadcast: TelemetryEvent
                                              │      ├─→ cc-mission-log writer task
                                              │      └─→ cc-ai-health task(s)
cc-ai-health → findings mpsc → cc-health-tx task → cc-link TX (priority queues)
cc-timesync task ⇄ cc-link TX/RX (priority 0)
supervisor task: watches every task handle, restarts or degrades per policy
```

- **All channels are bounded.** Overflow policy per consumer:
  - mission-log queue full → *backpressure the AI fan-out never, the logger
    buffers to a larger spill queue*; if the spill exceeds its cap, drop
    lowest-class telemetry rows first and count drops into `events.parquet`.
  - ai-health queue full → drop oldest window samples, degrade that algorithm's
    confidence, emit LinkStatus/AI-degraded event (per §11).
- **No task may block on disk I/O in the RX path.** Only the mission-log writer
  task touches disk.
- **Process supervision:** systemd unit with `Restart=on-failure`,
  `WatchdogSec=10` (daemon pets via `sd_notify`), and `OOMScoreAdjust` set so
  the OS kills bulk consumers before `companiond`.
- **Crash behavior:** on restart mid-mission, `companiond` opens a *new* mission
  segment directory (`.../segment_02/`) rather than appending to possibly-torn
  files; `cc_boot_id` changes; manifest links segments.

### 5.3 `cc-link` specifics

- Owns exactly one transport at a time (serial `/dev/ttyTHS*` @ configured baud,
  or UDP socket pair).
- **RX:** incremental MAVLink 2 parser; on garbage, resync on 0xFD; counters:
  `rx_frames`, `rx_crc_errors`, `rx_bad_source`, `rx_unknown_msg`,
  per-stream `sequence_gaps`.
- **TX priority queues** (strict priority, preemptive at frame boundary):
  - P0: HEARTBEAT, TIMESYNC, CC_HEALTH_REPORT
  - P1: CC_MISSION_CONTEXT, acks/session
  - P2: CC_AI_DIAGNOSTIC
  - P3: bulk/debug (dev only)
  P0 queue is small and *never* dropped; if P0 cannot drain, that is a
  link-down condition, not a queueing condition.
- **Reconnect:** serial: reopen with exponential backoff (100 ms → 2 s cap);
  UDP: socket is connectionless, "reconnect" = peer heartbeat reappearing.
  Link state (UP/DEGRADED/DOWN) derives from: PX4 heartbeat age, CRC error
  rate, and TIMESYNC RTT; published internally as `LinkStatus` events (which
  are themselves logged).

### 5.4 `cc-timesync` specifics

- Standard MAVLink `TIMESYNC` exchange: CC sends `tc1=0, ts1=cc_mono_ns`;
  PX4 replies with its timestamp; offset ≈ ((tc1_reply − ts1) + (tc1_reply −
  ts_rx))/2-style RTT-compensated estimate.
- Cadence: 10 Hz for the first 5 s after link-up (fast lock), then 1 Hz.
- Filter: keep a rolling window (e.g., 32 samples); reject samples with RTT >
  p90 of window × 1.5; estimate offset by median; expose
  `timesync_quality ∈ {LOCKED, DEGRADED, UNLOCKED}` with thresholds on RTT
  jitter and sample rejection rate.
- Exposes pure conversion functions both directions; consumers (logger, AI)
  read a snapshot (offset, rtt, quality) — they never re-derive.
- On FC reboot detection (px4_boot_id change or FC timestamp going backwards):
  invalidate the estimate, relock, and mark the discontinuity in
  `timesync.parquet`.

### 5.5 `cc-ingest` specifics

- Decodes only messages in the dialect; unknown IDs → counted, ignored, never a
  crash (forward compatibility).
- Per-stream continuity: tracks `last_sequence`; gap → `sequence_gaps += n`,
  logged as an event row (not per-row spam).
- Message age = `now_mono − timesync.to_cc(fc_timestamp_us)` when LOCKED, else
  receive-time only, with age flagged `unknown_offset`.
- **Stream watchdogs:** each Class A–F stream has an expected-rate window; a
  stream silent for > 4 × its nominal period ⇒ `StreamStale(stream_id)` event
  ⇒ dependent AI algorithms become UNAVAILABLE (never fed stale data silently).
- Single output type (extended from the base spec with link/stream meta):

```rust
pub enum TelemetryEvent {
    State(CcTelemetryState), Imu(CcTelemetryImu), Power(CcTelemetryPower),
    Gps(CcTelemetryGps), Actuator(CcTelemetryActuator),
    Estimator(CcTelemetryEstimator), SafetyStatus(CcSafetyStatus),
    Event(CcEvent), LinkStatus(LinkStatus), StreamStale(StreamId),
}
```

### 5.6 `cc-mission-log` specifics

- Mission directory per §7 layout; `manifest.json` written first (crash-safe:
  write temp + fsync + rename), updated at segment boundaries and mission end.
- **Format:** Parquet via `arrow-rs`/`parquet` crate; one writer per stream;
  row groups flushed every N rows or T seconds (e.g., 5000 rows / 10 s),
  fsync on flush. `raw_mavlink.bin` is a length-prefixed frame capture with CC
  receive timestamps — the ground truth if a decoder bug is ever suspected.
- Every row carries the full identity envelope (§3.4 + §7).
- **Disk budget:** startup check — refuse to start a mission below a configured
  free-space floor (e.g., 5 GB); during mission, low-disk triggers class-based
  shedding (stop raw_mavlink first, then Class B/F rows) + WARN health flag;
  never silently stop writing everything.
- Rotation: segments capped (e.g., 2 GB or 30 min) to bound loss from any
  single torn file.
- Mission end: finalize writers, complete manifest (row counts, drop counts,
  time range, dialect hash, software versions), mark `complete=true`.

### 5.7 `cc-ai-health` specifics

- Algorithms per domain file (battery_model, motor_balance, vibration_anomaly,
  gps_quality, estimator_consistency, thermal_monitor, link_quality,
  mission_risk), each implementing one trait:

```rust
pub trait HealthAlgorithm {
    fn required_streams(&self) -> &'static [StreamId];
    fn on_event(&mut self, ev: &TelemetryEvent, ctx: &MissionCtx);
    fn evaluate(&mut self, now_ns: u64) -> SmallVec<[HealthFinding; 4]>;
    fn availability(&self) -> Availability; // OK / DEGRADED / UNAVAILABLE(+why)
}
```

- Evaluation tick decoupled from ingest (e.g., 10 Hz evaluate; ingest feeds
  rolling windows continuously).
- An algorithm whose `required_streams` include a stale stream reports
  UNAVAILABLE; the aggregate never invents findings from missing data.
- Findings carry `confidence`; the runtime envelope is bounded: model
  inference (if any) runs in its own task with a deadline; a blown deadline =
  DEGRADED availability, never a stalled pipeline.

### 5.8 `cc-health-tx` specifics

- Merges findings → single worst-severity report + `health_flags` bitmask +
  dominant `detail_code`.
- **Hysteresis:** severity may rise instantly; may fall only after the
  underlying findings stay lower for a hold time (e.g., 3 s) — mirrors the FC
  monitor's `CC_MON_OK_COUNT` so the two never oscillate against each other.
- Rates: OK 1 Hz; WARN 2–5 Hz; CRITICAL immediate on transition + 5 Hz repeat
  **until** `CC_SAFETY_STATUS.last_report_sequence ≥` that report's sequence
  (the ack), then fall back to the steady rate for the current severity.
- Also embeds self-telemetry each report: `link_rtt_ms`, `telemetry_age_ms`,
  `companion_loop_ms`, `dropped_rx_count` — PX4 logs these as evidence of
  companion state at any incident.

### 5.9 `cc-replay` specifics

- Input: a mission directory (or raw_mavlink.bin). Reconstructs the
  `TelemetryEvent` stream with original timing (or accelerated ×N), feeds
  `cc-ai-health` builds A and B, diffs `HealthFinding` streams, and emits a
  report: new findings, vanished findings, severity/confidence deltas,
  timing deltas.
- Determinism requirement on cc-ai-health: same input events + same config ⇒
  same findings (no wall-clock reads inside algorithms; time comes from the
  event stream). This is what makes regression testing and the false-positive
  audit possible.

---

## 6. Real-Time Telemetry Contract (FC → CC)

The contract is the class table. PX4 sends these and only these; every field is
fixed-size; anything not listed lives in ULog, not on the link.

| Class | Message | UART rate | ETH rate (post-soak) | Notes |
|---|---|---|---|---|
| A | CC_TELEMETRY_STATE | 25 Hz | 50 Hz | nav/arming/failsafe, quat, rates, local pos/vel, heading — AI context & phase detection |
| B | CC_TELEMETRY_IMU | 50 Hz summary | 100–200 Hz | accel/gyro, delta-angle/vel, vibration metrics, clipping, temp. Raw full-rate IMU stays in ULog until link+CPU margin is *measured* |
| C | CC_TELEMETRY_POWER | 10 Hz | 20 Hz | V/I/P, mAh, remaining, cells, temp, warning |
| D | CC_TELEMETRY_GPS | 5 Hz | 10 Hz | fix, sats, lat/lon/alt, eph/epv, speed, heading, noise, jamming if available |
| E | CC_TELEMETRY_ESTIMATOR | 10 Hz | 20 Hz | status/innovation flags, test ratios — worth more to health AI than raw sensor spam |
| F | CC_TELEMETRY_ACTUATOR | 20 Hz | 50 Hz | commanded outputs[8] + motor_count only. **This vehicle's ESCs provide no telemetry**, so RPM/current/temp fields are excluded from the message entirely (ID 54008 reserved for a future `CC_TELEMETRY_ESC`). Motor-health AI must work from commanded output × battery power × vibration correlation, not per-motor feedback |
| G | CC_EVENT | event-driven | event-driven | id, severity, subsystem, 2 args — events beat re-logging static status |
| — | CC_SAFETY_STATUS | on change + 1 Hz | same | monitor state echo + report ack |

**Field-level definitions:** the uORB `.msg` files of the source design
(CcTelemetryState, CcTelemetryPower, CcHealthReport, CcSafetyStatus) are
adopted as the field contract, with one deviation: **CcTelemetryActuator is
trimmed to `timestamp, sequence, motor_count, actuator_output[8]`** because the
fitted ESCs have no telemetry. The MAVLink dialect (`cc_dialect.xml`) mirrors
the uORB definitions 1:1. Units/frames fixed once: NED local frame, quaternion
(w,x,y,z), radians, µs FC-boot-relative timestamps, volts/amps/°C, WGS-84
lat/lon ×1e7 as int32, alt mm AMSL as int32.

---

## 7. Mission Identity, Session Handshake & Dedup

**Identity fields in every log record:** `vehicle_id, mission_id, px4_boot_id,
cc_boot_id, fc_timestamp_us, cc_receive_time_ns, stream_id, sequence,
schema_version`.
**Dedup/join key:** `(vehicle_id, mission_id, px4_boot_id, stream_id, sequence)`.

**Who mints what:**
- `vehicle_id` — static config, same value in PX4 param and Jetson config;
  mismatch at handshake = configuration error, mission refused.
- `mission_id` — minted by the **Jetson** (it owns mission storage): 32-bit,
  monotonic per vehicle (persisted counter), sent in `CC_MISSION_CONTEXT`.
- `px4_boot_id` — minted by PX4 at boot (§4.2), echoed to Jetson in telemetry.
- `cc_boot_id` — minted by `companiond` at process start.

**Session handshake (after link-up + timesync lock):**
1. Jetson sends `CC_MISSION_CONTEXT{mission_id, cc_boot_id, vehicle_id,
   schema_version, dialect_hash}` at 1 Hz until PX4 acknowledges (via
   `CC_SAFETY_STATUS` leaving UNKNOWN, or explicit context echo).
2. PX4 validates vehicle_id + schema, stores context, includes mission_id in
   its own ULog via the logged `cc_*` topics.
3. Jetson performs a standard PARAM read snapshot → `px4_params_snapshot.json`.
4. Mission directory + manifest created; telemetry logging begins.

A PX4 mid-mission reboot (new `px4_boot_id`) or companiond restart (new
`cc_boot_id`) starts a new **segment** under the same mission_id, never a
silent continuation.

---

## 8. Rate & Bandwidth Budget (UART worst case)

Approximate payload sizes (fields per §6 + envelope) + MAVLink 2 overhead
(12 B header/CRC, no signing on a wired private link):

| Stream | ~frame B | Hz | B/s |
|---|---|---|---|
| STATE | 90 | 25 | 2 250 |
| IMU summary | 70 | 50 | 3 500 |
| POWER | 45 | 10 | 450 |
| GPS | 60 | 5 | 300 |
| ESTIMATOR | 55 | 10 | 550 |
| ACTUATOR | 60 | 20 | 1 200 |
| EVENT / SAFETY / HEARTBEAT / TIMESYNC | — | — | ≈ 300 |
| **Total FC→CC** | | | **≈ 8.6 kB/s ≈ 86 kbaud** |

At 921 600 baud (≈ 92 kB/s effective), the mission profile uses ~9% of the
link — comfortably inside the ≥30% (with flow control) / ≥40% (without)
headroom rule even after adding margin for bursts and events. CC→FC traffic
(reports + timesync + heartbeat) is < 500 B/s and negligible. Ethernet rates
in §6 stay far below 1% of a 100 Mbps link; the constraint there is PX4 CPU,
not bandwidth — which is why ETH rates still require a measured soak test.

**Budget rule:** any new stream or rate increase must update this table first
and re-verify headroom on the bench (CRC-error-free 30 min soak at full load).

---

## 9. Logging Modes & Duplication Policy

| | Mode 1 — Development | Mode 2 — Mission AI (target) | Mode 3 — Jetson unavailable |
|---|---|---|---|
| PX4 ULog | full (or high-rate per test) | minimal blackbox (§4.6 topic list) | normal ULog |
| Jetson | full mission dataset | primary high-rate dataset + AI findings + raw capture | absent |
| Duplication | accepted (Jetson code unproven) | controlled: overlap only on low-rate safety topics | n/a |
| Autonomy | per test plan | allowed if companion OK | Offboard blocked if `CC_MON_REQ_OFFB` |

Selected via `CC_TEL_PROFILE` + `SDLOG_PROFILE` + logger_topics.txt.
Invariant 5 applies: Mode 3 must always leave a complete PX4 flight log.

---

## 10. Data-Flow Sequences (normative)

**Cold start:**
1. PX4 boots → uORB, mavlink (TELEM2 onboard instance), logger,
   cc_telemetry_publisher, cc_safety_monitor start. Monitor = UNKNOWN.
2. PX4 publishes telemetry regardless of listener (streams are cheap unsent).
3. Jetson boots → systemd starts companiond → cc-link opens transport.
4. HEARTBEATs seen both ways → link UP.
5. cc-timesync fast-locks (10 Hz for 5 s) → LOCKED.
6. Handshake per §7 → mission directory + manifest + param snapshot.
7. Steady state: telemetry → ingest → log + AI; health reports at severity rate;
   monitor tracks state; CC_SAFETY_STATUS echoes/acks.

**Critical health event:**
1. cc-ai-health emits CRITICAL finding → cc-health-tx sends immediately, repeats
   5 Hz.
2. mavlink_receiver validates → `cc_health_report` uORB.
3. cc_safety_monitor: state → CRITICAL (edge) → policy table → e.g. issue Land
   `vehicle_command` per `CC_MON_CRIT_ACT` → publish CC_SAFETY_STATUS
   (action_taken + last_report_sequence ack) → PX4 event/warning to GCS.
4. Jetson receives ack → repeat rate falls to steady CRITICAL rate → logs
   finding, report, ack, and action in mission dataset.
5. Recovery: `CC_MON_OK_COUNT` consecutive OK reports → monitor OK →
   no automatic mode reversal (pilot/GCS decides); status echoed.

**Link loss mid-flight:** see §11 rows; key sequencing: PX4 side is driven purely
by report age (no reliance on TCP-style connection state); Jetson side keeps
logging locally, marks FC streams stale, and on link restoration re-locks
timesync before trusting FC timestamps again.

---

## 11. Failure Behavior Matrix

| Failure | Detection | PX4 behavior | Jetson behavior |
|---|---|---|---|
| Jetson RX overloaded | bounded-queue overflow | — | drop P2/P3 telemetry first, keep P0 alive, count drops into events |
| Jetson AI overloaded | evaluate deadline blown | — | keep raw logging, degrade confidence, WARN if prolonged |
| Link lost / Jetson dead | report age > CC_MON_TIMEOUT_MS | state STALE → block Offboard; `CC_MON_STALE_ACT` if in Offboard; manual flight unaffected | on its side: FC heartbeat age → link DOWN, keep local logging, backoff reconnect |
| companiond crash | systemd watchdog | (appears as link lost) | restart, new cc_boot_id, new segment |
| FC telemetry stream missing | ingest stream watchdog | — | mark stream STALE, dependent algorithms UNAVAILABLE, never fake values |
| Timesync poor | quality DEGRADED/UNLOCKED | — | log raw timestamps, timing-sensitive algorithms reduce confidence |
| Flooded/garbage inbound at PX4 | rate/validation gauntlet | drop + count + throttled warning; monitor state unaffected by invalid frames | n/a |
| FC reboot mid-mission | px4_boot_id change / timestamp regression | normal PX4 failsafe stack (out of scope here) | invalidate timesync, new segment, mark discontinuity |
| Jetson disk low | free-space monitor | — | shed raw capture → shed Class B/F rows → WARN health flag; never silent total stop |
| Schema mismatch | handshake / per-message check | reject reports (BAD_SCHEMA), throttled warning, state stays UNKNOWN/STALE | refuse mission start, log configuration error |

---

## 12. PX4 Parameters

| Param | Meaning | Default |
|---|---|---|
| `CC_MON_EN` | enable companion monitor | 1 |
| `CC_MON_REQ_OFFB` | require companion OK before Offboard | 1 |
| `CC_MON_TIMEOUT_MS` | stale timeout on health reports | 3000 |
| `CC_MON_OK_COUNT` | consecutive OK reports to recover OK | 3 |
| `CC_MON_CRIT_ACT` | action on CRITICAL (0 warn / 1 hold / 2 land / 3 RTL) | 1 |
| `CC_MON_STALE_ACT` | action on STALE while in Offboard | exit-Offboard→Hold |
| `CC_TEL_PROFILE` | MINIMAL / AI_UART / AI_ETH / DEBUG | AI_UART |
| `CC_TEL_IMU_RATE` | IMU summary stream Hz | 50 |
| `CC_TEL_ACT_RATE` | actuator/ESC stream Hz | 20 |

Safety behavior is parameterized, never hardcoded; all are runtime-updatable.

---

## 13. Runtime Envelope for "AI" (hard boundary)

To PX4 travel only: severity, recommended action, confidence, flags,
detail/evidence codes, and compact self-telemetry — fixed-size, ≤ one MAVLink 2
frame, ≤ 5 Hz sustained.
Never to PX4: tensors, embeddings, JSON, images, variable-length text
diagnostics, per-model internals, or any request that PX4 evaluate/classify.
Rich explanations live in `ai_health.parquet` / `CC_AI_DIAGNOSTIC` (log-only)
on the Jetson side, keyed by the same detail_code the compact report carried —
so an incident report can join "what PX4 was told" with "why the AI said it."

---

## 14. Testing & Validation Plan

1. **Dialect round-trip tests:** encode/decode every CC_* message in both the
   C and Rust bindings from the same XML; golden byte vectors checked into both
   repos; CI fails on divergence (catches CRC_EXTRA drift).
2. **Rust unit/property tests:** cc-ingest fed fuzzed frames (truncated, bad
   CRC, wrong source, wrong schema, sequence storms) — must never panic,
   counters must match injected faults exactly.
3. **PX4 SITL integration:** run the full stack against PX4 SITL over UDP with
   the custom dialect; scripted scenarios: nominal mission, CRITICAL report →
   verify Land command + ack; report flood → verify rate-limit counters;
   stale → verify Offboard block.
4. **Replay regression:** every recorded bench/flight mission becomes a fixture;
   cc-replay diffs algorithm versions; false-positive audit before any policy
   is allowed to auto-act (fly with `CC_MON_CRIT_ACT = warn-only` first).
5. **Bench HITL:** real V6X + Jetson over real UART; 30-min full-rate soak,
   zero CRC errors; measure PX4 CPU delta with streams on vs off; cable-pull
   and Jetson power-pull tests → verify STALE behavior and clean recovery.
6. **Fault injection in flight test plan (staged):** Phase 1 monitor
   passive/warn-only; Phase 2 monitor gates Offboard only; Phase 3 monitor may
   command Hold/Land — each phase gated on the previous phase's evidence.
7. **Pre-flight checklist additions:** companion state = OK, timesync LOCKED,
   disk floor OK, mission manifest created, correct `CC_TEL_PROFILE`.

---

## 15. Defensible Summary

PX4 remains the safety-critical real-time controller and keeps a minimal local
ULog blackbox. The Jetson is the primary mission-data and AI-health computer.
PX4 streams selected, timestamped, sequence-numbered telemetry at controlled
rates over a MAVLink 2 custom dialect; the Jetson stores the mission dataset,
runs health algorithms, and returns compact, validated health reports. A single
deterministic PX4 module (`cc_safety_monitor`) converts those reports into
conservative, parameterized actions and acknowledges them. Duplicate logging is
avoided by ownership, not by omission: PX4 logs safety/debug evidence, the
Jetson logs the high-rate AI dataset, and shared identity keys
(vehicle, mission, boot, stream, sequence) keep the two records joinable.
