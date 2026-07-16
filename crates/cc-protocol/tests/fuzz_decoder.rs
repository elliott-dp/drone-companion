//! Fuzz / property suite for the frame decoder — dev plan Phase 1.4.
//!
//! Requirements under test (spec §14.2, §5.3, §2.4):
//! * the parser must **never panic**, whatever bytes arrive (truncated
//!   frames, corrupted CRCs, random garbage, giant sequence values, chunked
//!   delivery, hot-plug mid-frame);
//! * it must **resynchronize** on the next STX after any fault;
//! * its **counters must match the injected faults exactly** — not "roughly":
//!   every test below asserts exact counter values wherever the fault model
//!   is deterministic, and the byte-conservation invariant everywhere:
//!
//!   ```text
//!   bytes_in == frames_ok_bytes + unknown_msg_bytes + bad_payload_bytes
//!               + garbage_bytes + pending()
//!   ```
//!
//! All randomness is a seeded xorshift64* PRNG: failures reproduce exactly,
//! in CI and locally, with no external fuzzing dependency. (A coverage-guided
//! cargo-fuzz target can be layered on later without changing this suite.)

use cc_protocol::cc_dialect::*;
use cc_protocol::framing::{DecodeCounters, IFLAG_SIGNED, STX_V2, V2_SIGNATURE_LEN};
use cc_protocol::identity;
use cc_protocol::mavlink_core::{write_v2_msg, MAVLinkV2MessageRaw, MavHeader};
use cc_protocol::CcFrameDecoder;

// --------------------------------------------------------------------------
// deterministic PRNG (xorshift64*)
// --------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// uniform in [0, n)
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// --------------------------------------------------------------------------
// helpers
// --------------------------------------------------------------------------

fn header(seq: u8) -> MavHeader {
    MavHeader {
        system_id: identity::SYSID_VEHICLE_DEFAULT,
        component_id: identity::COMPID_CC,
        sequence: seq,
    }
}

/// A benign CC_HEALTH_REPORT for stream-building. NOTE: not every message
/// can appear in the FD-free exact-counter tests — CC_LOG_CONTROL's own
/// message ID is 54013 = 0xD2FD, so its header always contains an 0xFD!
/// CC_HEALTH_REPORT (54010 → FA D2 00) is safe.
fn base_msg(sequence: u32, salt: u64) -> MavMessage {
    MavMessage::CC_HEALTH_REPORT(CC_HEALTH_REPORT_DATA {
        companion_timestamp_us: 0x0102_0304_0500_0000 + salt,
        sequence,
        mission_id: 777_001,
        companion_boot_id: 0x0C0C_C001,
        health_flags: CcHealthFlags::empty(),
        detail_code: 0x0B0B,
        link_rtt_ms: 12,
        telemetry_age_ms: 34,
        companion_loop_ms: 5,
        dropped_rx_count: 0,
        severity: CcSeverity::CC_SEVERITY_OK,
        recommended_action: CcRecommendedAction::CC_ACTION_NONE,
        confidence_percent: 90,
        schema_version: identity::CC_SCHEMA_VERSION,
    })
}

fn fd_free_msg(sequence: u32) -> MavMessage {
    base_msg(sequence, 0x60_0D00)
}

fn encode(msg: &MavMessage, seq: u8) -> Vec<u8> {
    let mut v = Vec::new();
    write_v2_msg(&mut v, header(seq), msg).unwrap();
    v
}

/// Encode a frame guaranteed to contain no 0xFD after the STX, so garbage
/// accounting in the exact-counter tests is fully deterministic. The field
/// values are fixed except a timestamp salt, searched deterministically
/// until the CRC bytes (the only content we cannot choose) are FD-free.
fn encode_fd_free(sequence: u32, seq: u8) -> Vec<u8> {
    for salt in 0..=255u64 {
        let bytes = encode(&base_msg(sequence, salt), seq);
        if bytes.iter().skip(1).all(|&b| b != STX_V2) {
            return bytes;
        }
    }
    unreachable!("no FD-free encoding in 256 salts — statistically impossible");
}

