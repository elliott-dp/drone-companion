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
| [cc_protocol_crate.md](phase1/cc_protocol_crate.md) | `crates/cc-protocol` reference — module layout, build-time binding generation, `FrameDecoder` semantics and counters, validation helpers, guidance for Phase 4 consumers |
| [../cc-dialect/README.md](../cc-dialect/README.md) | The dialect directory itself — layout, contract rules, the change workflow ("edit the XML" checklist) |

## Status at a glance (2026-07-20)

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
| Phase 5 soak (1 h `log-inspect`-clean mission, exit criterion) | ⏳ in progress (`--soak 3600`) |
