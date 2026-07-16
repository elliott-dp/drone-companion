# Documentation index

## Specification (the contract)

| Document | Role |
|---|---|
| [fc_cc_comm_architecture.md](fc_cc_comm_architecture.md) | Interface Control Document: invariants, transports, protocol, both software stacks, failure matrix |
| [development_plan.md](development_plan.md) | Phase 0–9 build plan; every phase ends demonstrable and tested |

## Implementation documentation (what exists, how it works)

| Document | Covers |
|---|---|
| [phase0_dialect_toolchain.md](phase0_dialect_toolchain.md) | **Phase 0.3** — dialect generation scripts (`gen_c.sh`, `gen_rust.sh`, `hash.sh`), pinned upstream definitions, determinism guarantees, CI wiring guidance |
| [phase1_protocol_layer.md](phase1_protocol_layer.md) | **Phase 1** — golden-vector mechanism (the CRC_EXTRA drift detector), the 16-frame golden set, fuzz/property suite, exit-criteria status, decisions & deviations log |
| [phase2_px4_telemetry.md](phase2_px4_telemetry.md) | **Phase 2** — PX4 v1.17.0 pin, the eight `Cc*.msg` uORB topics, `cc_telemetry_publisher` design + mappings, SIH SITL verification harness, results |
| [cc_protocol_crate.md](cc_protocol_crate.md) | `crates/cc-protocol` reference — module layout, build-time binding generation, `FrameDecoder` semantics and counters, validation helpers, guidance for Phase 4 consumers |
| [../cc-dialect/README.md](../cc-dialect/README.md) | The dialect directory itself — layout, contract rules, the change workflow ("edit the XML" checklist) |

## Status at a glance (2026-07-15)

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
| Phase 2 SITL verification (SIH, headless) | ✅ **37/37 checks green** (`tools/phase2/sitl_phase2_check.py`; results in the phase 2 doc Part C) |
