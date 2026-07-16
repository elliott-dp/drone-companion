# Phase 2 — PX4 uORB topics + `cc_telemetry_publisher` (SITL)

**Goal (dev plan):** *"curated telemetry exists inside PX4, correctly
rate-controlled."* This is the first PX4-side phase: the eight `Cc*.msg`
uORB definitions and the `cc_telemetry_publisher` module, proven in SITL —
no MAVLink yet (that is Phase 3: this phase ends at the uORB boundary).

---

## Part A — Plan

### A.1 Base firmware: which PX4, and why

| Decision | Value | Rationale |
|---|---|---|
| Tree | `code/PX4-Autopilot-CCFC` | pre-existing clone, reused (full history, clean tree) |
| Pin | **tag `v1.17.0`** (commit `d6f12ad1c4`) | the dev plan demands a release tag, *never `main`* — the clone arrived on `main` at `v1.18.0-beta1-43`, which is beta-line code, unacceptable for flight safety. GitHub designates v1.17.0 (2026-05-13) the **latest stable release**; v1.18 is beta; v1.16.2 is the previous line's last patch. |
| Fallback stance | v1.16.2 remains a one-line re-pin if v1.17.0 shows regressions before Phase 8 (bench HITL) | zero patch releases have landed on v1.17 yet; if that changes, re-evaluate. Record any re-pin here and in the spec header. |

Submodules are initialized **selectively** (SITL needs ~11 of 29; the rest
are simulator assets and NuttX trees measured in gigabytes):
`mavlink/mavlink` (recursive — it nests pymavlink), `gps/devices`,
`events/libevents`, `uxrce_dds_client/Micro-XRCE-DDS-Client`,
`cdrstream/{cyclonedds,rosidl}`, `crypto/{monocypher,libtomcrypt,libtommath}`,
`heatshrink`, `Tools/simulation/gz` (source presence only; gz is not built —
see A.5).

### A.2 The eight uORB messages (`msg/Cc*.msg`)

Mirrors of the dialect payloads (`cc-dialect/cc_dialect.xml` — **unchanged
in this phase**; no binding regeneration needed). uORB conventions applied:
first field is `uint64 timestamp` (µs since boot, = the dialect's
`fc_timestamp_us` at the MAVLink boundary in Phase 3); fields ordered
size-descending; one uORB topic per message, default naming
(`CcTelemetryState` → `cc_telemetry_state`).

Published by `cc_telemetry_publisher` (six):

| Topic | Sources (v1.17 uORB) | Mapping notes |
|---|---|---|
| `cc_telemetry_state` | `vehicle_status`, `vehicle_attitude`, `vehicle_angular_velocity`, `vehicle_local_position`, `vehicle_control_mode`, `failsafe_flags` | `vehicle_type` ← `vehicle_status.system_type` (MAV_TYPE, matches HEARTBEAT semantics); `control_mode_flags`/`failsafe_flags` compacted per bit tables **defined in the .msg comments** (the .msg file is the authority) |
| `cc_telemetry_imu` | `vehicle_imu`, `vehicle_imu_status` (primary instance), `sensor_combined` | `accel`/`gyro` ← `sensor_combined`; deltas ← `vehicle_imu`; `clipping_count` ← Σ `accel_clipping[3]`; `vibration_metric` ← `[accel_vibration_metric, gyro_vibration_metric, delta_angle_coning_metric]` — PX4 v1.17 exposes *scalar* metrics, not per-axis, so the three slots carry the three metrics (deviation D2) |
| `cc_telemetry_power` | `battery_status` (instance 0) | `power` = V·I; PX4 `@invalid` markers (-1/NaN) forwarded as the dialect's NaN-if-unknown |
| `cc_telemetry_gps` | `sensor_gps` | v1.17 uses `double latitude_deg` → `int32 degE7` (×1e7, rounded); `alt` ← `altitude_msl_m`×1e3 mm; `heading` ← `cog_rad` (NaN when unknown); `noise_per_ms`/`jamming_indicator` clamped int32→uint16 |
| `cc_telemetry_estimator` | `estimator_status` (instance from `estimator_selector_status.primary_instance`) | `status_flags` ← `filter_fault_flags` (u32); `innovation_check_flags`, `solution_status_flags` as-is (u16); test ratios ← `vel/pos/hgt/mag/tas_test_ratio` (`mag` = `hdg_test_ratio`, `airspeed` = `tas`, NaN when N/A) |
| `cc_telemetry_actuator` | `actuator_motors` | trimmed per spec §6: outputs[8] + `motor_count` = count of leading finite entries; unused slots NaN; when the source has never published (disarmed before first allocation) the frame publishes with `motor_count = 0`, all NaN |