fn assert_conservation(dec: &CcFrameDecoder) {
    let c = dec.counters();
    assert_eq!(
        c.bytes_in,
        c.accounted_bytes() + dec.pending() as u64,
        "byte conservation broken: {c:?}, pending={}",
        dec.pending()
    );
}

/// Garbage run of given length guaranteed to contain no STX byte.
fn fd_free_garbage(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| loop {
            let b = rng.byte();
            if b != STX_V2 {
                break b;
            }
        })
        .collect()
}

// --------------------------------------------------------------------------
// never-panic properties
// --------------------------------------------------------------------------

#[test]
fn never_panics_on_pure_random_streams() {
    for seed in 1..=64u64 {
        let mut rng = Rng::new(seed);
        let mut dec = CcFrameDecoder::new();
        let total = 16 * 1024;
        let mut fed = 0usize;
        while fed < total {
            let chunk_len = 1 + rng.below(509);
            let chunk: Vec<u8> = (0..chunk_len).map(|_| rng.byte()).collect();
            let _ = dec.push(&chunk); // must not panic; frames extremely unlikely
            fed += chunk_len;
            assert_conservation(&dec);
        }
        assert_eq!(dec.counters().bytes_in, fed as u64);
    }
}

#[test]
fn never_panics_on_mutated_valid_streams() {
    // A valid stream with random single-byte mutations: the nastiest input
    // class, because most bytes still look like real protocol.
    let mut clean = Vec::new();
    for i in 0..40u8 {
        clean.extend_from_slice(&encode(&fd_free_msg(u32::from(i)), i));
    }
    for seed in 1..=64u64 {
        let mut rng = Rng::new(0xBAD0_0000 + seed);
        let mut stream = clean.clone();
        for _ in 0..8 {
            let pos = rng.below(stream.len());
            stream[pos] ^= 1 << rng.below(8);
        }
        let mut dec = CcFrameDecoder::new();
        // deliver in random chunks
        let mut off = 0;
        while off < stream.len() {
            let n = (1 + rng.below(97)).min(stream.len() - off);
            let _ = dec.push(&stream[off..off + n]);
            off += n;
            assert_conservation(&dec);
        }
        let c = dec.counters();
        assert!(c.frames_ok <= 40, "cannot decode more frames than were sent");
    }
}

// --------------------------------------------------------------------------
// exact-counter fault injection
// --------------------------------------------------------------------------

#[test]
fn corrupted_crc_costs_exactly_one_error_and_the_frame() {
    // stream: F0 F1c F2   (F1c = payload byte corrupted, CRC now wrong)
    let f0 = encode_fd_free(100, 0);
    let mut f1 = encode_fd_free(101, 1);
    let f2 = encode_fd_free(102, 2);

    f1[10] ^= 0x01; // first payload byte
    assert_ne!(f1[10], STX_V2);

    let mut stream = Vec::new();
    stream.extend_from_slice(&f0);
    stream.extend_from_slice(&f1);
    stream.extend_from_slice(&f2);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].header.sequence, 0);
    assert_eq!(frames[1].header.sequence, 2);

    let c = dec.counters();
    assert_eq!(c.frames_ok, 2);
    assert_eq!(c.crc_errors, 1, "exactly the injected CRC fault");
    // the corrupted frame's bytes all drain into garbage during resync
    assert_eq!(c.garbage_bytes, f1.len() as u64);
    assert_eq!(c.frames_ok_bytes, (f0.len() + f2.len()) as u64);
    assert_eq!(c.unknown_msg_ids, 0);
    assert_eq!(c.bad_payloads, 0);
    assert_eq!(dec.pending(), 0);
    assert_conservation(&dec);
}

