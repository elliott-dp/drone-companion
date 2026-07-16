//! Envelope validation helpers (spec §3.3 addressing, §3.4 envelope,
//! §4.4 range rules as they apply on the companion side).
//!
//! Scope note — what belongs where:
//! * **Here (cc-protocol):** message direction, source-component and
//!   schema-version checks, plus value-range checks that the type system
//!   cannot express (`confidence_percent ≤ 100`). Pure functions, no state.
//! * **Not here:** sequence continuity, staleness, flood limiting — those
//!   are stateful link/ingest concerns (`cc-link`/`cc-ingest`, Phase 4) and,
//!   on the FC, `mavlink_receiver.cpp`'s validation gauntlet (Phase 3).
//!
//! Enum/bitmask fields need no range checks after decode: the generated
//! `parse` already rejects out-of-range discriminants (the framing layer
//! counts those as `bad_payloads`). The checks here therefore matter mostly
//! for *outbound* construction and for defense in depth.

use mavlink_core::MavHeader;

use crate::dialects::cc_dialect::MavMessage;
use crate::identity::{CC_SCHEMA_VERSION, COMPID_FC};

/// Who is allowed to originate a message (spec §3.2 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// FC → CC telemetry/status (CC_TELEMETRY_*, CC_EVENT, CC_SAFETY_STATUS).
    FcToCc,
    /// CC → FC companion-originated (CC_HEALTH_REPORT, CC_AI_DIAGNOSTIC,
    /// CC_MISSION_CONTEXT, CC_LOG_CONTROL).
    CcToFc,
    /// Standard messages both ends emit (HEARTBEAT, TIMESYNC, …).
    Common,
}

/// Direction of a CC dialect message by wire ID; `None` for standard
/// messages that are not part of the private 54000-block contract.
pub fn direction_of_id(msg_id: u32) -> Option<Direction> {
    match msg_id {
        // 54000-54007 defined today, 54008 reserved for CC_TELEMETRY_ESC —
        // all FC-originated per the spec §3.2 allocation table.
        54000..=54008 => Some(Direction::FcToCc),
        54010..=54013 => Some(Direction::CcToFc),
        // Unallocated IDs (54009, 54014+) get no direction until the spec
        // table assigns one; extend this match in the same commit that
        // extends cc_dialect.xml.
        _ => None,
    }
}

/// Envelope validation failure. Mirrors the FC-side reject taxonomy
/// (`CC_REJECT_*`, spec §4.4) where the concepts overlap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// Message came from the wrong system or component for its direction.
    BadSource {
        msg_id: u32,
        system_id: u8,
        component_id: u8,
    },
    /// schema_version does not match what this build speaks.
    BadSchema { msg_id: u32, got: u8 },
    /// A numeric field is outside its documented range.
    BadRange {
        msg_id: u32,
        field: &'static str,
        value: u32,
        max: u32,
    },
}

/// The `schema_version` envelope field carried by every CC_* payload
/// (spec §3.4); `None` for standard messages, which do not carry one.
pub fn schema_version_of(msg: &MavMessage) -> Option<u8> {
    use MavMessage::*;
    match msg {
        CC_TELEMETRY_STATE(m) => Some(m.schema_version),
        CC_TELEMETRY_IMU(m) => Some(m.schema_version),
        CC_TELEMETRY_POWER(m) => Some(m.schema_version),
        CC_TELEMETRY_GPS(m) => Some(m.schema_version),
        CC_TELEMETRY_ESTIMATOR(m) => Some(m.schema_version),
        CC_TELEMETRY_ACTUATOR(m) => Some(m.schema_version),
        CC_EVENT(m) => Some(m.schema_version),
        CC_SAFETY_STATUS(m) => Some(m.schema_version),
        CC_HEALTH_REPORT(m) => Some(m.schema_version),
        CC_AI_DIAGNOSTIC(m) => Some(m.schema_version),
        CC_MISSION_CONTEXT(m) => Some(m.schema_version),
        CC_LOG_CONTROL(m) => Some(m.schema_version),
        _ => None,
    }
}

/// Check the schema_version of a CC_* message against this build.
/// Standard messages (no schema field) pass.
pub fn validate_schema(msg_id: u32, msg: &MavMessage) -> Result<(), ValidateError> {
    match schema_version_of(msg) {
        Some(v) if v != CC_SCHEMA_VERSION => Err(ValidateError::BadSchema { msg_id, got: v }),
        _ => Ok(()),
    }
}

/// Range checks the type system cannot express (spec §4.4 step 4, the part
/// that applies to fields rather than enums).
pub fn validate_ranges(msg_id: u32, msg: &MavMessage) -> Result<(), ValidateError> {
    let confidence = match msg {
        MavMessage::CC_HEALTH_REPORT(m) => Some(m.confidence_percent),
        MavMessage::CC_AI_DIAGNOSTIC(m) => Some(m.confidence_percent),
        _ => None,
    };
    match confidence {
        Some(c) if c > 100 => Err(ValidateError::BadRange {
            msg_id,
            field: "confidence_percent",
            value: u32::from(c),
            max: 100,
        }),
        _ => Ok(()),
    }
}

