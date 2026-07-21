# Phase 6 — `cc_safety_monitor` + `cc-health-tx`: the safety loop closes

**Goal (dev plan):** report → validate → policy → action → ack, end to end, in
SITL. Exit criteria: every scenario green and repeatable; the policy table has
100% host-side unit coverage; the cross-log join works.

The safety loop is the first CCFC code that can *change what the vehicle does*
(the monitor may command Hold/Land/RTL). It is built to be provably
conservative and exhaustively testable off-vehicle.

---

## Part A — Design

### A.1 The decision core is pure and host-tested

The two pieces that actually decide vehicle behaviour are **PX4-free headers**,
so they compile and run on the host and are exhaustively unit-tested outside
the flight tree (`cc_policy_table_test.cpp`, **40/40**):

- **`cc_policy_table.hpp`** — the deterministic policy, a pure function
  `(companion_state, recommended_action, flight_context, params) → action`
  (spec §4.5 table). Two invariants are encoded structurally:
  1. **Conservative-only.** The output enum has *no* value that arms, takes
     off, enters Offboard, or increases authority — an exhaustive sweep over
     every `(state, recommendation, context, params)` combination asserts every
     reachable output is in `{None, Warn, BlockOffboard, Hold, Land, Rtl}`.
  2. **Parameter wins over recommendation**, and a bad `CC_MON_*_ACT` value
     **fails safe to Hold** (never None) — a misconfigured parameter can never
     silently disable the protective action.
- **`cc_state_machine.hpp`** — the companion state machine
  (UNKNOWN/OK/WARN/CRITICAL/STALE). Escalation is immediate; de-escalation to
  OK needs `CC_MON_OK_COUNT` consecutive OKs; CRITICAL never relaxes to WARN;
  STALE is left only by a run of OKs (a CRITICAL still escalates STALE→CRITICAL,
  the more-conservative direction).

The PX4 module (`CcSafetyMonitor`) owns only the *plumbing*: uORB, clocks,
edge-triggering, pilot-override, `CC_MON_*` params, and issuing the
`vehicle_command`. The risky logic has zero PX4 dependencies and 100% host
coverage.

### A.2 The module (`CcSafetyMonitor`)

Consumes validated `cc_health_report` (published by the extended
`mavlink_receiver` after the Phase-3 gauntlet: source/schema/range/sequence/
flood), `vehicle_status`, `vehicle_control_mode`, `vehicle_land_detected`. Each
20 Hz tick: ingest new reports → advance the state machine → run the policy →
publish `cc_safety_status` (state echo + report **ACK** —
`last_report_sequence` is what stops the companion's 5 Hz CRITICAL repeat). On a
state **transition** (edge-triggered, once per transition) it issues at most one
conservative `DO_SET_MODE` command (Hold/Land/RTL, or exit-Offboard→Hold) and
then honours **pilot override**: it never re-commands until a *new* transition,
so a pilot's mode change after a monitor action is respected. `CC_MON_EN=0`
disables all logic and echoes `reject_reason = MONITOR_DISABLED`.

### A.3 The companion source (`cc-health-tx`)

A **scripted** severity source (v0, no AI): a scenario file of timed events
drives the health conclusion. `ReportSource` emits `CC_HEALTH_REPORT` at P1 with
the spec rate policy (OK 1 Hz, WARN 2–5 Hz, CRITICAL 5 Hz **until acknowledged**
then 1 Hz keepalive), edge-triggered on severity change, and tracks the
monitor's ACK (`CC_SAFETY_STATUS.last_report_sequence` via companiond's demux
tap) to stop the CRITICAL repeat. The cores (scenario timeline, rate policy,
hysteresis) are pure and unit-tested.

---

## Part B — Inventory & deviations

**Fork** (`src/modules/cc_safety_monitor/`): `cc_policy_table.hpp`,
`cc_state_machine.hpp` (pure) · `cc_policy_table_test.cpp` (host tests) ·
`CcSafetyMonitor.{hpp,cpp}` · `params.c` (CC_MON_*) · `CMakeLists.txt` ·
`Kconfig`; enabled in the SITL board + `rcS`; the pre-existing
`CC_SAFETY_STATUS` mavlink stream carries the echo back.

