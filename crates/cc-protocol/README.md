# cc-protocol

Wrapper over the build-time-generated `cc_dialect` MAVLink 2 bindings. This is
the lowest layer of the workspace: every other crate that touches wire bytes
or dialect types (`cc-link`, `cc-ingest`, `cc-health-tx`, `cc-ai-health`,
`cc-replay`) depends on this crate and never touches the generated code
directly.

## Scope

| In this crate | Not in this crate |
|---|---|
| Generated dialect bindings (`dialects::cc_dialect`) | Transport (sockets, serial) — `cc-link` |
| Build-time dialect-hash constants | Sequence continuity / staleness tracking — `cc-ingest` |
| Incremental frame decoding (`framing`) | Priority TX queuing — `cc-link` |
| Envelope validation: source, schema, range (`validate`) | Health-detection logic — `cc-ai-health` |
| Component-ID / schema-version constants (`identity`) | |

## Code generation (`build.rs`)

The Rust bindings are **never vendored** — `build.rs` regenerates them from
`cc-dialect/cc_dialect.xml` on every build, so they cannot drift from the XML
source of truth. Steps:

1. Stage `cc_dialect.xml` plus the pinned upstream includes
   (`cc-dialect/upstream/{common,standard,minimal}.xml`) into
   `$OUT_DIR/definitions`, so `<include>` resolution in the XML uses exactly
   these pinned files.
2. Run `mavlink-bindgen` on `cc_dialect.xml`. Because mavlink-bindgen follows
   the `<include>` chain from the staged directory, the generated
   `dialects::cc_dialect` module contains **one** `MavMessage` enum holding
   both the `CC_*` messages and the included common/standard/minimal messages
   (`HEARTBEAT`, `TIMESYNC`, …).
3. Compute `CC_DIALECT_HASH` (first 4 bytes of the XML's SHA-256, big-endian
   `u32`) and `CC_DIALECT_SHA256` (full hex digest) into
   `$OUT_DIR/dialect_hash.rs`. This must be bit-identical to the C side's
   `cc-dialect/hash.sh` output — `tests/dialect_hash_consistency.rs` asserts
   this against the committed `cc-dialect/generated/dialect_hash.rs`.

Build fails loudly (`assert!`) if `cc-dialect/` is not checked out next to
`crates/` — there is no silent fallback.

## Public API

### `dialects::cc_dialect` (generated, re-exported as `cc_dialect`)

The generated `MavMessage` enum and one `..._DATA` struct per message,
including all twelve `CC_*` messages (IDs 54000–54013) and the standard
messages pulled in via the XML `<include>` chain. `#[allow(clippy::all)]` —
this module is never hand-edited.

Convenience aliases in `lib.rs`:

```rust
pub type CcMavMessage = dialects::cc_dialect::MavMessage;
pub type CcFrameDecoder = framing::FrameDecoder<CcMavMessage>;
```

### `dialect_hash` (generated)

```rust
pub const CC_DIALECT_HASH: u32;      // first 4 bytes of SHA-256(cc_dialect.xml), BE
pub const CC_DIALECT_SHA256: &str;   // full hex digest
```

`CC_DIALECT_HASH` travels in `CC_MISSION_CONTEXT.dialect_hash`; the receiving
side rejects a mission handshake if its own build's hash disagrees (schema
mismatch, spec §7/§11).

### `identity`

```rust
pub const CC_SCHEMA_VERSION: u8;        // = 1
pub const SYSID_VEHICLE_DEFAULT: u8;    // = 1
pub const COMPID_FC: u8;                // = 1   (MAV_COMP_ID_AUTOPILOT1)
pub const COMPID_CC: u8;                // = 191 (MAV_COMP_ID_ONBOARD_COMPUTER)
```

FC and CC share one MAVLink system ID and are distinguished by component ID.
`CC_SCHEMA_VERSION` is bumped whenever a `CC_*` message's field *semantics*
change (additive fields go in extensions instead, per spec §3.2).

### `framing`

`FrameDecoder<M: Message>` — an incremental, push-based MAVLink frame
decoder, generic over the dialect's message enum (`CcFrameDecoder` is the
pre-bound alias for `MavMessage`).

```rust
impl<M: Message> FrameDecoder<M> {
    pub fn new() -> Self;
    pub fn push(&mut self, bytes: &[u8]) -> Vec<DecodedFrame<M>>;
    pub fn counters(&self) -> &DecodeCounters;
    pub fn pending(&self) -> usize;   // bytes buffered awaiting the rest of a frame
}
```

`DecodedFrame<M>` carries `header: MavHeader`, `message: M`, `msg_id: u32`,
`payload_len: u8`, `signed: bool`, `frame_len: usize`.

**Never panics on any byte stream.** Recognizes both `0xFD` (MAVLink 2 STX)
and `0xFE` (MAVLink 1 STX) — PX4's shared TIMESYNC/parameter modules emit some
standard low-ID messages as MAVLink 1 regardless of `MAV_PROTO_VER`, so a
production decoder must accept both framings. `push` can be called with
anything from single bytes to whole reads; framing state persists across
calls, and after every call the internal buffer holds at most one incomplete
frame prefix (`< V2_MAX_FRAME_LEN` bytes).