#[test]
fn interleaved_garbage_counted_byte_exact() {
    let mut rng = Rng::new(0x6A5B_1E5C);
    let g1 = fd_free_garbage(&mut rng, 17);
    let g2 = fd_free_garbage(&mut rng, 251);
    let g3 = fd_free_garbage(&mut rng, 3);
    let f1 = encode_fd_free(200, 0);
    let f2 = encode_fd_free(201, 1);

    let mut stream = Vec::new();
    stream.extend_from_slice(&g1);
    stream.extend_from_slice(&f1);
    stream.extend_from_slice(&g2);
    stream.extend_from_slice(&f2);
    stream.extend_from_slice(&g3);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 2);
    let c = dec.counters();
    assert_eq!(c.frames_ok, 2);
    assert_eq!(c.garbage_bytes, (g1.len() + g2.len() + g3.len()) as u64);
    assert_eq!(c.crc_errors, 0);
    assert_eq!(dec.pending(), 0);
    assert_conservation(&dec);
}

#[test]
fn truncated_frame_all_cut_points_then_recovery() {
    // For EVERY possible truncation point of a valid frame: feed the
    // truncated prefix, then a burst of garbage-free silence (nothing), then
    // a full valid frame. The decoder must (a) never panic, (b) never emit a
    // frame from the truncated prefix, (c) still decode the follow-up frame.
    let whole = encode_fd_free(300, 7);
    let follow = encode_fd_free(301, 8);

    for cut in 1..whole.len() {
        let mut dec = CcFrameDecoder::new();
        let got = dec.push(&whole[..cut]);
        assert!(got.is_empty(), "cut={cut}: truncated prefix must not decode");
        assert_conservation(&dec);

        let frames = dec.push(&follow);
        // The truncated prefix + follow-up may combine into one candidate
        // whose CRC fails; resync must still recover the follow-up frame.
        assert_eq!(frames.len(), 1, "cut={cut}: follow-up frame lost");
        assert_eq!(frames[0].header.sequence, 8, "cut={cut}");
        assert_eq!(dec.counters().frames_ok, 1);
        assert_conservation(&dec);
    }
}

#[test]
fn hotplug_mid_stream_resyncs() {
    // Spec §2.4: hot-plug = we start listening mid-frame. Simulate by
    // starting the decoder at every offset inside an ongoing stream.
    let mut stream = Vec::new();
    for i in 0..6u8 {
        stream.extend_from_slice(&encode_fd_free(400 + u32::from(i), i));
    }
    let frame_len = stream.len() / 6;

    for start in 0..frame_len {
        let mut dec = CcFrameDecoder::new();
        let frames = dec.push(&stream[start..]);
        let expect = if start == 0 { 6 } else { 5 }; // partial first frame lost
        assert_eq!(frames.len(), expect, "start offset {start}");
        assert_conservation(&dec);
    }
}

// --------------------------------------------------------------------------
// unknown / foreign / malformed frame classes
// --------------------------------------------------------------------------

/// Hand-build a well-formed frame with a message ID outside the dialect
/// (54008 — reserved for CC_TELEMETRY_ESC, deliberately undefined today).
fn unknown_id_frame(payload_len: u8) -> Vec<u8> {
    let mut f = vec![
        STX_V2,
        payload_len,
        0x00, // incompat
        0x00, // compat
        0x2A, // seq
        0x01, // sysid
        0x01, // compid
    ];
    // 54008 = 0x00D2F8, little-endian on the wire
    f.extend_from_slice(&[0xF8, 0xD2, 0x00]);
    for i in 0..payload_len {
        f.push(0x10 + (i % 0xC0)); // FD-free payload
    }
    f.extend_from_slice(&[0x11, 0x22]); // CRC bytes (never checked: no CRC_EXTRA)
    assert!(f.iter().skip(1).all(|&b| b != STX_V2));
    f
}

