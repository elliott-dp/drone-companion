//! Incremental MAVLink 2 frame decoder with exact fault accounting.
//!
//! This is the protocol-layer foundation `cc-link` (spec §5.3 RX path) and
//! `cc-ingest` (spec §5.5) build on in Phase 4. Phase 1 proves it with the
//! fuzz/property suite (`tests/fuzz_decoder.rs`): the decoder must **never
//! panic** on any byte stream, must **resynchronize** on the MAVLink 2 STX
//! byte after garbage or a hot-plug (spec §2.4), and its counters must
//! account for **every byte pushed into it** (see [`DecodeCounters`]).
//!
//! ## Resync policy (documented so the counters are testable)
//!
//! * Bytes are scanned for `0xFD` (STX). Every byte discarded during the
//!   hunt increments `garbage_bytes`.
//! * A candidate frame whose **CRC fails** costs one `crc_errors` and the
//!   decoder then discards **only the STX byte** and rescans from the next
//!   byte. This is deliberately conservative: a real frame that begins
//!   *inside* a corrupted candidate is never lost. The net effect is that a
//!   corrupted frame of length L whose interior contains no `0xFD`
//!   eventually accounts for exactly L `garbage_bytes` and one `crc_errors`.
//! * A candidate with an **unknown message ID** cannot be CRC-checked at all
//!   (CRC_EXTRA is per-message and unknown by definition). Per spec §5.5
//!   unknown IDs must be *counted and ignored, never a crash* — they signal
//!   schema skew, e.g. an FC newer than the CC. The decoder skips the whole
//!   declared frame **only** when the declared end lands on another STX or
//!   the end of buffered data (`unknown_msg_ids`/`unknown_msg_bytes`);
//!   otherwise the candidate is treated as suspect garbage
//!   (`suspect_candidates`) and only the STX byte is discarded. This keeps
//!   well-formed unknown streams cheap (one count per frame, no phantom CRC
//!   errors) without letting a `0xFD` inside random garbage swallow a real
//!   frame that follows.
//! * A frame with **unknown incompat flags** must be dropped per the MAVLink
//!   spec (`bad_incompat_flags`); only bit 0 (`MAVLINK_IFLAG_SIGNED`) is
//!   known. Signed frames are length-handled (13-byte signature) and
//!   surfaced with `signed = true`; signature *verification* is not this
//!   layer's job (the wired private link does not use signing, spec §8).
//! * A frame whose CRC passes but whose payload fails semantic decode
//!   (e.g. invalid enum discriminant) costs one `bad_payloads`; the whole
//!   frame is skipped — the CRC already proved the framing.
//!
//! Memory is bounded: after every [`FrameDecoder::push`] call the internal
//! buffer holds at most one incomplete frame prefix
//! (< [`V2_MAX_FRAME_LEN`] bytes).

use core::marker::PhantomData;

use mavlink_core::{MAVLinkV2MessageRaw, MavHeader, MavlinkVersion, Message};

/// MAVLink 2 start-of-text (STX) marker.
pub const STX_V2: u8 = 0xFD;

/// Header length including the STX byte.
pub const V2_HEADER_LEN: usize = 10;

/// CRC length.
pub const V2_CRC_LEN: usize = 2;

/// Signature length when `MAVLINK_IFLAG_SIGNED` is set.
pub const V2_SIGNATURE_LEN: usize = 13;

/// Largest possible MAVLink 2 frame: header + 255 payload + CRC + signature.
pub const V2_MAX_FRAME_LEN: usize = V2_HEADER_LEN + 255 + V2_CRC_LEN + V2_SIGNATURE_LEN;

/// Incompatibility flag: frame is signed.
pub const IFLAG_SIGNED: u8 = 0x01;

/// Exact byte/fault accounting. The invariant, asserted by the property
/// tests after every interaction:
///
/// ```text
/// bytes_in == frames_ok_bytes + unknown_msg_bytes + bad_payload_bytes
///             + garbage_bytes + pending()
/// ```
///
/// (`crc_errors`, `bad_incompat_flags` and `suspect_candidates` count
/// *events*, not bytes — the bytes of those candidates drain into
/// `garbage_bytes` as the scanner steps past them.)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DecodeCounters {
    /// Total bytes ever pushed into the decoder.
    pub bytes_in: u64,
    /// Frames decoded successfully.
    pub frames_ok: u64,
    /// Bytes consumed by successfully decoded frames.
    pub frames_ok_bytes: u64,
    /// Candidates with a known message ID whose CRC check failed
    /// (wiring fault, corruption, or a false STX inside garbage).
    pub crc_errors: u64,
    /// Well-delimited frames whose message ID is not in the dialect
    /// (forward compatibility / schema skew tripwire, spec §5.5).
    pub unknown_msg_ids: u64,
    /// Bytes consumed by those unknown-ID frames.
    pub unknown_msg_bytes: u64,
    /// STX candidates with an unknown message ID that did *not* end on a
    /// frame boundary — treated as garbage, one byte discarded.
    pub suspect_candidates: u64,
    /// CRC-valid frames whose payload failed semantic decode
    /// (e.g. out-of-range enum value).
    pub bad_payloads: u64,
    /// Bytes consumed by those bad-payload frames.
    pub bad_payload_bytes: u64,
    /// Frames dropped for carrying unknown incompatibility flags.
    pub bad_incompat_flags: u64,
    /// Bytes discarded while hunting for STX (includes, byte by byte, the
    /// carcasses of CRC-failed / bad-incompat / suspect candidates).
    pub garbage_bytes: u64,
}

