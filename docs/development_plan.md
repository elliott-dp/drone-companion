# FC ↔ CC Development Plan

**Companion spec:** `fc_cc_comm_architecture.md` · **Protocol:** `cc_dialect.xml`
**Principle:** every phase ends with something *demonstrable and tested*; nothing
flight-facing is written before its SITL equivalent works. Phases 0–5 need no
drone hardware at all.

---

## Phase 0 — Repositories, toolchains, CI skeleton

**Goal:** both codebases build empty, share one dialect, and CI guards the contract.

1. **Repo layout.** Two repos (or one mono-repo with two roots):
   - `px4-firmware` — fork of PX4-Autopilot pinned to a release tag (pick the
     current stable, never `main`, and record the tag in the spec).
   - `drone-companion` — the Rust workspace from spec §5.1.
   - `cc-dialect` — tiny repo (or shared subdirectory) containing only
     `cc_dialect.xml` + generation scripts + golden test vectors. Both other
     repos consume it as a git submodule pinned by commit hash.
2. **Toolchains.**
   - PX4: install the PX4 dev environment (ubuntu.sh script), verify
     `make px4_sitl_default` and `make cuav_v6x_default` both build the
     *unmodified* fork before touching anything.
   - Jetson: rustup stable on your dev machine; add `aarch64-unknown-linux-gnu`
     cross target (or plan to build natively on the Orin — natively is simpler
     and the Orin is fast enough). `cargo new` the workspace + empty crates.
3. **Dialect generation scripts** (in `cc-dialect/`):
   - `gen_c.sh` → mavgen C, output vendored into the PX4 tree the way PX4
     expects custom dialects (PX4 docs: "MAVLink Messaging — custom messages").
   - `gen_rust.sh` → configure the `mavlink` crate's build-time XML input in
     `cc-protocol/build.rs` pointing at the submodule path.
   - `hash.sh` → stable hash of the XML → generated `dialect_hash` constant
     emitted into a C header and a Rust source file.
4. **CI (GitHub Actions or similar).**
   - Job 1: regenerate both bindings from the submodule commit; fail if the
     vendored C headers differ from freshly generated (drift guard).
   - Job 2: `cargo build --workspace && cargo test --workspace`.
   - Job 3: `make px4_sitl_default` with modules enabled.
   - Job 4 (added in Phase 1): golden-vector round-trip test.

**Exit criteria:** clean CI on an empty-but-wired-up codebase; V6X target builds.

---

## Phase 1 — Protocol layer proven on the bench (no PX4 code yet)

**Goal:** the dialect encodes/decodes identically in C and Rust.

1. Vendor generated C headers; wire `cc-protocol` build.rs; both compile.
2. **Golden vectors:** a small C program (built with the generated headers)
   encodes one instance of *every* CC_* message with fixed field values and
   dumps the raw MAVLink 2 frames to `golden_frames.bin`; commit it to
   `cc-dialect/`.
3. Rust test in `cc-protocol`: parse `golden_frames.bin`, assert every field
   equals the fixed values; then re-encode and assert byte-identical output.
   This is the CRC_EXTRA drift detector — if it passes, C and Rust agree on
   the wire format forever after (until the XML changes, which regenerates
   both and the vectors).
4. Fuzz/property tests in `cc-protocol`/`cc-ingest` foundations: truncated
   frames, corrupted CRC, random bytes, giant sequences — parser must never
   panic and counters must match injected faults.

**Exit criteria:** golden round-trip green in CI; fuzz suite green.

---

## Phase 2 — PX4 uORB topics + `cc_telemetry_publisher` (SITL)

**Goal:** curated telemetry exists inside PX4, correctly rate-controlled.

1. Add the eight `.msg` files (spec §4.1, with the trimmed
   `CcTelemetryActuator`: timestamp, sequence, motor_count,
   actuator_output[8]); register in `msg/CMakeLists.txt`; build.
2. Skeleton `cc_telemetry_publisher` module (ScheduledWorkItem, work queue
   `lp_default`, Kconfig entry, start from rcS): first iteration publishes only
   `cc_telemetry_state` at 25 Hz from `vehicle_status` + `vehicle_attitude` +
   `vehicle_local_position`.