Defined now, published by later phases (two): `cc_health_report`,
`cc_safety_status` (Phase 3 receiver / Phase 6 monitor) — required by dev
plan Phase 2.1 so the full msg set builds once.

**Missing-source rule (spec invariant 7):** streams keep publishing at their
configured rate with *explicit invalidity* — NaN floats, `estimator_valid=0`,
`connected=0`, `fix_type=0`, `motor_count=0` — never frozen stale values,
never fabricated ones. Rationale: the CC-side stream watchdogs (Phase 4)
key on stream *presence*; validity is per-field data, and PX4 must not
conflate "source quiet" with "link down".

### A.3 `cc_telemetry_publisher` module design

* **Pattern:** `ModuleBase` + `ModuleParams` + `px4::ScheduledWorkItem` on
  `wq_configurations::lp_default` (spec §4.1). No thread, no poll-blocking.
* **Tick:** fixed 50 Hz base tick (`AI_UART`/`MINIMAL`); 200 Hz only in
  `AI_ETH`/`DEBUG` (spec §4.2). Each stream holds an integer decimation
  divider chosen as the **nearest-actual-rate** divisor of the tick: a
  requested rate that does not divide the tick picks whichever of
  floor/ceil dividers lands closest (e.g. the spec's 20 Hz actuator stream
  on a 50 Hz tick → divider 3 → 16.7 Hz actual, not 25 Hz). Actual rates
  are therefore always exact tick divisors; the harness computes its
  expectations with the same rule.
* **Rates by profile** (`CC_TEL_PROFILE`):

  | Stream | MINIMAL(0) | AI_UART(1) | AI_ETH(2) | DEBUG(3) |
  |---|---|---|---|---|
  | state | off | 25 Hz | 50 Hz | 50 Hz |
  | imu | off | `CC_TEL_IMU_RATE` (50) | `CC_TEL_IMU_RATE`, ceiling 200 | same as ETH |
  | power | off | 10 Hz | 20 Hz | 20 Hz |
  | gps | off | 5 Hz | 10 Hz | 10 Hz |
  | estimator | off | 10 Hz | 20 Hz | 20 Hz |
  | actuator | off | `CC_TEL_ACT_RATE` (20 → **16.7 actual**, divider rule) | `CC_TEL_ACT_RATE`, ceiling 50 | same as ETH |

  `MINIMAL` = "heartbeat + safety only" (spec enum) → the publisher goes
  silent entirely (deviation D1, documented).
* **Params:** `CC_TEL_PROFILE` (0–3, default 1), `CC_TEL_IMU_RATE`
  (0–200 Hz, default 50), `CC_TEL_ACT_RATE` (0–50 Hz, default 20). All
  re-read via `parameter_update` subscription → **live rate changes without
  reboot** (spec §12), verified by the harness.
* **Sampling rule:** on each due tick, `copy()` the newest sample of each
  input (uORB latest-value semantics); never queue history, never block
  (spec §4.2).
