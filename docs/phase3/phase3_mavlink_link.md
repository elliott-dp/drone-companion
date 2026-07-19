# Phase 3 — MAVLink streams out + receiver in (SITL over UDP)

**Goal (dev plan):** *"custom messages actually cross a link, both
directions."* The Phase 2 uORB topics leave the FC as real `CC_*` MAVLink
frames, and companion-originated messages enter the FC through the full
validation gauntlet (spec §4.4) — proven end-to-end against SITL over UDP,
still with no Rust in the loop (that is Phase 4; Python is scaffolding, per
the dev plan).

---

## Part A — Plan

### A.1 Scope (dev plan Phase 3, items 1–4)

1. Eight MAVLink stream classes (`CC_TELEMETRY_*` ×6, `CC_EVENT`,
   `CC_SAFETY_STATUS`) registered in `mavlink_messages.cpp`, enabled on a
   dedicated SITL companion-link instance at profile rates.
2. A pymavlink harness with the custom dialect (bindings generated from the
   same pinned toolchain and XML as everything else — **already proven
   against the Phase 1 golden vectors**: 16/16 frames decode, dialect_hash
   `0xdc5b8e9f` matches, so C ↔ Rust ↔ Python agree on the wire before PX4
   enters the picture).
3. `mavlink_receiver.cpp` extension: `CC_HEALTH_REPORT`,
   `CC_MISSION_CONTEXT`, log-only `CC_AI_DIAGNOSTIC`, with the complete
   spec §4.4 gauntlet (source → schema → range → sequence → flood) and
   named drop counters.
4. Scripted gauntlet fault-injection tests: every invalid class increments
   exactly its counter and publishes nothing.

Out of scope, deliberately: `CC_LOG_CONTROL` handling (spec marks it
OPTIONAL/dev-only, param-gated; deferred until a phase needs it), and any
`cc_safety_monitor` behavior (Phase 6 — its stream class exists but stays
silent until the monitor publishes `cc_safety_status`).

### A.2 Dialect switch — how PX4 builds `cc_dialect`

Investigated mechanism (v1.17.0): the mavlink module generates its C
headers **at firmware build time** via the mavlink submodule's own
mavgen, from `src/modules/mavlink/mavlink/message_definitions/v1.0/
${CONFIG_MAVLINK_DIALECT}.xml`. SITL currently builds `"development"`.

Plan:

* `boards/px4/sitl/default.px4board`: `CONFIG_MAVLINK_DIALECT="cc_dialect"`.
* The XML cannot be committed *inside* the mavlink submodule (that would
  require forking the mavlink repo too). Instead the fork vendors it at
  **`ccfc_dialect/`** (fork root): `cc_dialect.xml` (verbatim copy of the
  drone-companion original), `PROVENANCE.md` (source + SHA-256), and
  `install.sh` which copies it into the submodule's
  `message_definitions/v1.0/` — the same operation `gen_c.sh --px4`
  performs from the drone-companion side. `tools/phase2/build_px4.sh` runs
  the installer automatically, so a fresh clone builds in two commands.
* **Drift guards** (the dialect now exists in two repos):
  1. the Phase 3 harness asserts sha256(fork copy) == sha256(companion
     copy) before any SITL test;
  2. the wire itself: the harness decodes with bindings generated from the
     *companion* copy — if the fork copy diverged, CRC_EXTRA kills every
     CC_* frame and the rate checks fail loudly.
* Include-chain nuance, on the record: at PX4 build time,
  `<include>common.xml</include>` resolves against the **submodule's**
  common.xml (mavlink repo pin of PX4 v1.17.0), not our pymavlink-2.4.49
  pins. CRC_EXTRA of CC_* messages depends only on our definitions; for
  the standard messages the harness empirically proves compatibility
  (HEARTBEAT/TIMESYNC decode). This is exactly the risk the phase 1 doc
  flagged; Phase 3 is where it gets measured instead of assumed.

### A.3 Stream classes (FC → CC)

Pattern copied from upstream (`streams/ATTITUDE.hpp` as template;
dialect-specific streams gated like upstream's FIGURE_EIGHT — include and
registration both wrapped in `#if defined(MAVLINK_MSG_ID_<NAME>)`, so
common-dialect boards still compile).

