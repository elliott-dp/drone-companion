//! Golden-vector round-trip test — dev plan Phase 1.3, the CRC_EXTRA drift
//! detector.
//!
//! `cc-dialect/golden/golden_frames.bin` was produced by
//! `cc-dialect/golden/gen_golden.c` using the mavgen-generated **C** encoder
//! (the same encoder family PX4 uses) with fixed, documented field values.
//! This test proves the **Rust** bindings agree with the C bindings on the
//! wire format:
//!
//! 1. every golden frame parses through the reference `read_v2_msg` path
//!    (a failed CRC would surface here — bad CRC means CRC_EXTRA drift);
//! 2. every field of every message equals the fixed golden value
//!    (mirrored literally from `gen_golden.c` — change only together!);
//! 3. re-encoding with the decoded header yields **byte-identical** frames
//!    (proves field ordering, zero-truncation and min-1-byte rules match);
//! 4. the manifest's per-message `crc_extra` equals the Rust table's;
//! 5. `CC_MISSION_CONTEXT.dialect_hash` (embedded by C from `hash.sh`
//!    output) equals the Rust build-time hash — the two hash pipelines
//!    agree end to end;
//! 6. the crate's own `FrameDecoder` produces the identical message
//!    sequence with clean counters.
//!
//! If this test fails after an XML edit, regenerate BOTH bindings and the
//! golden vectors from the same commit (see cc-dialect/README.md); if it
//! fails without an XML edit, someone's toolchain drifted — do NOT "fix"
//! the expected values.

use cc_protocol::cc_dialect::*;
use cc_protocol::mavlink_core::peek_reader::PeekReader;
use cc_protocol::mavlink_core::{read_v2_msg, write_v2_msg, MavHeader, Message};
use cc_protocol::{dialect_hash, identity, CcFrameDecoder};

const GOLDEN_BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../cc-dialect/golden/golden_frames.bin"
));
const GOLDEN_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../cc-dialect/golden/golden_manifest.txt"
));

/// Pinned quiet-NaN bit pattern used by gen_golden.c.
const QNAN_BITS: u32 = 0x7FC0_0000;

#[derive(Debug)]
struct ManifestRow {
    idx: usize,
    offset: usize,
    frame_len: usize,
    msgid: u32,
    hdr_seq: u8,
    sysid: u8,
    compid: u8,
    payload_len: u8,
    crc_extra: u8,
    name: String,
}

fn parse_manifest() -> Vec<ManifestRow> {
    GOLDEN_MANIFEST
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 10, "manifest row wrong arity: {l}");
            ManifestRow {
                idx: f[0].parse().unwrap(),
                offset: f[1].parse().unwrap(),
                frame_len: f[2].parse().unwrap(),
                msgid: f[3].parse().unwrap(),
                hdr_seq: f[4].parse().unwrap(),
                sysid: f[5].parse().unwrap(),
                compid: f[6].parse().unwrap(),
                payload_len: f[7].parse().unwrap(),
                crc_extra: f[8].parse().unwrap(),
                name: f[9].to_string(),
            }
        })
        .collect()
}

fn assert_nan(v: f32, what: &str) {
    assert_eq!(v.to_bits(), QNAN_BITS, "{what}: expected pinned quiet NaN");
}