/// Source check for the **CC ingest direction**: FC-originated dialect
/// messages must come from `(expected_sysid, COMPID_FC)`; CC-originated
/// dialect messages looping back to us are a wiring/config error.
/// Standard messages are only checked for the system ID.
pub fn validate_source_on_cc(
    header: &MavHeader,
    expected_sysid: u8,
    msg_id: u32,
) -> Result<(), ValidateError> {
    let bad = || ValidateError::BadSource {
        msg_id,
        system_id: header.system_id,
        component_id: header.component_id,
    };
    if header.system_id != expected_sysid {
        return Err(bad());
    }
    match direction_of_id(msg_id) {
        Some(Direction::FcToCc) if header.component_id != COMPID_FC => Err(bad()),
        Some(Direction::CcToFc) => Err(bad()), // our own class of message coming *at* us
        _ => Ok(()),
    }
}

/// Composed inbound validation for the companion RX path: source, schema,
/// ranges — in the same order the FC-side gauntlet documents (spec §4.4),
/// so counters and reject reasons stay comparable across the two sides.
pub fn validate_inbound_on_cc(
    header: &MavHeader,
    expected_sysid: u8,
    msg_id: u32,
    msg: &MavMessage,
) -> Result<(), ValidateError> {
    validate_source_on_cc(header, expected_sysid, msg_id)?;
    validate_schema(msg_id, msg)?;
    validate_ranges(msg_id, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::cc_dialect::*;
    use crate::identity::COMPID_CC;

    fn hdr(sys: u8, comp: u8) -> MavHeader {
        MavHeader {
            system_id: sys,
            component_id: comp,
            sequence: 0,
        }
    }

    fn health_report(confidence: u8, schema: u8) -> MavMessage {
        MavMessage::CC_HEALTH_REPORT(CC_HEALTH_REPORT_DATA {
            companion_timestamp_us: 1,
            sequence: 1,
            mission_id: 1,
            companion_boot_id: 1,
            health_flags: CcHealthFlags::CC_HF_BATTERY,
            detail_code: 1,
            link_rtt_ms: 1,
            telemetry_age_ms: 1,
            companion_loop_ms: 1,
            dropped_rx_count: 0,
            severity: CcSeverity::CC_SEVERITY_WARN,
            recommended_action: CcRecommendedAction::CC_ACTION_WARN_ONLY,
            confidence_percent: confidence,
            schema_version: schema,
        })
    }

    fn state_msg(schema: u8) -> MavMessage {
        MavMessage::CC_TELEMETRY_STATE(CC_TELEMETRY_STATE_DATA {
            schema_version: schema,
            ..Default::default()
        })
    }

    #[test]
    fn direction_table_matches_id_allocation() {
        assert_eq!(direction_of_id(54000), Some(Direction::FcToCc));
        assert_eq!(direction_of_id(54007), Some(Direction::FcToCc));
        assert_eq!(direction_of_id(54008), Some(Direction::FcToCc)); // reserved CC_TELEMETRY_ESC
        assert_eq!(direction_of_id(54009), None); // unallocated
        assert_eq!(direction_of_id(54010), Some(Direction::CcToFc));
        assert_eq!(direction_of_id(54013), Some(Direction::CcToFc));
        assert_eq!(direction_of_id(54014), None); // unallocated
        assert_eq!(direction_of_id(0), None); // HEARTBEAT
        assert_eq!(direction_of_id(111), None); // TIMESYNC
    }

    #[test]
    fn fc_telemetry_from_fc_passes() {
        let msg = state_msg(CC_SCHEMA_VERSION);
        assert!(validate_inbound_on_cc(&hdr(1, COMPID_FC), 1, 54000, &msg).is_ok());
    }

    #[test]
    fn fc_telemetry_from_wrong_component_rejected() {
        let msg = state_msg(CC_SCHEMA_VERSION);
        let err = validate_inbound_on_cc(&hdr(1, 42), 1, 54000, &msg).unwrap_err();
        assert!(matches!(err, ValidateError::BadSource { component_id: 42, .. }));
    }

    #[test]
    fn wrong_sysid_rejected() {
        let msg = state_msg(CC_SCHEMA_VERSION);
        let err = validate_inbound_on_cc(&hdr(2, COMPID_FC), 1, 54000, &msg).unwrap_err();
        assert!(matches!(err, ValidateError::BadSource { system_id: 2, .. }));
    }

    #[test]
    fn own_message_class_looping_back_rejected() {
        let msg = health_report(50, CC_SCHEMA_VERSION);
        let err = validate_inbound_on_cc(&hdr(1, COMPID_CC), 1, 54010, &msg).unwrap_err();
        assert!(matches!(err, ValidateError::BadSource { .. }));
    }

    #[test]
    fn schema_mismatch_rejected() {
        let msg = state_msg(CC_SCHEMA_VERSION + 1);
        let err = validate_inbound_on_cc(&hdr(1, COMPID_FC), 1, 54000, &msg).unwrap_err();
        assert!(matches!(err, ValidateError::BadSchema { got, .. } if got == CC_SCHEMA_VERSION + 1));
    }

    #[test]
    fn confidence_over_100_rejected() {
        let msg = health_report(101, CC_SCHEMA_VERSION);
        let err = validate_ranges(54010, &msg).unwrap_err();
        assert!(matches!(
            err,
            ValidateError::BadRange { field: "confidence_percent", value: 101, max: 100, .. }
        ));
        assert!(validate_ranges(54010, &health_report(100, CC_SCHEMA_VERSION)).is_ok());
    }

    #[test]
    fn standard_messages_pass_schema_and_ranges() {
        let hb = MavMessage::HEARTBEAT(HEARTBEAT_DATA::default());
        assert_eq!(schema_version_of(&hb), None);
        assert!(validate_schema(0, &hb).is_ok());
        assert!(validate_ranges(0, &hb).is_ok());
    }
}
