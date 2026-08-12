# Documentation index

## Specification (the contract)

| Document | Role |
|---|---|
| [fc_cc_comm_architecture.md](fc_cc_comm_architecture.md) | Interface Control Document: invariants, transports, protocol, both software stacks, failure matrix |
| [development_plan.md](development_plan.md) | Phase 0–9 build plan; every phase ends demonstrable and tested |

## Implementation documentation (what exists, how it works)

| Document | Covers |
|---|---|
| [phase0_dialect_toolchain.md](phase1/phase0_dialect_toolchain.md) | **Phase 0.3** — dialect generation scripts (`gen_c.sh`, `gen_rust.sh`, `hash.sh`), pinned upstream definitions, determinism guarantees, CI wiring guidance |
| [phase1_protocol_layer.md](phase1/phase1_protocol_layer.md) | **Phase 1** — golden-vector mechanism (the CRC_EXTRA drift detector), the 16-frame golden set, fuzz/property suite, exit-criteria status, decisions & deviations log |
| [phase2_px4_telemetry.md](phase2/phase2_px4_telemetry.md) | **Phase 2** — PX4 v1.17.0 pin, the eight `Cc*.msg` uORB topics, `cc_telemetry_publisher` design + mappings, SIH SITL verification harness, results |
| [phase3_mavlink_link.md](phase3/phase3_mavlink_link.md) | **Phase 3** — dialect switch (`CONFIG_MAVLINK_DIALECT="cc_dialect"`), the 8 CC_* stream classes, receiver validation gauntlet + `mavlink status` counters, mission handshake + echo, pymavlink harness, results |
| [phase4_companiond.md](phase4/phase4_companiond.md) | **Phase 4** — the real Rust RX path: cc-link (transport/priority TX/heartbeat), cc-timesync (filter + runner), cc-ingest (continuity/age/watchdogs), companiond v0, fault drills + soak, CI diagnosis (B.4) |
| [phase5_mission_log.md](phase5/phase5_mission_log.md) | **Phase 5** — the crash-safe mission dataset: cc-config (layered), cc-mission-log (row-group-per-file Parquet, resume, shed ladder, raw capture), log-inspect (three-state verdict), companiond mission supervisor; judge-panel design + deviations; crash/disk-full/soak results (Part C) |
| [phase6_safety_loop.md](phase6/phase6_safety_loop.md) | **Phase 6** — the deterministic safety loop: PX4 `cc_safety_monitor` (pure host-tested policy table + state machine, edge-triggered Hold/Land/RTL, CC_MON_* params), `cc-health-tx` (scripted severity source, rate policy, ACK-tracking), companiond wiring; conservative-only invariants; 40 host + 12/12 SITL scenario results (Part C) |
| [cc_protocol_crate.md](phase1/cc_protocol_crate.md) | `crates/cc-protocol` reference — module layout, build-time binding generation, `FrameDecoder` semantics and counters, validation helpers, guidance for Phase 4 consumers |
| [../cc-dialect/README.md](../cc-dialect/README.md) | The dialect directory itself — layout, contract rules, the change workflow ("edit the XML" checklist) |

## Proposals (designed, not built)