/// The golden field contract, message by message — mirrored from
/// gen_golden.c. Panics with a field-precise message on any mismatch.
fn assert_golden_fields(idx: usize, name: &str, msg: &MavMessage) {
    match (idx, msg) {
        (0, MavMessage::HEARTBEAT(m)) => {
            assert_eq!(m.custom_mode, 16_909_060); // 0x01020304
            assert_eq!(m.mavtype, MavType::MAV_TYPE_QUADROTOR);
            assert_eq!(m.autopilot, MavAutopilot::MAV_AUTOPILOT_PX4);
            assert_eq!(m.base_mode, MavModeFlag::from_bits(129).unwrap());
            assert_eq!(m.system_status, MavState::MAV_STATE_ACTIVE);
            assert_eq!(m.mavlink_version, 3);
        }
        (1, MavMessage::TIMESYNC(m)) => {
            assert_eq!(m.tc1, 111_222_333_444_555_i64);
            assert_eq!(m.ts1, 999_888_777_666_555_i64);
        }
        (2, MavMessage::CC_TELEMETRY_STATE(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_456);
            assert_eq!(m.sequence, 42_001);
            assert_eq!(m.px4_boot_id, 0xB007_B007);
            assert_eq!(m.mission_id, 777_001);
            assert_eq!(m.failsafe_flags, 0x0001_0203);
            assert_eq!(m.q, [1.0, -0.5, 0.25, -0.125]);
            assert_eq!(m.angular_velocity, [0.125, -0.25, 0.5]);
            assert_eq!(m.position_ned, [10.5, -20.25, -30.125]);
            assert_eq!(m.velocity_ned, [1.5, -2.25, 3.125]);
            assert_eq!(m.heading, 1.5);
            assert_eq!(m.nav_state, 14);
            assert_eq!(m.arming_state, 2);
            assert_eq!(m.vehicle_type, 2);
            assert_eq!(m.estimator_valid, 1);
            assert_eq!(m.control_mode_flags, 0xA5);
            assert_eq!(m.schema_version, identity::CC_SCHEMA_VERSION);
        }
        (3, MavMessage::CC_TELEMETRY_IMU(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_457);
            assert_eq!(m.sequence, 42_002);
            assert_eq!(m.clipping_count, 7);
            assert_eq!(m.accel, [0.5, -9.8125, 1.25]);
            assert_eq!(m.gyro, [0.0625, -0.125, 0.25]);
            assert_eq!(m.delta_angle, [0.03125, -0.0625, 0.125]);
            assert_eq!(m.delta_velocity, [0.25, -0.5, 0.75]);
            assert_eq!(m.vibration_metric, [1.5, 2.5, 3.5]);
            assert_eq!(m.temperature, 45.5);
            assert_eq!(m.schema_version, 1);
        }
        (4, MavMessage::CC_TELEMETRY_POWER(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_458);
            assert_eq!(m.sequence, 42_003);
            assert_eq!(m.voltage, 22.5);
            assert_eq!(m.current, 15.25);
            assert_eq!(m.power, 343.125);
            assert_eq!(m.consumed_mah, 1250.5);
            assert_eq!(m.remaining, 0.75);
            assert_nan(m.temperature, "CC_TELEMETRY_POWER.temperature");
            assert_eq!(m.cell_count, 6);
            assert_eq!(m.warning, 1);
            assert_eq!(m.connected, 1);
            assert_eq!(m.schema_version, 1);
        }
        (5, MavMessage::CC_TELEMETRY_GPS(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_459);
            assert_eq!(m.sequence, 42_004);
            assert_eq!(m.lat, 473_977_420);
            assert_eq!(m.lon, 85_455_940);
            assert_eq!(m.alt, 488_000);
            assert_eq!(m.eph, 1.25);
            assert_eq!(m.epv, 2.5);
            assert_eq!(m.ground_speed, 12.5);
            assert_eq!(m.heading, -1.5);
            assert_eq!(m.noise_per_ms, 55);
            assert_eq!(m.jamming_indicator, 33);
            assert_eq!(m.fix_type, 3);
            assert_eq!(m.satellites_used, 17);
            assert_eq!(m.schema_version, 1);
        }
        (6, MavMessage::CC_TELEMETRY_ESTIMATOR(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_460);
            assert_eq!(m.sequence, 42_005);
            assert_eq!(m.status_flags, 0x00FF_00FF);
            assert_eq!(m.innovation_check_flags, 0x0F0F);
            assert_eq!(m.solution_status_flags, 0x0355);
            assert_eq!(m.velocity_test_ratio, 0.25);
            assert_eq!(m.position_test_ratio, 0.5);
            assert_eq!(m.height_test_ratio, 0.125);
            assert_eq!(m.mag_test_ratio, 0.75);
            assert_nan(m.airspeed_test_ratio, "CC_TELEMETRY_ESTIMATOR.airspeed_test_ratio");
            assert_eq!(m.schema_version, 1);
        }
        (7, MavMessage::CC_TELEMETRY_ACTUATOR(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_461);
            assert_eq!(m.sequence, 42_006);
            assert_eq!(m.actuator_output[0], 0.1015625);
            assert_eq!(m.actuator_output[1], 0.203125);
            assert_eq!(m.actuator_output[2], 0.3046875);
            assert_eq!(m.actuator_output[3], 0.40625);
            for i in 4..8 {
                assert_nan(m.actuator_output[i], "CC_TELEMETRY_ACTUATOR unused slot");
            }
            assert_eq!(m.motor_count, 4);
            assert_eq!(m.schema_version, 1);
        }
        (8, MavMessage::CC_EVENT(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_462);
            assert_eq!(m.sequence, 42_007);
            assert_eq!(m.event_id, 0x00CC_0001);
            assert_eq!(m.argument0, 111);
            assert_eq!(m.argument1, 222);
            assert_eq!(m.severity, CcSeverity::CC_SEVERITY_WARN);
            assert_eq!(m.subsystem, CcSubsystem::CC_SUBSYS_VIBRATION);
            assert_eq!(m.schema_version, 1);
        }
        (9, MavMessage::CC_SAFETY_STATUS(m)) => {
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_463);
            assert_eq!(m.last_report_sequence, 88_005);
            assert_eq!(
                m.active_health_flags,
                CcHealthFlags::CC_HF_BATTERY
                    | CcHealthFlags::CC_HF_VIBRATION
                    | CcHealthFlags::CC_HF_TIMESYNC
            );
            assert_eq!(m.report_age_ms, 150);
            assert_eq!(m.missed_reports, 3);
            assert_eq!(m.companion_state, CcCompanionState::CC_STATE_WARN);
            assert_eq!(m.action_taken, CcRecommendedAction::CC_ACTION_WARN_ONLY);
            assert_eq!(m.reject_reason, CcRejectReason::CC_REJECT_NONE);
            assert_eq!(m.schema_version, 1);
        }
        (10, MavMessage::CC_HEALTH_REPORT(m)) => {
            assert_eq!(m.companion_timestamp_us, 987_654_321_098_765);
            assert_eq!(m.sequence, 88_005);
            assert_eq!(m.mission_id, 777_001);
            assert_eq!(m.companion_boot_id, 0xCC0C_C001);
            assert_eq!(
                m.health_flags,
                CcHealthFlags::CC_HF_BATTERY
                    | CcHealthFlags::CC_HF_VIBRATION
                    | CcHealthFlags::CC_HF_TIMESYNC
            );
            assert_eq!(m.detail_code, 0x0BAD);
            assert_eq!(m.link_rtt_ms, 12);
            assert_eq!(m.telemetry_age_ms, 40);
            assert_eq!(m.companion_loop_ms, 5);
            assert_eq!(m.dropped_rx_count, 2);
            assert_eq!(m.severity, CcSeverity::CC_SEVERITY_WARN);
            assert_eq!(m.recommended_action, CcRecommendedAction::CC_ACTION_WARN_ONLY);
            assert_eq!(m.confidence_percent, 87);
            assert_eq!(m.schema_version, 1);
        }
        (11, MavMessage::CC_AI_DIAGNOSTIC(m)) => {
            assert_eq!(m.companion_timestamp_us, 987_654_321_098_766);
            assert_eq!(m.sequence, 91_001);
            assert_eq!(m.value, 3.375);
            assert_eq!(m.limit, 2.5);
            assert_eq!(m.detail_code, 0x0BAD);
            assert_eq!(m.subsystem, CcSubsystem::CC_SUBSYS_BATTERY);
            assert_eq!(m.severity, CcSeverity::CC_SEVERITY_WARN);
            assert_eq!(m.confidence_percent, 87);
            assert_eq!(m.schema_version, 1);
        }
        (12, MavMessage::CC_MISSION_CONTEXT(m)) => {
            assert_eq!(m.mission_id, 777_001);
            assert_eq!(m.cc_boot_id, 0xCC0C_C001);
            assert_eq!(m.vehicle_id, 4242);
            // End-to-end hash agreement: C side embedded hash.sh's value,
            // Rust side computed its own in build.rs.
            assert_eq!(
                m.dialect_hash,
                dialect_hash::CC_DIALECT_HASH,
                "dialect_hash mismatch: hash.sh (C) vs build.rs (Rust)"
            );
            assert_eq!(m.sw_version.to_str().unwrap(), "cc-golden-0.1.0");
            assert_eq!(m.schema_version, 1);
        }
        (13, MavMessage::CC_LOG_CONTROL(m)) => {
            assert_eq!(m.companion_timestamp_us, 987_654_321_098_767);
            assert_eq!(m.sequence, 91_002);
            assert_eq!(m.requested_profile, CcLogProfile::CC_PROFILE_DEBUG);
            assert_eq!(m.schema_version, 1);
        }
        (14, MavMessage::CC_EVENT(m)) => {
            // zero-truncation probe: trailing zeros were cut on the wire and
            // must have been zero-filled back on decode
            assert_eq!(m.fc_timestamp_us, 1_234_567_890_123_999);
            assert_eq!(m.sequence, 42_999);
            assert_eq!(m.event_id, 0x00CC_0002);
            assert_eq!(m.argument0, 222);
            assert_eq!(m.argument1, 0);
            assert_eq!(m.severity, CcSeverity::CC_SEVERITY_OK);
            assert_eq!(m.subsystem, CcSubsystem::CC_SUBSYS_NONE);
            assert_eq!(m.schema_version, 0);
        }
        (15, MavMessage::TIMESYNC(m)) => {
            // min-payload probe: all-zero payload truncated to 1 byte
            assert_eq!(m.tc1, 0);
            assert_eq!(m.ts1, 0);
        }
        _ => panic!("frame {idx} ({name}): unexpected message variant {msg:?}"),
    }
}

