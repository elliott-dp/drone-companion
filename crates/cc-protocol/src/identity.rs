//! Addressing & envelope identity constants (spec §3.3, §3.4).
//!
//! FC and CC share one MAVLink *system* (they are one vehicle) and are told
//! apart by *component* ID. PX4's receiver discards CC_* command/health
//! messages from any component other than [`COMPID_CC`]; symmetrically, the
//! CC ingest path only trusts FC-originated telemetry from [`COMPID_FC`].

/// Payload schema version currently produced by this codebase.
/// Bumped on any field-semantics change of any CC_* message (spec §3.2:
/// additions go in extensions; semantic changes bump this).
pub const CC_SCHEMA_VERSION: u8 = 1;

/// Default MAVLink system ID for the vehicle (both ends; deployment may
/// override via configuration, but they must match).
pub const SYSID_VEHICLE_DEFAULT: u8 = 1;

/// PX4 autopilot component: `MAV_COMP_ID_AUTOPILOT1`.
pub const COMPID_FC: u8 = 1;

/// Jetson companion component: `MAV_COMP_ID_ONBOARD_COMPUTER`.
pub const COMPID_CC: u8 = 191;