#[test]
fn unknown_msgid_counted_and_skipped_between_frames() {
    let f1 = encode_fd_free(500, 0);
    let unk = unknown_id_frame(21);
    let f2 = encode_fd_free(501, 1);

    let mut stream = Vec::new();
    stream.extend_from_slice(&f1);
    stream.extend_from_slice(&unk);
    stream.extend_from_slice(&f2);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 2, "both known frames must survive");
    let c = dec.counters();
    assert_eq!(c.frames_ok, 2);
    assert_eq!(c.unknown_msg_ids, 1, "exactly the injected foreign frame");
    assert_eq!(c.unknown_msg_bytes, unk.len() as u64);
    assert_eq!(c.crc_errors, 0, "unknown id must NOT count as corruption");
    assert_eq!(c.garbage_bytes, 0);
    assert_conservation(&dec);
}

#[test]
fn unknown_msgid_at_stream_end_counted() {
    // boundary case: the unknown frame ends exactly at the buffer end
    let f1 = encode_fd_free(510, 0);
    let unk = unknown_id_frame(4);

    let mut dec = CcFrameDecoder::new();
    let mut stream = f1.clone();
    stream.extend_from_slice(&unk);
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 1);
    let c = dec.counters();
    assert_eq!(c.unknown_msg_ids, 1);
    assert_eq!(c.unknown_msg_bytes, unk.len() as u64);
    assert_eq!(dec.pending(), 0);
    assert_conservation(&dec);
}

#[test]
fn unknown_incompat_flags_dropped_exactly_once() {
    let f1 = encode_fd_free(520, 0);
    let mut bad = encode_fd_free(521, 1);
    bad[2] = 0x02; // unknown incompat bit (only 0x01 is defined)
    let f2 = encode_fd_free(522, 2);

    let mut stream = Vec::new();
    stream.extend_from_slice(&f1);
    stream.extend_from_slice(&bad);
    stream.extend_from_slice(&f2);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 2);
    let c = dec.counters();
    assert_eq!(c.bad_incompat_flags, 1);
    assert_eq!(c.frames_ok, 2);
    // the flagged frame's bytes all drain into garbage during resync
    assert_eq!(c.garbage_bytes, bad.len() as u64);
    assert_conservation(&dec);
}

#[test]
fn signed_frame_length_handled_and_flagged() {
    // The wired link does not use signing (spec §8), but a signed frame must
    // still be framed correctly (13-byte signature) and surfaced, never
    // desynchronize the stream.
    let msg = fd_free_msg(530);
    let mut raw = MAVLinkV2MessageRaw::new();
    raw.serialize_message_for_signing(header(9), &msg);
    let signed_bytes = raw.raw_bytes().to_vec();

    let follow = encode_fd_free(531, 10);
    let mut stream = signed_bytes.clone();
    stream.extend_from_slice(&follow);

    let mut dec = CcFrameDecoder::new();
    let frames = dec.push(&stream);

    assert_eq!(frames.len(), 2);
    assert!(frames[0].signed);
    assert_eq!(frames[0].frame_len, signed_bytes.len());
    assert_eq!(
        frames[0].frame_len,
        12 + frames[0].payload_len as usize + V2_SIGNATURE_LEN
    );
    assert_eq!(signed_bytes[2] & IFLAG_SIGNED, IFLAG_SIGNED);
    assert!(!frames[1].signed);
    assert_conservation(&dec);
}

// --------------------------------------------------------------------------
// volume / extreme values / chunking
// --------------------------------------------------------------------------