#[test]
fn golden_roundtrip() {
    let manifest = parse_manifest();
    assert_eq!(manifest.len(), 16, "golden set must hold 16 frames");
    assert_eq!(
        manifest.iter().map(|r| r.frame_len).sum::<usize>(),
        GOLDEN_BIN.len(),
        "manifest frame lengths must cover the whole file"
    );

    let mut reader = PeekReader::new(GOLDEN_BIN);
    let mut running_offset = 0usize;

    for row in &manifest {
        assert_eq!(row.offset, running_offset, "manifest offset drift at {}", row.idx);

        // 1. reference decode (CRC enforced inside read_v2_msg)
        let (header, msg): (MavHeader, MavMessage) = read_v2_msg(&mut reader)
            .unwrap_or_else(|e| panic!("frame {} ({}) failed to decode: {e:?}", row.idx, row.name));

        // header contract
        assert_eq!(header.system_id, row.sysid, "frame {} sysid", row.idx);
        assert_eq!(header.component_id, row.compid, "frame {} compid", row.idx);
        assert_eq!(header.sequence, row.hdr_seq, "frame {} header seq", row.idx);
        assert_eq!(msg.message_id(), row.msgid, "frame {} msgid", row.idx);
        // manifest names may carry a probe suffix (e.g. CC_EVENT_TRUNCATION_PROBE)
        assert!(
            row.name.starts_with(msg.message_name()),
            "frame {} name: manifest {} vs wire {}",
            row.idx,
            row.name,
            msg.message_name()
        );
        // all golden frames are unsigned: frame = STX+9 header + payload + 2 CRC
        assert_eq!(
            row.frame_len,
            12 + row.payload_len as usize,
            "frame {} length arithmetic",
            row.idx
        );

        // 4. explicit CRC_EXTRA table agreement (C manifest vs Rust bindings)
        assert_eq!(
            MavMessage::extra_crc(row.msgid),
            row.crc_extra,
            "CRC_EXTRA drift on {} — regenerate both bindings from the same XML commit!",
            row.name
        );

        // 2. every field equals the golden value
        assert_golden_fields(row.idx, &row.name, &msg);

        // 3. byte-identical re-encode
        let mut reencoded = Vec::with_capacity(row.frame_len);
        write_v2_msg(&mut reencoded, header, &msg)
            .unwrap_or_else(|e| panic!("frame {} re-encode failed: {e:?}", row.idx));
        let original = &GOLDEN_BIN[row.offset..row.offset + row.frame_len];
        assert_eq!(
            reencoded, original,
            "frame {} ({}) re-encode is not byte-identical",
            row.idx, row.name
        );

        running_offset += row.frame_len;
    }

    // nothing may remain
    assert!(
        read_v2_msg::<MavMessage, _>(&mut reader).is_err(),
        "unexpected trailing frame after the golden set"
    );
}