impl DecodeCounters {
    /// Bytes attributed to a terminal bucket (everything but `pending`).
    pub fn accounted_bytes(&self) -> u64 {
        self.frames_ok_bytes + self.unknown_msg_bytes + self.bad_payload_bytes + self.garbage_bytes
    }
}

/// One successfully decoded frame.
#[derive(Debug, Clone)]
pub struct DecodedFrame<M> {
    /// Sender identity + per-link sequence from the MAVLink header.
    pub header: MavHeader,
    /// The decoded dialect message.
    pub message: M,
    /// 24-bit message ID as found on the wire.
    pub msg_id: u32,
    /// On-wire (possibly zero-truncated) payload length.
    pub payload_len: u8,
    /// Frame carried `MAVLINK_IFLAG_SIGNED` (signature not verified here).
    pub signed: bool,
    /// Total frame length in bytes, including STX/CRC/signature.
    pub frame_len: usize,
}

/// Incremental push-based MAVLink 2 decoder. Generic over the dialect's
/// message enum; use [`crate::CcFrameDecoder`] for the cc dialect.
pub struct FrameDecoder<M: Message> {
    buf: Vec<u8>,
    counters: DecodeCounters,
    _dialect: PhantomData<M>,
}

impl<M: Message> Default for FrameDecoder<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Message> FrameDecoder<M> {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(2 * V2_MAX_FRAME_LEN),
            counters: DecodeCounters::default(),
            _dialect: PhantomData,
        }
    }

    /// Current fault/byte accounting.
    pub fn counters(&self) -> &DecodeCounters {
        &self.counters
    }

    /// Bytes buffered waiting for the rest of a frame.
    pub fn pending(&self) -> usize {
        self.buf.len()
    }

    /// Feed bytes; returns every frame completed by this chunk.
    ///
    /// Never panics, whatever the input. Call with anything from single
    /// bytes to whole reads — framing state is carried across calls.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<DecodedFrame<M>> {
        self.counters.bytes_in += bytes.len() as u64;
        self.buf.extend_from_slice(bytes);

        let mut out = Vec::new();
        let mut pos = 0usize;

        loop {
            // -- 1. hunt for STX ------------------------------------------
            while pos < self.buf.len() && self.buf[pos] != STX_V2 {
                pos += 1;
                self.counters.garbage_bytes += 1;
            }
            if pos >= self.buf.len() {
                break;
            }

            // -- 2. header ---------------------------------------------------
            if self.buf.len() - pos < V2_HEADER_LEN {
                break; // incomplete header: wait for more bytes
            }
            let payload_len = self.buf[pos + 1] as usize;
            let incompat = self.buf[pos + 2];

            if incompat & !IFLAG_SIGNED != 0 {
                // Unknown incompat flag: MUST drop (MAVLink 2 rule). Discard
                // the STX and rescan — the "frame" may be garbage anyway.
                self.counters.bad_incompat_flags += 1;
                self.counters.garbage_bytes += 1;
                pos += 1;
                continue;
            }
            let signed = incompat & IFLAG_SIGNED != 0;
            let frame_len = V2_HEADER_LEN
                + payload_len
                + V2_CRC_LEN
                + if signed { V2_SIGNATURE_LEN } else { 0 };

            if self.buf.len() - pos < frame_len {
                break; // incomplete frame: wait for more bytes
            }

            let frame = &self.buf[pos..pos + frame_len];
            let msg_id = u32::from(frame[7])
                | (u32::from(frame[8]) << 8)
                | (u32::from(frame[9]) << 16);

            // -- 3. unknown message id: cannot CRC-check (no CRC_EXTRA) ----
            if M::default_message_from_id(msg_id).is_none() {
                let end = pos + frame_len;
                let ends_on_boundary = end == self.buf.len() || self.buf[end] == STX_V2;
                if ends_on_boundary {
                    self.counters.unknown_msg_ids += 1;
                    self.counters.unknown_msg_bytes += frame_len as u64;
                    pos += frame_len;
                } else {
                    self.counters.suspect_candidates += 1;
                    self.counters.garbage_bytes += 1;
                    pos += 1;
                }
                continue;
            }

            // -- 4. CRC ------------------------------------------------------
            let mut raw_bytes = [0u8; 1 + 9 + 255 + 2 + 13];
            raw_bytes[..frame_len].copy_from_slice(frame);
            let raw = MAVLinkV2MessageRaw::from_bytes_unparsed(raw_bytes);
            if !raw.has_valid_crc::<M>() {
                // Corruption or false STX: conservative resync, drop only
                // the STX byte so a genuine frame inside is never lost.
                self.counters.crc_errors += 1;
                self.counters.garbage_bytes += 1;
                pos += 1;
                continue;
            }

            // -- 5. payload decode -------------------------------------------
            match M::parse(MavlinkVersion::V2, msg_id, raw.payload()) {
                Ok(message) => {
                    self.counters.frames_ok += 1;
                    self.counters.frames_ok_bytes += frame_len as u64;
                    out.push(DecodedFrame {
                        header: MavHeader {
                            system_id: frame[5],
                            component_id: frame[6],
                            sequence: frame[4],
                        },
                        message,
                        msg_id,
                        payload_len: payload_len as u8,
                        signed,
                        frame_len,
                    });
                }
                Err(_) => {
                    // CRC proved the framing, so skipping the whole frame is
                    // safe; the payload is semantically invalid (bad enum…).
                    self.counters.bad_payloads += 1;
                    self.counters.bad_payload_bytes += frame_len as u64;
                }
            }
            pos += frame_len;
        }

        self.buf.drain(..pos);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::cc_dialect::{CC_LOG_CONTROL_DATA, MavMessage};

    fn frame_bytes(msg: &MavMessage, seq: u8) -> Vec<u8> {
        let mut v = Vec::new();
        mavlink_core::write_v2_msg(
            &mut v,
            MavHeader {
                system_id: 1,
                component_id: crate::identity::COMPID_CC,
                sequence: seq,
            },
            msg,
        )
        .unwrap();
        v
    }

    fn sample_msg() -> MavMessage {
        MavMessage::CC_LOG_CONTROL(CC_LOG_CONTROL_DATA {
            companion_timestamp_us: 111_222_333_444,
            sequence: 7,
            requested_profile: crate::dialects::cc_dialect::CcLogProfile::CC_PROFILE_AI_UART,
            schema_version: crate::identity::CC_SCHEMA_VERSION,
        })
    }

    #[test]
    fn decodes_single_frame() {
        let mut dec = FrameDecoder::<MavMessage>::new();
        let bytes = frame_bytes(&sample_msg(), 3);
        let frames = dec.push(&bytes);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].msg_id, 54013);
        assert_eq!(frames[0].header.sequence, 3);
        assert_eq!(dec.counters().frames_ok, 1);
        assert_eq!(dec.counters().frames_ok_bytes, bytes.len() as u64);
        assert_eq!(dec.pending(), 0);
    }

    #[test]
    fn reassembles_across_arbitrary_chunk_sizes() {
        let bytes = frame_bytes(&sample_msg(), 0);
        for chunk in 1..bytes.len() {
            let mut dec = FrameDecoder::<MavMessage>::new();
            let mut got = 0;
            for c in bytes.chunks(chunk) {
                got += dec.push(c).len();
            }
            assert_eq!(got, 1, "chunk size {chunk}");
            assert_eq!(dec.pending(), 0);
        }
    }

    #[test]
    fn incomplete_frame_stays_pending() {
        let mut dec = FrameDecoder::<MavMessage>::new();
        let bytes = frame_bytes(&sample_msg(), 0);
        let cut = bytes.len() - 4;
        assert!(dec.push(&bytes[..cut]).is_empty());
        assert_eq!(dec.pending(), cut);
        // counters account for pending bytes
        let c = dec.counters();
        assert_eq!(c.bytes_in, cut as u64);
        assert_eq!(c.accounted_bytes(), 0);
        // rest arrives -> frame completes
        let frames = dec.push(&bytes[cut..]);
        assert_eq!(frames.len(), 1);
        assert_eq!(dec.pending(), 0);
    }

    #[test]
    fn leading_garbage_counted_exactly() {
        let mut dec = FrameDecoder::<MavMessage>::new();
        let mut stream = vec![0x00, 0x11, 0x22, 0x33, 0x44]; // no 0xFD
        let frame = frame_bytes(&sample_msg(), 9);
        stream.extend_from_slice(&frame);
        let frames = dec.push(&stream);
        assert_eq!(frames.len(), 1);
        assert_eq!(dec.counters().garbage_bytes, 5);
        assert_eq!(dec.counters().frames_ok_bytes, frame.len() as u64);
    }
}