| Stream | Source uORB | Mapping notes |
|---|---|---|
| `CC_TELEMETRY_STATE` | `cc_telemetry_state` | 1:1; `timestamp` → `fc_timestamp_us` |
| `CC_TELEMETRY_IMU` | `cc_telemetry_imu` | 1:1 |
| `CC_TELEMETRY_POWER` | `cc_telemetry_power` | 1:1 |
| `CC_TELEMETRY_GPS` | `cc_telemetry_gps` | 1:1 |
| `CC_TELEMETRY_ESTIMATOR` | `cc_telemetry_estimator` | 1:1 |
| `CC_TELEMETRY_ACTUATOR` | `cc_telemetry_actuator` | 1:1 |
| `CC_EVENT` | `event` (PX4 events interface) | see D12 below |
| `CC_SAFETY_STATUS` | `cc_safety_status` | 1:1; publisher arrives in Phase 6 — `get_size()==0` until then (event-driven per spec §4.3) |

All telemetry streams use `update()` semantics (send only on fresh uORB
sample) so the MAVLink rate can never exceed the uORB rate — the publisher
decimator (Phase 2) remains the single rate authority; `mavlink stream -r`
is a ceiling, exactly as spec §4.3 prescribes ("publisher decimation
defines the ceiling; mavlink stream rate ≤ that ceiling").

**D12 — CC_EVENT mapping** (spec Class G, no dedicated uORB topic exists by
design): source is PX4's `event` topic. `event_id ← id`,
`argument0/1 ← first 8 argument bytes (LE)`, `sequence ← event_sequence`
(the events interface's own monotonic counter), severity mapped from the
event's internal log level (emerg/alert/crit/err → CRITICAL, warn/notice →
WARN, info/debug → OK), `subsystem = CC_SUBSYS_NONE` (PX4 events don't
carry our subsystem taxonomy; refining this is Phase 6+ work).

### A.4 Receiver gauntlet (CC → FC)

`mavlink_receiver.{h,cpp}` gains three handlers (gated on
`MAVLINK_MSG_ID_CC_HEALTH_REPORT`), executing spec §4.4's checks **in
order**, each failure incrementing a named counter and dropping the
message before any uORB publish:

| # | Check | Counter | Notes |
|---|---|---|---|
| 1 | CRC | (implicit — parser drops; visible in mavlink RX stats) | |
| 2 | `sysid == own`, `compid == 191` | `ccfc_rx_bad_source` | spec §3.3 |
| 3 | `schema_version == 1` | `ccfc_rx_bad_schema` | one throttled warning per boot (`events::send` equivalent: `PX4_WARN` throttled) |
| 4 | ranges: severity ≤ 3, recommended_action ≤ 5, confidence ≤ 100, subsystem ≤ 11 | `ccfc_rx_bad_range` | |
| 5 | sequence strictly newer (mod 2³², signed-diff window); duplicates/regressions dropped; gaps accumulate `ccfc_rx_missed_reports` | `ccfc_rx_dup_seq` | first message after boot always accepted |
| 6 | flood: > 20 `CC_HEALTH_REPORT`/s (rolling 1 s window) | `ccfc_rx_flood_dropped` | spec: a buggy companion must not spam the work queue |

Accepted messages publish `cc_health_report` / `cc_mission_context` /
`cc_ai_diagnostic` uORB (with `timestamp` = receive time) and increment
`ccfc_rx_accepted_*`. Counters are surfaced in **`mavlink status`**
(spec §4.4: "surfaced by print_status()") via a small CCFC block in
`Mavlink::display_status()` — the harness parses that text.

`CC_MISSION_CONTEXT` additionally validates (spec §7): `vehicle_id` ==
param **`CC_VEHICLE_ID`** (new param, D11 — spec §7 requires "same value in
PX4 param and Jetson config" but §12 never named the param) →
`ccfc_rx_bad_source` on mismatch; `dialect_hash` == FC-side build constant
(from `ccfc_dialect` install, compiled in as `CC_DIALECT_HASH`… Phase 3
implementation: computed at generation time is not available in C — the
vendored `dialect_hash.h` from cc-dialect provides it; the fork carries a
copy in `ccfc_dialect/`) → `ccfc_rx_bad_schema` on mismatch.

**D9/D10 — two new uORB messages** (spec §4.1 lists eight `.msg` files but
§4.4 requires receiver→publisher/logger handoff topics that aren't in the
list — resolved in PX4's idiom, documented as spec deviations):
`CcMissionContext.msg` (mission_id, cc_boot_id, vehicle_id, dialect_hash,
sw_version[24], schema_version) and `CcAiDiagnostic.msg` (mirror of the
wire message). `cc_telemetry_publisher` subscribes `cc_mission_context`
and echoes `mission_id` into `CC_TELEMETRY_STATE` (closing the spec §4.4
"echo into context" loop — the harness asserts the echo end-to-end).

### A.5 SITL link topology

`ROMFS/px4fmu_common/init.d-posix/px4-rc.mavlink` gains a CCFC block
(following the file's existing per-instance port scheme):

```
udp_ccfc_port_local=$((24540+px4_instance))   # PX4 binds here
udp_ccfc_port_remote=$((24040+px4_instance))  # harness listens here (udpin)
mavlink start -x -u $udp_ccfc_port_local -r 400000 -m onboard -o $udp_ccfc_port_remote
mavlink stream -r 25 -s CC_TELEMETRY_STATE  -u $udp_ccfc_port_local
mavlink stream -r 50 -s CC_TELEMETRY_IMU    -u $udp_ccfc_port_local
mavlink stream -r 10 -s CC_TELEMETRY_POWER  -u $udp_ccfc_port_local
mavlink stream -r  5 -s CC_TELEMETRY_GPS    -u $udp_ccfc_port_local
mavlink stream -r 10 -s CC_TELEMETRY_ESTIMATOR -u $udp_ccfc_port_local
mavlink stream -r 20 -s CC_TELEMETRY_ACTUATOR  -u $udp_ccfc_port_local
mavlink stream -r 25 -s CC_EVENT            -u $udp_ccfc_port_local
mavlink stream -r 25 -s CC_SAFETY_STATUS    -u $udp_ccfc_port_local
```

Ports 24540/24040 collide with nothing in the file (18570 GCS, 14580
offboard, 14280 payload, 13030 gimbal, 19450 sihsim). `onboard` mode
matches spec §2.1 (companion-appropriate defaults); the standard onboard
streams also flow — harmless on loopback UDP and realistic. Event-driven
streams get a 25 Hz *polling ceiling* (they emit only on new uORB data).
On the real vehicle this block's serial equivalent is `MAV_1_CONFIG =
TELEM3` + rcS extras (Phase 8).

### A.6 Verification plan (`tools/phase3/sitl_phase3_check.py`)

Shared pxh plumbing moves to `tools/common/pxh.py` (phase 2 harness
imports it too — re-run phase 2 suite as regression). New harness, stdlib
+ generated pymavlink module only, self-bootstrapping onto the pinned
venv:

0. **Preflight**: fork-XML == companion-XML (sha256); python bindings
   decode `golden_frames.bin` 16/16 (already demonstrated standalone).
1. Boot headless SIH SITL; `udpin` on the CCFC remote port; require
   HEARTBEAT (link up, MAVLink 2).
2. **FC→CC streams**: for each of the six telemetry streams collect
   samples; assert `schema_version==1`; rate via Δ`sequence`/Δ`fc_timestamp_us`
   within ±20% of the uORB-ceiling expectation (25/50/10/5/10/16.7 Hz);
   sequence gap ratio < 5% (UDP loopback should drop ~nothing);
   `px4_boot_id` constant across STATE samples; spot value plausibility
   (GPS fix, voltage band, normalized q).
3. **Gauntlet, message by message** (counters read from `mavlink status`
   before/after each injection; `cc_health_report` observed via pxh
   single-shot `listener`):
   - 3 valid reports (seq 1,2,3) → accepted=3, listener shows seq 3;
   - wrong source component (42) → `bad_source` +1, nothing published;
   - bad schema (99) → `bad_schema` +1;
   - out-of-range severity (7) → `bad_range` +1;
   - duplicate sequence → `dup_seq` +1;
   - gap (jump to 10) → accepted, `missed_reports` +6;
   - 100-report burst → accepted ≤ window cap, `flood_dropped` ≥ 78,
     receiver alive (next valid report still accepted);
   - `CC_MISSION_CONTEXT` valid (vehicle_id = `CC_VEHICLE_ID`, real
     dialect_hash) → `cc_mission_context` published AND
     `CC_TELEMETRY_STATE.mission_id` on the wire flips to the sent value
     (the end-to-end echo);
   - `CC_MISSION_CONTEXT` wrong vehicle_id → rejected, mission_id
     unchanged;
   - `CC_AI_DIAGNOSTIC` valid → `cc_ai_diagnostic` published (log-only
     path).
4. Evidence capture: `mavlink status` CCFC block + stream statistics into
   the committed run log.

Exit criteria (dev plan): *"scripted SITL session shows all FC→CC streams
decoded externally, and CC→FC reports pass/fail the gauntlet exactly as
specified"* — every line above is an assertion, not an observation.

### A.7 Risks / notes

* The `-r` values in `mavlink stream` are wall-clock-domain inside the
  mavlink module while uORB publication is lockstep-sim-domain; since
  send() is update-gated, mismatch only *lowers* wire rate. Rates are
  asserted from message-internal fields (sequence/fc_timestamp), immune to
  both domains.
* `mavlink status` output format is upstream-owned; the CCFC block is
  clearly delimited (`CCFC rx:` prefix) so parsing is anchored to our own
  lines only.
* The onboard-mode default stream set adds bandwidth on the CCFC instance;
  irrelevant for UDP loopback, revisit for TELEM3 serial budgeting in
  Phase 8 (spec §8 budget covers CC_* only — the real link will run a
  trimmed mode).

---

## Part B — Implementation notes

### B.1 Fork footprint (Phase 3 increment on `PX4-Autopilot-CCFC` main)

New files:

```
ccfc_dialect/cc_dialect.xml            vendored wire contract (verbatim companion copy)
ccfc_dialect/install.sh                copies XML into the mavlink submodule pre-build,
                                       refuses on hash mismatch vs the committed header
ccfc_dialect/PROVENANCE.md             source + SHA-256 + update procedure
src/include/ccfc/cc_dialect_hash.h     CC_DIALECT_HASH/SHA constants (receiver compiles in)
msg/CcMissionContext.msg               D9 — receiver→publisher/logger handoff
msg/CcAiDiagnostic.msg                 D10 — log-only evidence topic
src/modules/mavlink/streams/CC_TELEMETRY_{STATE,IMU,POWER,GPS,ESTIMATOR,ACTUATOR}.hpp
src/modules/mavlink/streams/CC_EVENT.hpp          (D12 mapping from PX4 events)
src/modules/mavlink/streams/CC_SAFETY_STATUS.hpp  (silent until Phase 6)
```

Modified upstream files (every edit under a `CCFC fork:` comment):

| File | Change |
|---|---|
| `boards/px4/sitl/default.px4board` | `CONFIG_MAVLINK_DIALECT="development"` → `"cc_dialect"` |
| `msg/CMakeLists.txt` | register the two new messages |
| `src/modules/mavlink/mavlink_messages.cpp` | include + register the 8 streams, gated on `MAVLINK_MSG_ID_CC_TELEMETRY_STATE` so common-dialect boards still build |
| `src/modules/mavlink/mavlink_receiver.h` | handler declarations, publications, gauntlet state + counter struct, public `ccfc_print_stats()` |
| `src/modules/mavlink/mavlink_receiver.cpp` | switch cases + the three handlers implementing the §4.4 gauntlet |
| `src/modules/mavlink/mavlink_main.cpp` | one line: `display_status()` prints the CCFC counters |
| `src/modules/cc_telemetry_publisher/*` | subscribes `cc_mission_context`, echoes `mission_id` into STATE; `CC_VEHICLE_ID` param added (D11) |
| `ROMFS/px4fmu_common/init.d-posix/px4-rc.mavlink` | CCFC companion-link instance (ports 24540/24040) + 8 `mavlink stream` rate commands |

companion repo: `tools/phase3/` (bindings generator + harness),
`tools/common/pxh.py` (pty/pxh plumbing extracted from the Phase 2 harness,
which now imports it — its load-bearing constraints are documented in the
module header), `tools/phase2/build_px4.sh` now runs the fork's dialect
installer before every build.

### B.2 Decisions & deviations log (continuing Phase 2's numbering)

| # | Decision | Rationale |
|---|---|---|
| D9 | `CcMissionContext.msg` added (9th uORB message) | spec §4.4 requires storing + echoing the handshake and giving it to the logger, but §4.1's msg list has no carrier; a uORB topic is the PX4-idiomatic handoff |
| D10 | `CcAiDiagnostic.msg` added (10th) | same gap: "validate → log-only uORB" needs a topic ULog can subscribe |
| D11 | `CC_VEHICLE_ID` param (int32, default 1) | spec §7 mandates a PX4-param vehicle identity but §12 never names one |
| D12 | `CC_EVENT` sourced from PX4's `event` topic: severity from the internal log level (err+→CRITICAL, warn/notice→WARN, else OK), `sequence` from the events interface's own u16 counter, `subsystem=NONE` | spec defines no CcEvent uORB by design; PX4 events carry no CC subsystem taxonomy — honest minimal mapping, refine when the monitor lands |
| D13 | Gauntlet order literal to spec (sequence check *then* flood check) | flood-dropped-but-valid reports advance the sequence window — they were real traffic, only rate-limited; keeps `missed_reports` honest |
| D14 | vehicle-id mismatch counted as `bad_source`, dialect-hash mismatch as `bad_schema` | spec's `CC_REJECT_*` taxonomy has no dedicated codes; these are the closest semantics (wrong identity / wrong contract) |
| D15 | Counters live in `MavlinkReceiver` and print via `mavlink status` (`CCFC rx …` lines, stable single-line format) | spec §4.4 says "surfaced by print_status()"; the mavlink module's status is its print surface. The harness sums the lines across instances (only the CCFC instance receives CC traffic) |
| D16 | `CC_LOG_CONTROL` not handled | spec §3.6/§4.4 marks it optional/dev-only, param-gated; no phase needs it yet — rejecting-by-ignoring keeps the attack surface at zero until it earns its keep |

### B.3 Verification tooling notes

* The pymavlink bindings are generated per-run from the pinned venv
  (`gen_python.sh`), never vendored, and **self-verify against the Phase 1
  golden vectors (16/16) plus the fork↔companion XML sha before any SITL
  packet is trusted** — a three-language wire agreement (C encoder → Rust
  round-trip in Phase 1 → Python decode here) all rooted in one XML.
* Stream rates are measured as Δ`sequence`/Δ`fc_timestamp_us` from message
  *content* (sim-time on both axes — immune to lockstep/sim-speed and to
  wall-clock scheduling of the mavlink sender). Wire-continuity is asserted
  separately: frames received / sequence span ≥ 95% on loopback UDP.
* Gauntlet assertions are **exact counter deltas** (`expect_delta`: the
  named counters move by exactly N, every other counter stays 0), read from
  `mavlink status` before/after each injection — the same
  counters-match-injected-faults discipline the Phase 1 fuzz suite set.
* The flood assertion tolerates the sim/wall clock ratio (accepted ≤ 25 of
  100, dropped ≥ 75, sum exact) because the 1 s flood window runs on sim
  time while the harness injects on wall time.

## Part C — Verification results

**Run:** 2026-07-15, macOS host, headless SIH, `px4_sitl_default` @
v1.17.0 + CCFC (dialect `cc_dialect`). **50/50 checks passed** (full log:
`tools/phase3/last_run.log`; rerun:
`tools/phase2/build_px4.sh && tools/phase3/sitl_phase3_check.py`).

| Exit criterion (dev plan) | Result |
|---|---|
| Preflight wire agreement | fork XML == companion XML (`dc5b8e9f`); Python bindings decode the Phase 1 golden vectors **16/16** — C/Rust/Python concur before SITL |
| All FC→CC streams decoded externally | over 12 s: STATE 403 frames @ **25.0 Hz**, IMU 807 @ **50.1**, POWER 161 @ **10.0**, GPS 81 @ **5.0**, ESTIMATOR 162 @ **10.0**, ACTUATOR 270 @ **16.7** — every rate exact (Δseq/Δfc_timestamp), continuity ≥ 99.8%, `schema_version==1` on every frame, `px4_boot_id` constant, values plausible (‖q‖=1.000, GPS 3D fix, 16.2 V) |
| Reports pass/fail the gauntlet exactly | 3 valid → `reports +3` and uORB seq 3; wrong compid → `bad_source +1`; schema 99 → `bad_schema +1`; severity 7 → `bad_range +1`; duplicate → `dup_seq +1`; gap to 10 → accepted + `missed +6`; **flood of 100 → exactly 20 accepted / 80 dropped**, no other counter moved; receiver alive after flood |
| Handshake | valid CC_MISSION_CONTEXT → `context +1`, uORB mission_id 777001, **mission_id echoed on the CC_TELEMETRY_STATE wire within 3 s**; wrong vehicle_id → `bad_source +1`, echo unchanged; wrong dialect_hash → `bad_schema +1` |
| Log-only diagnostic | CC_AI_DIAGNOSTIC → `diagnostic +1`, uORB `detail_code 0x0BAD`, no policy path touched |
| Phase 2 regression | phase 2 harness re-run green after the dialect switch + rcS changes (see ../README.md status) |

Counter block at end of run (CCFC mavlink instance):
`accepted: reports 25 context 1 diagnostic 1 · dropped: bad_source 2
bad_schema 2 bad_range 1 dup_seq 1 flood 80 missed_reports 95` — the 95
missed = the injected gap (6) plus the post-flood jump to seq 200 (89),
both intentional.