* **Identity:** `px4_boot_id` minted once at module start
  (wall-clock seconds XOR `hrt_absolute_time()`, nonzero-forced — uniqueness
  across boots, not cryptography, per spec §4.2's "simply a random 32-bit
  drawn at init"); per-stream `uint32 sequence` incremented on publish,
  never reset (wraps mod 2³²); `mission_id` = 0 until Phase 3's
  CC_MISSION_CONTEXT provides one (dialect: "0 if none").
* **Prohibitions (spec §4.2):** no heap allocation after `init()` (fixed
  members only, no containers/strings), no file I/O, no float formatting in
  the hot path, no behavior conditioned on subscriber presence.
* **`print_status()`:** profile, tick rate, boot id, and per stream:
  configured rate, divider, publish count, last publish age.
* **Integration points (each a one-line, clearly-ours edit):** register the
  eight messages in `msg/CMakeLists.txt`; `CONFIG_MODULES_CC_TELEMETRY_PUBLISHER=y`
  in `boards/px4/sitl/default.px4board`; `cc_telemetry_publisher start` in
  `ROMFS/px4fmu_common/init.d-posix/rcS`. (V6X board enablement is
  deliberately deferred to Phase 8 bench prep.)

### A.4 Verification strategy (the "tests as before")

The dev plan's Phase 2 verification is interactive (`listener`, `param set`,
eyeballing). As with Phase 1, that gets turned into a **scripted,
assertion-based harness**: `tools/phase2/sitl_phase2_check.py` (Python
stdlib only — no dependencies).

Mechanics: PX4 SITL runs **headless with the built-in SIH simulator**
(`simulator_sih` module — airframe `10040_sihsim_quadx`), which needs no
Gazebo/jMAVSim/Java and works on macOS. The harness starts the `px4` server
process, then drives it through PX4's client model (`px4-listener`,
`px4-param`, `px4-commander`, … talk to the daemon over its local socket).
Rates are measured from the `timestamp` fields inside `listener -n N`
samples — not wall clock — so lockstep quirks don't skew them.

Assertions (exit criteria mapped):

1. **All six topics publish at profile rates** — rate from sample
   timestamps within ±20% for every topic at AI_UART defaults.
2. **Sequence monotonicity** — strictly +1 per sample within each listen
   window; `px4_boot_id` nonzero and constant across the whole run.
3. **Params change rates live** — `param set CC_TEL_IMU_RATE 10` → imu
   re-measured ~10 Hz; restore → ~50 Hz; `param set CC_TEL_PROFILE 0`
   (MINIMAL) → state topic silent within a timeout; back to 1 → flowing.
4. **Plausible values while flying** — arm + `commander takeoff` under SIH:
   `estimator_valid=1`, quaternion normalized (‖q‖≈1), local position
   altitude increases on climb, actuator `motor_count=4` with finite
   outputs in [0..1], battery voltage in a sane band, GPS fix_type ≥ 3.
5. **`print_status()`** — reports all six streams with counts advancing.
6. **Work-queue load** — `work_queue status` captured; `cc_telemetry_publisher`
   present on `wq:lp_default`; interval sane. (CPU% is measured properly on
   the V6X in Phase 8; SITL numbers are recorded, not gated.)
7. **No-heap-after-init** — by construction & review (fixed-size members
   only); noted in the code-review checklist. (Heap instrumentation on
   NuttX comes with Phase 8 bench work.)

### A.5 Build environment decisions (macOS host)

Pinned in `tools/phase2/build_px4.sh` (the harness and CI call it):

