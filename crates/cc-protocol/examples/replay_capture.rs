//! Replay a length-prefixed UDP datagram capture through CcFrameDecoder,
//! printing per-type counts + decoder counters. Debug scaffolding for the
//! Phase 4 timesync investigation (kept: useful for future wire captures).
//!
//! Capture format: repeated [u32 LE length][datagram bytes].

use std::collections::BTreeMap;

use cc_protocol::CcFrameDecoder;
use cc_protocol::cc_dialect::MavMessage;

fn main() {
    let path = std::env::args().nth(1).expect("usage: replay_capture <capture.bin>");
    let data = std::fs::read(path).expect("read capture");

    let mut dec = CcFrameDecoder::new();
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut replies = 0u64;
    let mut own_requests = 0u64;

    let mut off = 0usize;
    let mut datagrams = 0usize;
    while off + 4 <= data.len() {
        let len = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        let dg = &data[off..off + len];
        off += len;
        datagrams += 1;

        for frame in dec.push(dg) {
            if let MavMessage::TIMESYNC(t) = &frame.message {
                if t.tc1 != 0 {
                    replies += 1;
                } else {
                    own_requests += 1;
                }
            }
            let name = format!("{:?}", frame.msg_id);
            *counts.entry(name).or_default() += 1;
        }
    }

    let c = dec.counters();
    println!("datagrams: {datagrams}");
    println!("frames_ok: {}", c.frames_ok);
    println!("TIMESYNC replies (tc1!=0): {replies}");
    println!("TIMESYNC requests (tc1==0): {own_requests}");
    println!("crc_errors: {}  garbage_bytes: {}  unknown_msg_ids: {}  suspect: {}  bad_payloads: {}",
             c.crc_errors, c.garbage_bytes, c.unknown_msg_ids, c.suspect_candidates, c.bad_payloads);
    println!("pending: {}", dec.pending());
}