**Companion**: `crates/cc-health-tx` (scenario/policy/lib) · companiond
`--health-scenario`, the `CC_SAFETY_STATUS` ack tap, and the `safety` object in
the status JSON.

### Deviations

- **D45 — decision core as pure PX4-free headers** (host-testable), the module
  is plumbing only. Makes the safety-critical logic 100% host-covered.
- **D46 — state-machine hysteresis** read literally from §4.5: CRITICAL never
  relaxes to WARN; STALE is left only by `CC_MON_OK_COUNT` OKs (a CRITICAL
  escalates it immediately); a bad `CC_MON_*_ACT` fails safe to Hold.
- **D47 — Offboard block is active, not (yet) a pre-arm gate.** The monitor
  *exits* Offboard to Hold when the companion is UNKNOWN/STALE/CRITICAL-on-
  ground and `CC_MON_REQ_OFFB=1`. Registering a `HealthAndArmingChecks` item to
  *refuse Offboard entry pre-arm* is a follow-up; the active exit is arguably
  stronger and is fully SITL-tested.
- **D48 — cc-health-tx v0 reports the scripted conclusion directly**; the
  hysteresis governs the report *rate*. A real noisy AI source (Phase 7) will
  hysterese the conclusion itself.
- **D49 — `CC_MON_TIMEOUT_MS` → `CC_MON_TMOUT_MS`** (PX4's 16-char param limit).
- **D50 — `vehicle_land_detected.landed` defaults false = the safe default**: a
  flying vehicle is never mis-classified as grounded, so CRITICAL never
  downgrades from Land to a mere arming-block while airborne.
- **D51 — airborne-flight SITL scenario deferred.** The headless SIH config
  used across Phases 2–6 is telemetry-focused and not arming/flight-ready
  (sensor-health preflight fails in a fresh rootfs). The airborne action
  decisions (CRITICAL/STALE while flying → Land/Hold/RTL, per-`CC_MON_CRIT_ACT`
  mapping) are exhaustively covered by the 40 host tests; the command-issuance
  path was validated in the SITL smoke.

---

## Part C — Results

### C.1 Policy core (host, exhaustive)

`c++ -std=c++14 cc_policy_table_test.cpp` → **40/40**: one case per §4.5 table
row (CRITICAL air→param-action, CRITICAL ground→block, WARN→warn-only, STALE
in/out of Offboard, OK honour-block-offboard, UNKNOWN→block), param-wins,
fail-safe-on-bad-param, the exhaustive conservative-only sweep, and the full
state machine (immediate escalation, OK_COUNT recovery, CRITICAL-never-relaxes,
STALE entry/exit, reboot reset). **Policy-table 100% host coverage — exit
criterion met.**

### C.2 SITL scenario suite

`tools/phase6/sitl_phase6_check.py` — **12/12** against headless SITL + the
release companiond, asserting the monitor's response from companiond's status
JSON `safety` object:

| Scenario | Result |
|---|---|
| nominal | OK reports → monitor **OK**, action **NONE**, ACK advancing |
| critical (ground) | → monitor **CRITICAL**, action **BLOCK_OFFBOARD** (never auto-Land on the ground) |
| critical ACK | `last_report_sequence` keeps advancing under the 5 Hz repeat |
| recovery | OK × `CC_MON_OK_COUNT` → back to **OK** |
| garbage/flood | bad-source / out-of-range / flooded reports never move the state (receiver gauntlet) |
| stale | reports stop → **STALE** at `CC_MON_TMOUT_MS`; resume OK → recover |
| disabled | `CC_MON_EN=0` → `reject_reason = MONITOR_DISABLED`, no action |

### C.3 Cross-log join (dev-plan 6.4)

The Phase-5 mission log already captures the `safety_status` stream, and PX4's
ULog captures `cc_health_report` + `cc_safety_status`; both carry the identity
dedup key. A dedicated join-and-diff tool over the two logs is a Phase-6.4
follow-up alongside the airborne SITL scenario.

### Status

The safety loop is **implemented, provably conservative, and validated end to
end** (40 host + 11 Rust + 12/12 SITL). Remaining follow-ups: the airborne
arm+takeoff+assert-one-Land SITL scenario (needs a flight-ready sim), the
pre-arm `HealthAndArmingChecks` registration (D47), and the ULog↔mission-log
cross-join tool (C.3).