#[test]
fn golden_frames_through_own_decoder() {
    // 6. the crate's FrameDecoder must agree with the reference path and
    // account every byte with clean counters.
    let mut reference = Vec::new();
    let mut reader = PeekReader::new(GOLDEN_BIN);
    while let Ok((h, m)) = read_v2_msg::<MavMessage, _>(&mut reader) {
        reference.push((h, m));
    }
    assert_eq!(reference.len(), 16);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(GOLDEN_BIN);
    assert_eq!(frames.len(), 16);

    for (i, (frame, (ref_h, ref_m))) in frames.iter().zip(&reference).enumerate() {
        // Compare via re-serialization, not PartialEq: three golden frames
        // deliberately carry NaN fields and NaN != NaN under float equality,
        // while the wire bytes ARE the contract (NaN bits preserved).
        let mut a = Vec::new();
        let mut b = Vec::new();
        write_v2_msg(&mut a, frame.header, &frame.message).unwrap();
        write_v2_msg(&mut b, *ref_h, ref_m).unwrap();
        assert_eq!(a, b, "frame {i} bytes mismatch between decoders");
        assert_eq!(frame.msg_id, ref_m.message_id(), "frame {i} msgid");
        assert_eq!(frame.header.system_id, ref_h.system_id);
        assert_eq!(frame.header.component_id, ref_h.component_id);
        assert_eq!(frame.header.sequence, ref_h.sequence);
        assert!(!frame.signed);
    }

    let c = dec.counters();
    assert_eq!(c.frames_ok, 16);
    assert_eq!(c.frames_ok_bytes, GOLDEN_BIN.len() as u64);
    assert_eq!(c.crc_errors, 0);
    assert_eq!(c.unknown_msg_ids, 0);
    assert_eq!(c.bad_payloads, 0);
    assert_eq!(c.bad_incompat_flags, 0);
    assert_eq!(c.garbage_bytes, 0);
    assert_eq!(dec.pending(), 0);
}

#[test]
fn golden_manifest_dialect_hash_matches_build() {
    // The manifest header records hash.sh's SHA-256; build.rs computed its
    // own from the same file. They must agree even before any frame parses.
    let sha_line = GOLDEN_MANIFEST
        .lines()
        .find(|l| l.contains("dialect sha256"))
        .expect("manifest lost its dialect sha256 header");
    assert!(
        sha_line.contains(dialect_hash::CC_DIALECT_SHA256),
        "manifest sha256 differs from build.rs sha256 — regenerate golden vectors"
    );
}