| Document | Covers |
|---|---|
| [phase10_radar_harness.md](phase10/phase10_radar_harness.md) | **Phase 10 (proposed)** — the radar **harness**: MMWCAS ↔ Jetson link plus an environment any signal-processing/ML pipeline can be built in. Roles (FC is a thin relay: RC start/stop in, one compact vitals report out to ELRS), the airborne-transmit interlock, the two-path architecture (bulk raw record + reduced live tier), the pipeline extension point with Rust/Python/MATLAB parity, direct answers on CPU load and compressed storage, phases and exit criteria |
| [radar_transport_and_sync.md](phase10/radar_transport_and_sync.md) | The control-plane sequence (read from the open-source cascade tool), **capture modes spanning 100× in data rate** (VITALS-1 at 0.66 MB/s → SCAN-12 at 62.9 MB/s), the four possible data paths including CSI-2 straight into the Orin, and the timing architecture: no PTP on Orin Nano, hardware edge timestamping via Tegra HTE, the three-way frame ledger, and 12 gating bench tests |
| [radar_dataset_and_storage.md](phase10/radar_dataset_and_storage.md) | The dataset: format decision, layout, schemas, join keys, **measured compression results** (transform matters, not codec: 1.5–2.7× lossless; controlled loss budgeted in micrometres), ground-truth and negative-control channels, readers for Rust/Python/MATLAB with a CI parity test, integrity/versioning/privacy |
| [radar_dsp_ml_survey.md](phase10/radar_dsp_ml_survey.md) | The algorithm survey: the five numbers that constrain everything, every stage of the chain with its real alternatives (calibration, TDM Doppler trap, clutter removal, cell selection, phase extraction, harmonic problem, rate estimators, ego-motion compensation, multi-person, micro-Doppler), the ML landscape with a latency reality check, **what to test in order** and **what deliberately not to test**, the false-alarm catalogue, and metrics |
| [radar_realtime_budget.md](phase10/radar_realtime_budget.md) | Real-time architecture: task classes and deadlines (frame-rate DSP, soft live tier, **deferred ML decision path**), the compute budget with arithmetic (classic DSP ≈ 0.14 % of the Orin Nano's GPU; ML affordability is set by cadence, not model size), process/thread layout, the lossy shared-memory pipeline bridge, Jetson tuning, the degradation ladder, and 10 measurements to take on target |
| [radar_fc_integration.md](phase10/radar_fc_integration.md) | **The PX4-side integration spec**: three new dialect messages (54014 `CC_PAYLOAD_COMMAND`, 54015 `CC_VITALS_REPORT`, 54016 `CC_PAYLOAD_STATUS`) plus a `landed_state` extension on 54000 so the airborne interlock knows rather than infers; uORB topics, the `cc_payload_bridge` module (RC edge → command, repeat-until-acked), stream classes and receiver-gauntlet entries, `CC_PL_*` parameters, files-touched table, and the 868 MHz ELRS budget with its Listen-Before-Talk consequences |
| [radar_rbec_method.md](phase10/radar_rbec_method.md) | **Method proposal (RBEC)** — reference-beam ego-motion cancellation: target + reference beams from one datacube, the anchor estimators, shared-LO common-mode analysis, budgets, failure modes, and a six-rung validation ladder |
| [radar_rbec_validation.md](phase10/radar_rbec_validation.md) | **RBEC numerical validation (V1)** — seeded simulations ([`tools/phase10/rbec/`](../tools/phase10/rbec/README.md)) showing the budget closes on paper, plus the chest-velocity unwrap requirement and the calibration-spur nuance |
| [radar_primary_source_findings.md](phase10/radar_primary_source_findings.md) | The 2026-08 primary-source verification record: 55 claims → 26 confirmed / 21 nuanced / 6 refuted / 2 not-found, with citations and the corrections applied |
| [phase10_bench_manual.md](phase10/phase10_bench_manual.md) | **Phase 10.0 bench manual** — every E-test as a runnable protocol. The freeze-cals + calibration-report configuration (and the `calibPeriodicity` trap that would have silenced it), one shared low-IF profile serving all three capture modes, the sync-tap electrical design derived from the recovered MMWCAS-RF-EVM ODB++ netlist (the LMK00804B **does** pass a 20 Hz pulse; J4 pin 83 does **not** work as a tap; `TP1_1..4` do; contention is one 0402 removal), HTE bring-up on stock JetPack (pins 3/5 not 27/28; zero rate drift on the Jetson side), why E3 is impossible by construction, the randomised-stimulus upgrade to E10, the canonical bench data product, and the corrections it applies to the other documents |

## Status at a glance (2026-07-21)

| Item | State |
|---|---|
| Phase 0.1 repo layout / 0.2 toolchains / 0.4 CI | **owner: you** (per your split); scripts below are CI-ready |
| Phase 0.3 generation scripts | ✅ done, run and verified on this machine |
| Phase 1.1 vendored C headers + Rust build wiring | ✅ done (`cc-dialect/generated/c/`, `cc-protocol/build.rs`) |
| Phase 1.2 golden vectors from C | ✅ done — 16 frames, 749 bytes, deterministic regeneration |
| Phase 1.3 Rust golden round-trip | ✅ done — field-exact + byte-identical re-encode + CRC_EXTRA table check |
| Phase 1.4 fuzz/property suite | ✅ done — 12 tests, exact fault-counter accounting, never-panic sweeps |
| Phase 1 test suite | ✅ 29/29 green, clippy clean (`cargo test --workspace`) |
| Phase 2 PX4 base | ✅ `PX4-Autopilot-CCFC` pinned to **v1.17.0** (latest stable; was on `main`/v1.18-beta) |
| Phase 2 uORB topics + `cc_telemetry_publisher` | ✅ built into `px4_sitl_default`, zero warnings |
| Phase 2 SITL verification (SIH, headless) | ✅ **37/37 checks green** (`tools/phase2/sitl_phase2_check.py`; results in the phase 2 doc Part C) — re-verified after the Phase 3 dialect switch |
| Phase 3 dialect switch + 8 CC_* streams | ✅ mavlink module builds `cc_dialect`; streams registered, zero CCFC warnings |
| Phase 3 receiver gauntlet | ✅ source/schema/range/sequence/flood + mission handshake, counters in `mavlink status` |
| Phase 3 SITL verification (UDP, pymavlink) | ✅ **50/50 checks green** (`tools/phase3/sitl_phase3_check.py`; results in the phase 3 doc Part C) — re-verified on the Phase 4 `px4-rc.mavlink` |
| Phase 4 `cc-link` / `cc-timesync` / `cc-ingest` / `companiond` | ✅ built + clippy clean; **44 unit/property tests** (incl. MAVLink 1 decode) |
| Phase 4 MAVLink 1 timesync-reply fix | ✅ decoder accepts v1+v2 framing (D25); PX4 emits `TIMESYNC` replies as MAVLink 1 uncontrollably — diagnosed in phase 4 doc §C.1 |
| Phase 4 SITL integration + fault drills | ✅ **36/36 checks green** (`tools/phase4/sitl_phase4_check.py`; timesync LOCK ≤ 5 s, rates ±20%, garbage/pause/reboot drills; results in the phase 4 doc Part C) |
| Phase 4 fork edit | ✅ CC instance `-m custom` + explicit `HEARTBEAT` + `MAV_PROTO_VER 2`; **no PX4 C/C++ changed** (fork stays pinned to v1.17.0) |
| Phase 4 soak (1 h unattended, exit criterion) | ✅ **47/47 incl. soak** — 1 h, Δ0 gaps / 0 crc / 0 stale, timesync held LOCKED (305 716 frames); **exit criterion met** |
| Phase 5 `cc-config` | ✅ **13/13** — layered defaults→file→env→CLI, per-field precedence, cross-field validation |
| Phase 5 `cc-mission-log` | ✅ **29/29** + clippy clean — row-group-per-file crash-safety crux, resume-same-mission (§7), shed ladder, deterministic crash/disk-full lifecycle; arrow+parquet pinned to 59 |
| Phase 5 `log-inspect` + companiond supervisor | ✅ built — three-state verdict (Clean/Dirty/Corrupt), mission supervisor + handshake, pre-decode raw tap, status `log` object |
| Phase 5 SITL verification (clean/crash/disk-full) | ✅ **20/20** (`tools/phase5/sitl_phase5_check.py`; results in the phase 5 doc Part C) |
| Phase 5 soak (1 h `log-inspect`-clean mission, exit criterion) | ✅ **18/18** — 1 h mission CLEAN + complete, 354 550 rows, 0 gaps, 0 drops, 3 segments (30 min cap); **exit criterion met** |
| Phase 6 policy core (host) | ✅ **40/40** — pure `cc_policy_table.hpp` + `cc_state_machine.hpp`, one case per §4.5 row + hysteresis/staleness/reboot + exhaustive conservative-only sweep; **policy 100% host coverage — exit criterion met** |
| Phase 6 `cc_safety_monitor` module + `cc-health-tx` | ✅ builds + runs in SITL; module (state machine → policy → cc_safety_status + edge-triggered Hold/Land/RTL + pilot-override + CC_MON_*); cc-health-tx **11 tests** (rate policy, hysteresis, ACK-tracking) |
| Phase 6 SITL scenario suite | ✅ **12/12** (`tools/phase6/sitl_phase6_check.py`) — nominal/critical-ground/recovery/stale/garbage/disabled; airborne arm+takeoff+Land scenario is a follow-up (SIH not flight-ready) |