Resync policy (see `framing.rs` module docs for the full rationale):

- Bytes are scanned for an STX; every discarded byte counts as `garbage_bytes`.
- A CRC failure on a known-ID candidate discards **only the STX byte** and
  rescans from the next byte — conservative, so a real frame starting inside
  a corrupted candidate is never lost.
- An unknown message ID cannot be CRC-checked (`CRC_EXTRA` is per-message).
  If the declared frame length lands cleanly on the next STX or end of
  buffer, it's counted as `unknown_msg_ids`/`unknown_msg_bytes` and skipped
  whole; otherwise it's `suspect_candidates` and only the STX byte is
  discarded.
- Unknown incompatibility flags (anything but `MAVLINK_IFLAG_SIGNED`) force a
  drop per the MAVLink 2 spec (`bad_incompat_flags`).
- A CRC-valid frame whose payload fails semantic decode (e.g. an
  out-of-range enum discriminant) counts as `bad_payloads`; the whole frame
  is skipped since the CRC already proved the framing.

`DecodeCounters` accounts for **every byte pushed**, exactly:

```text
bytes_in == frames_ok_bytes + unknown_msg_bytes + bad_payload_bytes
            + garbage_bytes + pending()
```

(`crc_errors`, `bad_incompat_flags`, `suspect_candidates` count *events*; the
bytes of those candidates drain into `garbage_bytes` as the scanner steps
past them.) This invariant is asserted by the fuzz/property test suite after
every interaction.

### `validate`

Pure, stateless envelope-validation functions — no sequence continuity, no
staleness, no flood limiting (those are `cc-link`/`cc-ingest` concerns).

```rust
pub enum Direction { FcToCc, CcToFc, Common }
pub fn direction_of_id(msg_id: u32) -> Option<Direction>;

pub enum ValidateError {
    BadSource { msg_id: u32, system_id: u8, component_id: u8 },
    BadSchema { msg_id: u32, got: u8 },
    BadRange  { msg_id: u32, field: &'static str, value: u32, max: u32 },
}

pub fn schema_version_of(msg: &MavMessage) -> Option<u8>;
pub fn validate_schema(msg_id: u32, msg: &MavMessage) -> Result<(), ValidateError>;
pub fn validate_ranges(msg_id: u32, msg: &MavMessage) -> Result<(), ValidateError>;
pub fn validate_source_on_cc(header: &MavHeader, expected_sysid: u8, msg_id: u32) -> Result<(), ValidateError>;
pub fn validate_inbound_on_cc(header: &MavHeader, expected_sysid: u8, msg_id: u32, msg: &MavMessage) -> Result<(), ValidateError>;
```

`direction_of_id` maps a message ID to who is allowed to send it: `54000..=54008`
are FC→CC (54008 reserved for a future `CC_TELEMETRY_ESC`), `54010..=54013`
are CC→FC; unallocated IDs return `None`. `validate_inbound_on_cc` composes
source → schema → range checks in that order, matching the FC-side gauntlet's
check order (spec §4.4) so counters/reject reasons are comparable across both
ends. `validate_source_on_cc` rejects a message either from the wrong system
ID, or from the wrong component for its direction, or — critically — an
inbound message whose direction is `CcToFc` (the companion's own message
class looping back at itself, a wiring/config error).

`validate_ranges` currently checks `confidence_percent <= 100` on
`CC_HEALTH_REPORT`/`CC_AI_DIAGNOSTIC`; other range constraints are enforced by
the type system at decode time (out-of-range enum discriminants are rejected
by the generated `parse` and counted as `bad_payloads` in `framing`, not
reachable here).

## Testing

- `tests/golden_roundtrip.rs` — the CRC_EXTRA drift detector. Decodes
  `cc-dialect/golden/golden_frames.bin` (produced by the **C** encoder via
  `cc-dialect/golden/gen_golden.c`) through the Rust bindings, asserts every
  field matches the fixed golden values, re-encodes and asserts
  byte-identical output, and cross-checks `CC_MISSION_CONTEXT.dialect_hash`
  (embedded by the C side) against the Rust build-time hash. If this fails
  without an XML edit, the C and Rust toolchains have drifted — the fix is
  never to update the expected values without also checking the XML.
- `tests/dialect_hash_consistency.rs` — asserts the shell-computed
  (`hash.sh`) and Rust-computed (`build.rs`) dialect hashes agree.
- `tests/fuzz_decoder.rs` — property tests over `FrameDecoder`: never panics,
  resynchronizes after garbage/hot-plug, and the `DecodeCounters` invariant
  holds for arbitrary inputs and arbitrary chunk boundaries.
- `examples/replay_capture.rs` — replays a length-prefixed UDP datagram
  capture (`[u32 LE length][datagram bytes]` repeated) through
  `CcFrameDecoder`, printing per-message-type counts and decoder counters.
  Debug scaffolding from the Phase 4 timesync investigation, kept for future
  wire-capture analysis.

## Dependencies

`mavlink-core` (re-exported, so downstream crates use exactly the
workspace-pinned version rather than risking a mismatched duplicate),
`bitflags`, `num-derive`, `num-traits` (used by the generated bindings).
Build-time only: `mavlink-bindgen`, `sha2`.
