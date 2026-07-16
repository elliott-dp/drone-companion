# `cc-protocol` crate reference

The Jetson-side wrapper over the generated `cc_dialect` bindings
(spec §5.1: *"generated dialect bindings wrapper, envelope types,
validation"*). **No other crate may touch the raw generated code** — future
crates (`cc-link`, `cc-ingest`, …) consume this crate's re-exports, so a
binding change has exactly one blast radius.

```
crates/cc-protocol/
├── Cargo.toml          pinned deps; feature "dialect-cc_dialect" (see §2)
├── build.rs            stages XML → mavlink-bindgen → dialect hash
├── src/
│   ├── lib.rs          module wiring + re-exports + type aliases
│   ├── identity.rs     COMPID_FC=1, COMPID_CC=191, CC_SCHEMA_VERSION=1, …
│   ├── framing.rs      FrameDecoder — incremental MAVLink 2 parser + counters
│   └── validate.rs     direction/source/schema/range envelope checks
└── tests/
    ├── golden_roundtrip.rs         C↔Rust wire agreement (3 tests)
    ├── fuzz_decoder.rs             robustness + exact counters (12 tests)
    └── dialect_hash_consistency.rs hash.sh ↔ build.rs (2 tests)
```

---

## 1. Public surface (what downstream crates use)

```rust
use cc_protocol::cc_dialect::{MavMessage, CC_HEALTH_REPORT_DATA, CcSeverity, ...};
use cc_protocol::{CcMavMessage, CcFrameDecoder};       // aliases
use cc_protocol::dialect_hash::{CC_DIALECT_HASH, CC_DIALECT_SHA256};
use cc_protocol::identity::{COMPID_CC, COMPID_FC, CC_SCHEMA_VERSION};
use cc_protocol::framing::{FrameDecoder, DecodeCounters, DecodedFrame};
use cc_protocol::validate;
use cc_protocol::mavlink_core;   // re-export: everyone uses the SAME pinned runtime
```

The generated `cc_dialect` module contains the 12 CC_* messages **and** the
included common/standard/minimal messages (HEARTBEAT, TIMESYNC, COMMAND_LONG,
STATUSTEXT, …) in one `MavMessage` enum — one dialect, as spec §3.1 demands.
Generated typing worth knowing:

| XML construct | Rust shape |
|---|---|
| `enum="CC_SEVERITY"` field | `CcSeverity` enum, variants keep full names (`CcSeverity::CC_SEVERITY_WARN`) |
| `enum="CC_HEALTH_FLAGS"` (bitmask) | `CcHealthFlags` bitflags struct over `u32` |
| `char[24]` | `CharArray<24>` (`.to_str()`, NUL-aware) |
| `float[8]` | `[f32; 8]` |
| HEARTBEAT's `type` field | renamed `mavtype` (reserved word) |

## 2. `build.rs` — the generation pipeline

1. **Stage**: copies `cc-dialect/cc_dialect.xml` + the pinned
   `upstream/{common,standard,minimal}.xml` into `OUT_DIR/definitions/`
   (same staging rule as `gen_c.sh` — include resolution can only see pinned
   files) and emits `cargo:rerun-if-changed` for each.
2. **Generate**: `mavlink_bindgen::generate(XmlDefinitions::Files([cc_dialect.xml]), …)`.
   Passing only the dialect file makes bindgen **merge the include chain
   into a single module** — one `MavMessage`, no separate common/minimal
   modules to compile.
3. **Hash**: computes `CC_DIALECT_HASH`/`CC_DIALECT_SHA256` from the raw XML
   (sha2), independently of `hash.sh` — the test suite proves both pipelines
   agree (see `tests/dialect_hash_consistency.rs`).

Wiring notes (each encodes a real failure mode found while building this):

* The generated `mod.rs` gates the module behind
  `#[cfg(feature = "dialect-cc_dialect")]` → the feature **must exist and be
  default** in Cargo.toml. (With the feature absent, the module silently
  compiles to nothing — the crate "builds" while containing no dialect.)
* Generated code needs `bitflags`, `num-derive`, `num-traits` as normal
  deps — the same set the official `mavlink` facade crate declares.
* Generated code carries `cfg_attr` gates for optional integrations
  (`serde`/`ts-rs`/`arbitrary`); build.rs declares those cfg values via
  `cargo::rustc-check-cfg` so `unexpected_cfgs` stays meaningful for our
  own code instead of drowning in ~2000 generated warnings.
* `mavlink-core`/`mavlink-bindgen` are pinned `=0.18.0` in the workspace
  manifest: a silent minor bump of the MAVLink runtime must never be able to
  change emitted bytes. Bump deliberately, golden suite in the same commit.

## 3. `identity` — the addressing contract as constants

`SYSID_VEHICLE_DEFAULT = 1`, `COMPID_FC = 1` (MAV_COMP_ID_AUTOPILOT1),
`COMPID_CC = 191` (MAV_COMP_ID_ONBOARD_COMPUTER), `CC_SCHEMA_VERSION = 1`.
Spec §3.3/§3.4. `CC_SCHEMA_VERSION` bumps only on field-semantics changes
(additive fields go into MAVLink extensions instead).

## 4. `framing::FrameDecoder` — semantics you can rely on

Push-based incremental MAVLink 2 parser, generic over the dialect
(`CcFrameDecoder` = bound to `cc_dialect::MavMessage`):

```rust
let mut dec = CcFrameDecoder::new();
for frame in dec.push(&bytes_from_uart) {   // Vec<DecodedFrame<MavMessage>>
    // frame.header (sysid/compid/seq), frame.message, frame.msg_id,
    // frame.payload_len, frame.signed, frame.frame_len
}
dec.counters();  // &DecodeCounters
dec.pending();   // bytes buffered awaiting a frame's remainder
```

Properties (all pinned by tests):

* **Never panics** on any input; memory bounded (≤ one incomplete frame,
  < 280 bytes, retained across pushes).
* **Chunking-agnostic**: byte-at-a-time or bulk delivery decode identically.
* **Resync on 0xFD** after garbage/hot-plug (spec §2.4).
* **Byte conservation** — the accounting invariant:

  ```
  bytes_in == frames_ok_bytes + unknown_msg_bytes + bad_payload_bytes
              + garbage_bytes + pending()
  ```

### Fault policy table

| Situation | Action | Counters touched |
|---|---|---|
| bytes before an STX | consumed | `garbage_bytes` per byte |
| header/frame incomplete | wait (buffered) | `pending()` |
| unknown incompat flag (≠ 0x01) | drop STX, rescan | `bad_incompat_flags`; bytes drain to `garbage_bytes` |
| CRC failure (known message ID) | drop **only the STX byte**, rescan from next byte | `crc_errors`; frame's bytes drain to `garbage_bytes` |
| unknown message ID, declared end lands on an STX or buffer end | skip whole declared frame | `unknown_msg_ids`, `unknown_msg_bytes` |
| unknown message ID, end not on a boundary | treat as suspect garbage, drop STX | `suspect_candidates`; bytes → `garbage_bytes` |
| CRC ok, payload semantically invalid (bad enum) | skip whole frame (CRC proved framing) | `bad_payloads`, `bad_payload_bytes` |
| signed frame (IFLAG 0x01) | framed with 13-byte signature, surfaced `signed=true`, signature **not** verified (wired link uses no signing, spec §8) | `frames_ok` |

Why drop-one-byte on CRC failure (not the whole candidate): a genuine frame
that starts *inside* a corrupted candidate is never lost; the cost is a
rescan of < 280 bytes on a 92 kB/s link. Why the boundary check on unknown
IDs: an unknown-but-real message (schema skew — e.g. a future
CC_TELEMETRY_ESC from a newer FC) is counted once per frame with **zero
phantom CRC errors** (which would otherwise pollute the link-quality signal
that spec §2.1 treats as a wiring-fault indicator), while a false STX inside
garbage cannot swallow a real frame that follows.

Known residual (accepted, documented): unknown-ID frame lengths are taken on
trust after the boundary check — a pathological garbage candidate whose
declared span ends exactly on a later frame's mid-payload `0xFD` can cost
one real frame before resync recovers. Bounded, astronomically unlikely,
and irrelevant on a link where both ends share one XML.

### What FrameDecoder is NOT

No sequence-gap tracking, staleness windows, flood limiting, or
`rx_bad_source` — those are stateful link/stream concerns for `cc-link` /
`cc-ingest` (Phase 4), which will layer them on top of these counters
(spec §5.3/§5.5). The decoder is deliberately a pure protocol object.

## 5. `validate` — envelope checks

Pure functions; no state:

| Function | Checks | Spec |
|---|---|---|
| `direction_of_id(msg_id)` | FC→CC (54000–54008 incl. reserved ESC) vs CC→FC (54010–54013); `None` for standard/unallocated | §3.2 |
| `validate_source_on_cc(header, sysid, msg_id)` | system ID matches; FC-class messages must come from comp 1; CC-class arriving at the CC is a config error | §3.3 |
| `validate_schema(msg_id, msg)` | `schema_version == CC_SCHEMA_VERSION` for CC_* messages (standard messages pass) | §3.4 |
| `validate_ranges(msg_id, msg)` | `confidence_percent ≤ 100` (the one range the type system can't express — enums/bitmasks are already rejected at decode as `bad_payloads`) | §4.4 |
| `validate_inbound_on_cc(...)` | the three above, in gauntlet order | §4.4 |

The FC-side gauntlet (sequence window, 20 Hz flood cap, reject counters into
`CC_SAFETY_STATUS.reject_reason`) is C++ in `mavlink_receiver.cpp` — Phase 3.

## 6. Testing recap

29 tests: 12 unit (framing chunk/pending/garbage exactness, validate table),
3 golden round-trip, 2 hash consistency, 12 fuzz/property. Inventory and
fault matrix: [phase1_protocol_layer.md](phase1_protocol_layer.md) §4–5.

```sh
cargo test -p cc-protocol            # everything
cargo test -p cc-protocol --test golden_roundtrip
cargo test -p cc-protocol --test fuzz_decoder
```

## 7. Guidance for Phase 4 consumers (`cc-link`, `cc-ingest`)

* Feed raw transport reads straight into `FrameDecoder::push`; never
  pre-frame.
* Map counters onto the spec §5.3 names: `rx_frames` ← `frames_ok`,
  `rx_crc_errors` ← `crc_errors`, `rx_unknown_msg` ← `unknown_msg_ids`;
  add `rx_bad_source` and per-stream `sequence_gaps` at the ingest layer
  using `validate::validate_source_on_cc` + per-stream state.
* Derive link health (UP/DEGRADED/DOWN) from counter *deltas* per interval,
  not absolutes — the counters are cumulative u64s and never reset.
* `DecodedFrame.signed` should be treated as an anomaly flag on this link
  (log + count; never required).