3. Verify in SITL: `make px4_sitl gz_x500` (or jmavsim), then
   `listener cc_telemetry_state` — check rate, sequence monotonicity,
   plausible values while flying a scripted SITL mission.
4. Add the remaining five output topics one at a time, each verified with
   `listener` before the next. Implement `CC_TEL_PROFILE` /
   `CC_TEL_IMU_RATE` / `CC_TEL_ACT_RATE` parameters and confirm rates follow
   them live (`param set` mid-run).
5. Implement and verify the module's `print_status()` (per-stream counts,
   rates, px4_boot_id).
6. Measure: `top` in the NuttX shell on a real V6X later, but in SITL check
   the work-queue timing (`work_queue status`) shows negligible load.

**Exit criteria:** all six topics publish at profile rates in SITL; params
change rates live; zero dynamic allocation after init (review + heap checks).

---

## Phase 3 — MAVLink streams out + receiver in (SITL over UDP)

**Goal:** custom messages actually cross a link, both directions.

1. Add the eight `MavlinkStreamCcTelemetry*`/`CC_EVENT`/`CC_SAFETY_STATUS`
   stream classes; register in `mavlink_messages.cpp`; configure the SITL
   MAVLink instance to enable them (`mavlink stream -u <port> -s ... -r ...`).