#[test]
fn five_thousand_frames_with_giant_sequences_in_7_byte_chunks() {
    // "giant sequences": u32 sequence values around the wrap point must
    // transport verbatim (wrap SEMANTICS belong to cc-ingest, Phase 4).
    let n = 5000u32;
    let mut stream = Vec::new();
    let mut expected_seq = Vec::new();
    for i in 0..n {
        let seq32 = (u32::MAX - 2500).wrapping_add(i); // crosses u32::MAX -> wraps
        let msg = MavMessage::CC_HEALTH_REPORT(CC_HEALTH_REPORT_DATA {
            companion_timestamp_us: u64::from(i) * 1000,
            sequence: seq32,
            mission_id: 777_001,
            companion_boot_id: 0xCC0C_C001,
            health_flags: CcHealthFlags::empty(),
            detail_code: 0,
            link_rtt_ms: 1,
            telemetry_age_ms: 2,
            companion_loop_ms: 3,
            dropped_rx_count: 0,
            severity: CcSeverity::CC_SEVERITY_OK,
            recommended_action: CcRecommendedAction::CC_ACTION_NONE,
            confidence_percent: 100,
            schema_version: identity::CC_SCHEMA_VERSION,
        });
        expected_seq.push(seq32);
        stream.extend_from_slice(&encode(&msg, (i % 256) as u8));
    }

    let mut dec = CcFrameDecoder::new();
    let mut got = Vec::new();
    for chunk in stream.chunks(7) {
        got.extend(dec.push(chunk));
    }

    assert_eq!(got.len(), n as usize);
    for (frame, want) in got.iter().zip(&expected_seq) {
        match &frame.message {
            MavMessage::CC_HEALTH_REPORT(m) => assert_eq!(m.sequence, *want),
            other => panic!("unexpected message {other:?}"),
        }
    }
    let c = dec.counters();
    assert_eq!(
        c,
        &DecodeCounters {
            bytes_in: stream.len() as u64,
            frames_ok: u64::from(n),
            frames_ok_bytes: stream.len() as u64,
            ..DecodeCounters::default()
        },
        "a clean stream must produce perfectly clean counters"
    );
    assert_eq!(dec.pending(), 0);
}

#[test]
fn seeded_fault_soup_conserves_every_byte() {
    // Random mix of: valid frames, CRC-corrupted frames, unknown-id frames,
    // FD-free garbage runs. Exact per-class counts are not asserted (fault
    // interactions are input-dependent by design) — but byte conservation
    // and "no frame invented, none lost from clean spans" always hold.
    for seed in 1..=32u64 {
        let mut rng = Rng::new(0xF00D_0000 + seed);
        let mut stream = Vec::new();
        let mut valid_sent = 0u64;

        for i in 0..120u32 {
            match rng.below(4) {
                0 => {
                    stream.extend_from_slice(&encode_fd_free(i, (i % 256) as u8));
                    valid_sent += 1;
                }
                1 => {
                    // corrupt exactly one payload bit, never producing an
                    // 0xFD byte (that would change the fault model), and
                    // never silently leaving the frame valid
                    let mut f = encode_fd_free(i, (i % 256) as u8);
                    let pos = 10 + rng.below(f.len() - 12); // payload region
                    let mut bit = rng.below(8);
                    if f[pos] ^ (1 << bit) == STX_V2 {
                        bit = (bit + 1) % 8; // at most one flip can make 0xFD
                    }
                    f[pos] ^= 1 << bit;
                    assert_ne!(f[pos], STX_V2);
                    stream.extend_from_slice(&f);
                }
                2 => stream.extend_from_slice(&unknown_id_frame((rng.below(40) + 1) as u8)),
                _ => {
                    let len = rng.below(64);
                    stream.extend_from_slice(&fd_free_garbage(&mut rng, len));
                }
            }
        }

        let mut dec = CcFrameDecoder::new();
        let mut got = 0u64;
        let mut off = 0;
        while off < stream.len() {
            let nmax = stream.len() - off;
            let n = (1 + rng.below(300)).min(nmax);
            got += dec.push(&stream[off..off + n]).len() as u64;
            off += n;
            assert_conservation(&dec);
        }

        let c = dec.counters();
        assert_eq!(c.frames_ok, got);
        assert_eq!(
            c.frames_ok, valid_sent,
            "seed {seed}: every uncorrupted frame must decode, none invented"
        );
        assert_eq!(dec.pending(), 0);
    }
}
