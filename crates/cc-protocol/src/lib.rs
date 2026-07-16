//! # cc-protocol
//!
//! Wrapper over the build-time-generated `cc_dialect` MAVLink 2 bindings
//! (spec §5.1: *"generated dialect bindings wrapper, envelope types,
//! validation"*). **No other crate touches the raw generated code** — higher
//! layers (`cc-link`, `cc-ingest`, …) consume this crate's re-exports.
//!
//! What lives here (Phase 1 scope):
//!
//! * [`dialects::cc_dialect`] — the generated bindings. `build.rs` stages
//!   `cc-dialect/cc_dialect.xml` plus the pinned upstream includes and runs
//!   mavlink-bindgen on every build, so the bindings can never drift from
//!   the XML. The module contains the CC_* messages **and** the included
//!   common/standard/minimal messages (HEARTBEAT, TIMESYNC, …) in one
//!   `MavMessage` enum.
//! * [`dialect_hash`] — `CC_DIALECT_HASH` / `CC_DIALECT_SHA256`, computed by
//!   `build.rs` from the raw XML bytes (same definition as
//!   `cc-dialect/hash.sh`). Carried in `CC_MISSION_CONTEXT.dialect_hash`.
//! * [`identity`] — component-ID and schema-version constants of the
//!   addressing contract (spec §3.3, §3.4).
//! * [`framing`] — an incremental, never-panicking MAVLink 2 frame decoder
//!   with exact fault counters: the foundation `cc-link` (RX path) and
//!   `cc-ingest` build on in Phase 4, and the object of the Phase 1 fuzz
//!   suite.
//! * [`validate`] — envelope validation helpers (source component, schema
//!   version, range checks) shared by future crates and by tests.
//!
//! The Phase 1 exit criteria are enforced by this crate's integration
//! tests: `tests/golden_roundtrip.rs` (C↔Rust wire agreement, the CRC_EXTRA
//! drift detector) and `tests/fuzz_decoder.rs` (parser robustness with
//! exact counter accounting).

// Re-export the MAVLink runtime so downstream crates use exactly this
// version (it is pinned in the workspace manifest; a mismatched duplicate
// would be a wire-contract risk).
pub use mavlink_core;

/// Build-time-generated MAVLink dialect bindings (see `build.rs`).
#[allow(clippy::all)]
pub mod dialects {
    include!(concat!(env!("OUT_DIR"), "/mavlink/mod.rs"));
}

/// Dialect hash constants computed from `cc_dialect.xml` at build time.
pub mod dialect_hash {
    include!(concat!(env!("OUT_DIR"), "/dialect_hash.rs"));
}

pub mod framing;
pub mod identity;
pub mod validate;

/// Convenience alias: the one dialect this system speaks.
pub use dialects::cc_dialect;

/// Convenience alias for the dialect's message enum.
pub type CcMavMessage = dialects::cc_dialect::MavMessage;

/// [`framing::FrameDecoder`] pre-bound to the cc dialect.
pub type CcFrameDecoder = framing::FrameDecoder<CcMavMessage>;