2. Quick validation harness before any Rust: a 30-line pymavlink script with
   the custom dialect that connects to SITL UDP and prints decoded CC_*
   messages. (Python here is scaffolding only, but it isolates "PX4 sends
   wrong" from "Rust decodes wrong" for the rest of the project.)
3. Extend `mavlink_receiver.cpp`: handle `CC_HEALTH_REPORT`,
   `CC_MISSION_CONTEXT` (+ log-only `CC_AI_DIAGNOSTIC`), implement the full
   validation gauntlet (spec §4.4: source=191, schema, ranges, sequence,
   flood limit) with named drop counters, publish `cc_health_report` uORB.
4. Test the gauntlet from the pymavlink harness: send valid reports (appear in
   `listener cc_health_report`), then each invalid class (wrong compid, bad
   schema, out-of-range severity, duplicate sequence, 100 Hz flood) and assert
   the right counter increments and *nothing* publishes.

**Exit criteria:** scripted SITL session shows all FC→CC streams decoded
externally, and CC→FC reports pass/fail the gauntlet exactly as specified.

---

## Phase 4 — Rust `cc-link` + `cc-timesync` + `cc-ingest` against SITL

**Goal:** the real companion RX path replaces the Python harness.

1. `cc-link`: UDP transport first (matches SITL), serial second (same trait);
   incremental parser, resync-on-0xFD, per-stream counters, priority TX queues
   (P0–P3 per spec §5.3), HEARTBEAT task at 1 Hz.
2. `cc-timesync`: TIMESYNC client (10 Hz fast-lock → 1 Hz), RTT-filtered
   median offset, quality states, conversion functions; unit-test the filter
   against synthetic jitter traces.
3. `cc-ingest`: decode → validate → sequence-continuity → age → the
   `TelemetryEvent` enum → tokio broadcast; per-stream staleness watchdogs.
4. `companiond` v0: wires link+timesync+ingest, prints a 1 Hz status line
   (link state, per-stream Hz, gaps, RTT, offset, quality).
5. Integration test (scripted, in CI if runners allow SITL, else a make
   target): start SITL, start companiond, fly a scripted mission, assert:
   every stream at expected rate ±20%, zero unexplained sequence gaps,
   timesync LOCKED within 5 s.
6. Fault drills against SITL: kill/restart SITL (FC "reboot": px4_boot_id
   change → timesync invalidated, streams re-lock), pause telemetry (stream
   watchdogs fire), garbage injection on the socket (resync works).

**Exit criteria:** companiond runs unattended against SITL for 1 h with clean
counters; all fault drills behave per spec §11.

---

## Phase 5 — `cc-mission-log` + `cc-config` + supervision

**Goal:** the mission dataset exists, crash-safe, replayable.

1. `cc-config`: layered config (file + env + CLI): transport, device path,
   baud, vehicle_id, rates expected, disk floor, mission root.
2. `cc-mission-log`: mission/segment directories, manifest (temp+fsync+rename),
   Parquet writers per stream (arrow-rs), rotation caps, raw_mavlink.bin
   capture, drop accounting, disk-floor + shedding ladder.
3. Handshake: `CC_MISSION_CONTEXT` at 1 Hz until acked by
   `CC_SAFETY_STATUS` leaving UNKNOWN (Phase 6 provides the real ack; until
   then, stub-accept on first heartbeat), PARAM snapshot →
   `px4_params_snapshot.json`.
4. `log-inspect` CLI: open a mission dir → per-stream row counts, time ranges,
   gap totals, drop totals, manifest validity, dialect hash match.
5. Crash tests: `kill -9` companiond mid-mission → restart → new segment,
   previous segment's flushed row groups readable, manifest links segments;
   fill the disk in a sandbox → shedding ladder + WARN flag, never a silent
   total stop.

**Exit criteria:** a 1 h SITL mission produces a complete, `log-inspect`-clean
dataset; kill/disk-full tests pass.

---

## Phase 6 — `cc_safety_monitor` + `cc-health-tx`: the safety loop closes

**Goal:** report → validate → policy → action → ack, end to end, in SITL.

1. PX4 `cc_safety_monitor` module: state machine (UNKNOWN/OK/WARN/CRITICAL/
   STALE), the pure policy table in `cc_policy_table.hpp` (unit-testable in
   isolation — write host-side unit tests for every table row), edge-triggered
   actions, `CC_MON_*` parameters, `cc_safety_status` publication, arming-check
   contribution for `CC_MON_REQ_OFFB`.
2. `cc-health-tx` v0: no AI yet — a *scripted* severity source (config-driven
   scenario file: "at t=120 s emit CRITICAL/LAND, confidence 90") merged into
   reports with hysteresis, rate policy (1/2–5/5 Hz), ack-tracking against
   `last_report_sequence`.
3. SITL scenario suite (this is the core deliverable of the phase):
   - nominal: OK reports → monitor OK → Offboard permitted.
   - critical-in-air: scripted CRITICAL/LAND while SITL hovers →
     assert exactly one Land `vehicle_command`, `action_taken=LAND` in
     CC_SAFETY_STATUS, Jetson ack-detected, repeat rate drops.
   - stale: stop reports → monitor STALE at `CC_MON_TIMEOUT_MS`, Offboard
     entry refused; resume OK ×`CC_MON_OK_COUNT` → recovery.
   - flood/garbage: assert monitor state never moves on invalid input.
   - pilot override: RC mode change after monitor action → monitor does not
     re-command until a new transition.
   - param sweep: each `CC_MON_CRIT_ACT` value produces its action.
4. Log evidence check: ULog (SITL) contains `cc_health_report` +
   `cc_safety_status`; Jetson mission log contains the same events — join on
   the dedup key and diff timestamps (this validates the whole identity
   scheme).

**Exit criteria:** every scenario green and repeatable; policy table has 100%
host-side unit coverage; the cross-log join works.

---

## Phase 7 — `cc-ai-health` + `cc-replay`

**Goal:** real algorithms behind the (already-proven) safety loop.

1. Implement the `HealthAlgorithm` trait + runner (10 Hz evaluate, rolling
   windows, availability tracking, deadline enforcement).
2. Algorithms in order of evidence value for *this* vehicle (no ESC telemetry,
   so motor insight comes from correlation, not per-motor feedback):
   1. `battery_model` — voltage-under-load vs current model, sag anomaly,
      consumed-vs-remaining consistency.
   2. `vibration_anomaly` — per-axis vibration metric baselining + clipping.
   3. `estimator_consistency` — test-ratio trends, innovation flag streaks.
   4. `gps_quality` — eph/epv/sat/noise/jamming composite.
   5. `motor_balance` — **correlation-based**: commanded output asymmetry vs
      attitude-rate residuals vs total power vs vibration signature
      (a failing motor shows as persistent output offset + power/vibration
      shift; document the reduced observability honestly).
   6. `link_quality`, `thermal_monitor` (battery + IMU temps), `mission_risk`.
   Each ships with unit tests on synthetic traces before touching live data.
3. `cc-replay`: mission dir → timed `TelemetryEvent` stream → algorithm
   harness → `HealthFinding` diff between versions; determinism test (same
   input ⇒ byte-identical findings). Every recorded SITL/bench mission
   becomes a regression fixture.
4. Threshold tuning loop: record benign SITL + bench missions → replay →
   drive false-positive rate to ~zero on benign data *before* the monitor is
   ever allowed to auto-act on these algorithms.

**Exit criteria:** algorithms green on synthetic + replay fixtures; replay
deterministic; false-positive audit documented.

---

## Phase 8 — Bench HITL: real V6X ↔ real Jetson over TELEM3

**Goal:** the SITL-proven system runs on the actual wire.

1. **Jetson serial prep:** disable nvgetty / `serial-getty@ttyTHS0`, confirm
   `/dev/ttyTHS0`, add service user to `dialout`, systemd unit for companiond
   (`Restart=on-failure`, `WatchdogSec=10`).
2. **Wiring:** TELEM3 TX→pin 10, RX→pin 8, GND→pin 6; verify V6X TELEM3
   NuttX device node in the board config; loopback-test each side separately
   before connecting them.
3. **Bring-up ladder:** 57600 heartbeats-only → 921600 heartbeats →
   full AI_UART profile. At each step: zero CRC errors or stop and fix wiring.
4. **Soak:** 30 min full-rate, motors/props OFF then ON at low throttle on the
   bench (EMI check): CRC error rate must stay ~0; record PX4 CPU with streams
   on vs off (expect ≲1–2% delta); record Jetson CPU/disk rates.
5. **Fault drills on hardware:** pull the UART mid-stream → STALE behavior +
   clean resync on reconnect; power-cycle the Jetson → PX4 unaffected, new
   segment on return; reboot PX4 → companion re-locks timesync, new segment.
6. First real ULog + mission-dataset pair; run the cross-log join from
   Phase 6.4 on real data.

**Exit criteria:** soak clean, all drills per spec §11, CPU/bandwidth measured
and written into spec §8 as *measured* numbers.

---

## Phase 9 — Staged flight testing

Gate each stage on the previous stage's logs; change one variable at a time.

- **Stage 1 — passive:** `CC_MON_CRIT_ACT = warn-only`, `CC_MON_REQ_OFFB = 0`.
  Manual flights. Objective: dataset collection + false-positive audit in the
  real vibration/EMI environment. Several flights, replay everything.
- **Stage 2 — gating:** `CC_MON_REQ_OFFB = 1`; monitor may block Offboard but
  never commands modes. Verify STALE blocks Offboard entry in the air-adjacent
  ground test first.
- **Stage 3 — acting:** enable `CC_MON_CRIT_ACT = Hold` (not Land) first;
  provoke a controlled CRITICAL (e.g., scripted health-tx injection, not a
  real fault) at safe altitude with the pilot ready to override. Only after
  that: real-algorithm-driven actions, then Land/RTL policies if desired.
- Every flight: pre-flight checklist additions (companion OK, timesync
  LOCKED, disk floor, manifest created, correct profile); post-flight:
  archive ULog + mission dir together, replay, diff against expectations.

---

## Cross-phase working rules

- **Contract changes** (any XML edit): bump schema_version if semantics
  change, regenerate both bindings + golden vectors in one commit, never
  reuse/renumber IDs, extensions-only field additions.
- **Definition of done** per module: unit tests, a fault-path test, a
  `status`/inspect surface, and a spec section that still matches the code.
- **Never skip the ladder:** SITL → bench → passive flight → gating → acting.
  Any anomaly at a stage sends that feature back one stage.