* Python deps for PX4's codegen from `PX4-Autopilot-CCFC/.venv-px4`
  (created from PX4's own `Tools/setup/requirements.txt`).
* `GIT_SUBMODULES_ARE_EVIL=1` — the selective submodule set stays as-is;
  PX4's Makefile must not pull every simulator asset.
* Gazebo modules **skipped** via `CMAKE_DISABLE_FIND_PACKAGE_{gz-transport,
  gz-sim,gz-sensors,gz-plugin,Protobuf}`: this host's Homebrew gz install
  has broken cmake exports (gz-gui8 Qt target), Homebrew protobuf 35
  requires C++17 while PX4's `gz_msgs` compiles C++14, and — decisively —
  Phase 2 uses SIH, not Gazebo. All five packages gate cleanly
  (`if (FOUND)`) in upstream cmake, so this is a skip, not a patch.

### A.6 Risks / open items

* v1.17.0 has no patch releases yet — watch upstream for v1.17.x before
  Phase 8; re-pin is one line.
* SIH provides GPS/baro/IMU/mag but is a simplified dynamics model — value
  plausibility checks are bounded loosely; fidelity checks belong to later
  phases (real bench data).
* The `.msg` mirrors must stay in lockstep with `cc_dialect.xml`. Until a
  generator enforces it (worth adding in Phase 3 when the MAVLink streams
  bind the two together), the rule is procedural: any XML change touches
  `msg/Cc*.msg` in the same commit — the Phase 3 stream classes will fail
  to compile on mismatch, which is the backstop.

---

## Part B — Implementation notes

### B.1 Fork footprint (kept deliberately minimal)

New files, all clearly CCFC-marked:

```
msg/CcTelemetryState.msg      msg/CcTelemetryImu.msg    msg/CcTelemetryPower.msg
msg/CcTelemetryGps.msg        msg/CcTelemetryEstimator.msg
msg/CcTelemetryActuator.msg   msg/CcHealthReport.msg    msg/CcSafetyStatus.msg
src/modules/cc_telemetry_publisher/{CcTelemetryPublisher.hpp,.cpp,params.c,Kconfig,CMakeLists.txt}
```

Modified upstream files — 3 files, 19 lines, each edit under a `# CCFC fork:`
comment:

| File | Change |
|---|---|
| `msg/CMakeLists.txt` | register the eight `Cc*.msg` |
| `boards/px4/sitl/default.px4board` | `CONFIG_MODULES_CC_TELEMETRY_PUBLISHER=y`; `CONFIG_MODULES_UXRCE_DDS_CLIENT=n` (unused ROS 2 bridge whose ExternalProject breaks on spaces-in-path; see A.5) |
| `ROMFS/px4fmu_common/init.d-posix/rcS` | `cc_telemetry_publisher start` after `navigator start` |

### B.2 Decisions & deviations log

| # | Decision | Rationale |
|---|---|---|
| D1 | `MINIMAL` profile = publisher fully silent | spec enum text: "heartbeat + safety only" |
| D2 | `vibration_metric[3]` = `[accel_vib, gyro_vib, coning]` | v1.17 exposes scalar metrics, not per-axis; slots documented in the .msg |
| D3 | Nearest-actual-rate divider; spec's 20 Hz actuator on the 50 Hz tick runs at **16.7 Hz** | integer decimation of a fixed tick cannot produce 20 Hz; 16.7 (−16 %) beats 25 (+25 %), and the rule is mirrored in the harness |
| D4 | Estimator stream follows `estimator_selector_status.primary_instance` via `ChangeInstance()` | V6X flies multi-EKF; SITL proves the mechanism |
| D5 | IMU deltas/status from instance 0 | SIH runs one IMU; primary-IMU matching by device id is Phase 8 bench work |
| D6 | On stale estimator source: ratios go NaN, flag words carry last-known values | flags zeroed would fake "no faults"; NaN ratios are the staleness marker |
| D7 | `innovation_check_flags` re-mapped from v1.17's `estimator_status_flags.reject_*` bools (bit table in the .msg) | upstream deleted the legacy bitmask; wire field kept, semantics documented |
| D8 | `px4_boot_id` = wall-clock ⊕ hrt at module start, forced nonzero | uniqueness across boots is the requirement, not crypto (spec §4.2 allows exactly this) |

### B.3 Verification tooling (`tools/phase2/`)

* `build_px4.sh` — pinned reproducible build (venv python deps, selective
  submodules, gz/protobuf skips per A.5).
* `sitl_phase2_check.py` — the Phase 2 test suite (assertions in A.4).
  Transport lessons encoded in its header comment, found the hard way:
  * **Upstream bug: `listener <topic> -n N` is unusable on SITL** (v1.17.0,
    `src/systemcmds/topic_listener/listener_main.cpp`). Its wait loop calls
    plain `poll()` on a uORB subscription handle, which never signals on
    the POSIX build → no samples ever print; and its "abort key" check is
    `read(0, &c, 1); if (ret) return;` — **any** stdin byte exits (the
    q/ESC/ctrl-C switch below is dead code). Net behavior: `-n` mode hangs
    until any input arrives, then quits silently. Worth reporting upstream.
  * Therefore the harness uses **single-shot `listener <topic>`** only (a
    synchronous latest-sample print with no poll loop — the path that
    works) and measures rates as **Δsequence/Δtimestamp between two
    snapshots**: both axes are sim-time, immune to lockstep/sim-speed
    quirks, and the measurement doubles as proof that the sequence counter
    tracks publishes one-to-one.
  * Commands go strictly one at a time — stdin bytes reach the *running
    command*, not pxh, so queued input corrupts the session.
  * PX4 log output (`PX4_INFO`, `status`) is unbuffered stderr, but
    listener samples are raw stdout — **fully buffered on a pipe**, i.e.
    invisible to a pipe-driven harness. The harness therefore runs pxh on
    a **pseudo-terminal** (Python `pty`). The px4 daemon+client transport
    exhibits the same failures and was abandoned.
  * The SITL rootfs lives in the system temp dir: PX4 startup breaks when
    the working directory contains spaces ("UAV Project/…").
  * Silence assertions (MINIMAL profile) compare the module's own publish
    counters between two `status` calls — same time base on both sides.

## Part C — Verification results

**Run:** 2026-07-15, macOS host, headless SIH (`10040_sihsim_quadx`),
`px4_sitl_default` @ v1.17.0 + CCFC changes. **37/37 checks passed**
(full log: `tools/phase2/last_run.log`, transcript:
`tools/phase2/px4_server_last.log`; rerun with
`tools/phase2/build_px4.sh && tools/phase2/sitl_phase2_check.py`).

| Exit criterion (dev plan) | Result |
|---|---|
| All six topics publish at profile rates | state **25.0**, imu **50.0**, power **10.0**, gps **5.0**, estimator **10.0**, actuator **16.7** Hz — every one exactly the divider-rule expectation, measured as Δsequence/Δtimestamp |
| Sequence monotonicity + identity | strictly increasing everywhere; `px4_boot_id` nonzero, identical across all streams and `print_status` |
| Params change rates live | `CC_TEL_IMU_RATE` 50→10→50 re-measured 10.0 / 50.0 Hz mid-run, no reboot; `CC_TEL_PROFILE`→MINIMAL froze all six publish counters exactly, →AI_UART restored 25.0 Hz |
| Plausible values while flying | armed + takeoff under SIH: `estimator_valid=1`, ‖q‖=1.000, climbed to NED z −2.51 m, `motor_count=4` with hover mix [0.516, 0.431, 0.450, 0.503] and NaN unused slots, battery connected 15.49 V, GPS 3D fix / 10 sats, `velocity_test_ratio` 0.070, `airspeed_test_ratio` NaN (quad) |
| `print_status()` | all six streams listed with advancing counts; boot id echoed |
| Work-queue load | on `wq:lp_default` at 50.1 Hz, 19 975 µs actual vs 20 000 µs nominal; earlier steady-state capture: 19 996.9 µs avg, **0.0 µs rms** over 6 494 cycles |
| No heap after init | by construction (fixed-size members only, single `new` in `task_spawn`); review-verified. NuttX heap instrumentation lands with Phase 8 bench work |

Notes for the record: the SIH lockstep sim on this host runs faster than
wall clock at times; all rate measurements are sim-time-based by design, so
this does not affect the results. The `listener -n` upstream bug (B.3) cost
most of the harness iterations — the committed harness avoids that mode
entirely.
