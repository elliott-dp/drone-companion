//! cc-link behavior tests (dev plan Phase 4.1) over real localhost UDP:
//! RX decode path + counters, the source gate, heartbeat cadence, and
//! garbage resilience. Real sockets → real time (short, generous bounds).

use std::net::SocketAddr;
use std::time::Duration;

use cc_link::{spawn, transport, LinkState};
use cc_protocol::cc_dialect::*;
use cc_protocol::identity;
use cc_protocol::mavlink_core::{write_v2_msg, MavHeader};
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// FC-side test peer: a UDP socket playing PX4's CCFC instance.
struct FcPeer {
    sock: UdpSocket,
    link_addr: SocketAddr,
}

impl FcPeer {
    async fn send_msg(&self, sysid: u8, compid: u8, seq: u8, msg: &MavMessage) {
        let mut buf = Vec::new();
        write_v2_msg(
            &mut buf,
            MavHeader { system_id: sysid, component_id: compid, sequence: seq },
            msg,
        )
        .unwrap();
        self.sock.send_to(&buf, self.link_addr).await.unwrap();
    }
}

async fn rig() -> (cc_link::Link, FcPeer) {
    // bind link on an ephemeral port, peer learned from first datagram
    let (rx_half, tx_half, peer_tx) = transport::udp("127.0.0.1:0".parse().unwrap(), None)
        .await
        .unwrap();
    let link_addr = match &rx_half {
        transport::RxHalf::Udp(sock) => sock.local_addr().unwrap(),
        _ => unreachable!(),
    };
    let link = spawn(rx_half, tx_half, peer_tx, identity::SYSID_VEHICLE_DEFAULT);
    let peer = FcPeer {
        sock: UdpSocket::bind("127.0.0.1:0").await.unwrap(),
        link_addr,
    };
    (link, peer)
}

fn heartbeat() -> MavMessage {
    MavMessage::HEARTBEAT(HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: MavType::MAV_TYPE_QUADROTOR,
        autopilot: MavAutopilot::MAV_AUTOPILOT_PX4,
        base_mode: MavModeFlag::empty(),
        system_status: MavState::MAV_STATE_ACTIVE,
        mavlink_version: 3,
    })
}

fn state_msg(seq: u32) -> MavMessage {
    MavMessage::CC_TELEMETRY_STATE(CC_TELEMETRY_STATE_DATA {
        sequence: seq,
        schema_version: identity::CC_SCHEMA_VERSION,
        ..Default::default()
    })
}

#[tokio::test]
async fn rx_decodes_and_source_gate_drops() {
    let (mut link, peer) = rig().await;
    let mut frames = link.take_frames();

    // valid FC-originated frame
    peer.send_msg(1, identity::COMPID_FC, 0, &state_msg(7)).await;
    let f = timeout(Duration::from_secs(5), frames.recv()).await.unwrap().unwrap();
    assert_eq!(f.msg_id, 54000);
    assert_eq!(f.header.component_id, identity::COMPID_FC);

    // wrong component for an FC-class message -> dropped + counted
    peer.send_msg(1, 42, 1, &state_msg(8)).await;
    // wrong system id -> dropped + counted
    peer.send_msg(9, identity::COMPID_FC, 2, &state_msg(9)).await;
    // then a good one to sequence the assertion
    peer.send_msg(1, identity::COMPID_FC, 3, &state_msg(10)).await;

    let f = timeout(Duration::from_secs(5), frames.recv()).await.unwrap().unwrap();
    match &f.message {
        MavMessage::CC_TELEMETRY_STATE(d) => assert_eq!(d.sequence, 10, "gated frames must not pass"),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(link.stats().rx_bad_source, 2);
    // the decoder decodes every WELL-FORMED frame (4); the source gate
    // drops the two imposters after decode — layered counters by design
    assert_eq!(link.stats().frames_ok, 4);
}

#[tokio::test]
async fn garbage_between_frames_is_resynced_and_counted() {
    let (mut link, peer) = rig().await;
    let mut frames = link.take_frames();

    peer.send_msg(1, identity::COMPID_FC, 0, &state_msg(1)).await;
    timeout(Duration::from_secs(5), frames.recv()).await.unwrap().unwrap();

    // pure garbage datagram (no 0xFD), then a valid frame
    let garbage = vec![0x11u8; 977];
    peer.sock.send_to(&garbage, peer.link_addr).await.unwrap();
    peer.send_msg(1, identity::COMPID_FC, 1, &state_msg(2)).await;

    let f = timeout(Duration::from_secs(5), frames.recv()).await.unwrap().unwrap();
    match &f.message {
        MavMessage::CC_TELEMETRY_STATE(d) => assert_eq!(d.sequence, 2),
        other => panic!("unexpected {other:?}"),
    }
    let s = link.stats();
    assert_eq!(s.garbage_bytes, 977, "every garbage byte accounted");
    assert_eq!(s.crc_errors, 0);
}

#[tokio::test]
async fn companion_heartbeat_flows_at_one_hz_and_link_state_tracks_fc() {
    let (link, peer) = rig().await;

    // teach the link its peer + feed an FC heartbeat -> link must go UP
    peer.send_msg(1, identity::COMPID_FC, 0, &heartbeat()).await;

    // collect our companion heartbeats on the peer socket for ~2.5 s
    let mut got = 0usize;
    let mut buf = [0u8; 2048];
    let deadline = tokio::time::Instant::now() + Duration::from_millis(2500);
    let mut saw_comp_id = 0u8;
    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_millis(300), peer.sock.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) if n > 9 => {
                // MAVLink2: compid at byte 6, msgid 0 for heartbeat
                let msgid = u32::from(buf[7]) | u32::from(buf[8]) << 8 | u32::from(buf[9]) << 16;
                if msgid == 0 {
                    got += 1;
                    saw_comp_id = buf[6];
                }
            }
            _ => {}
        }
    }
    assert!((2..=4).contains(&got), "expected ~2-3 heartbeats in 2.5 s, got {got}");
    assert_eq!(saw_comp_id, identity::COMPID_CC, "companion heartbeats carry comp 191");

    // the single FC heartbeat from test start has aged past 2.5 s by now:
    // the link must have judged that DEGRADED (spec §5.3 thresholds)
    assert_eq!(link.status.borrow().state, LinkState::Degraded, "aged heartbeat -> DEGRADED");

    // a fresh FC heartbeat restores UP within one state-task tick (500 ms)
    peer.send_msg(1, identity::COMPID_FC, 1, &heartbeat()).await;
    tokio::time::sleep(Duration::from_millis(700)).await;
    let status = *link.status.borrow();
    assert_eq!(status.state, LinkState::Up, "fresh FC heartbeat -> UP");
    assert!(status.fc_heartbeat_age_ns.is_some());
}
