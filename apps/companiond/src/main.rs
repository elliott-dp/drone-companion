//! companiond v0 — the companion runtime daemon (spec §5.2, Phase 4 scope).
//!
//! Wires cc-link → demux → { cc-timesync | cc-ingest } and prints a 1 Hz
//! status line. `--status-json` switches to one JSON object per line
//! (fixed schema, hand-emitted — the Phase 4 harness's interface; schema
//! documented in tools/phase4/README.md). Config layering (cc-config)
//! arrives in Phase 5; v0 takes flags:
//!
//! ```text
//! companiond [--udp-bind ADDR:PORT]     default 0.0.0.0:24040 (SITL CCFC link)
//!            [--remote ADDR:PORT]       fixed peer (else learned from first rx)
//!            [--serial PATH --baud N]   TELEM3 transport instead of UDP
//!            [--sysid N]                MAVLink system id (default 1)
//!            [--status-json]            machine-readable status lines
//! ```
//!
//! Task graph (spec §5.2, Phase 4 subset — every channel bounded):
//!
//! ```text
//! [udp|serial] → cc-link RX ─→ demux ─→ TIMESYNC replies → cc-timesync ⇄ P0 TX
//!                                   └─→ cc-ingest → broadcast<TelemetryEvent>
//! cc-link heartbeat (1 Hz) → P0 TX      status task (1 Hz) → stdout
//! ```

use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

use cc_ingest::StreamId;
use cc_link::{clock, LinkState};
use cc_protocol::cc_dialect::MavMessage;
use cc_protocol::identity;
use cc_timesync::Quality;
use tokio::sync::{mpsc, watch};

struct Args {
    udp_bind: SocketAddr,
    remote: Option<SocketAddr>,
    serial: Option<String>,
    baud: u32,
    sysid: u8,
    status_json: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        udp_bind: "0.0.0.0:24040".parse().unwrap(),
        remote: None,
        serial: None,
        baud: 921_600,
        sysid: identity::SYSID_VEHICLE_DEFAULT,
        status_json: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--udp-bind" => args.udp_bind = val("--udp-bind")?.parse().map_err(|e| format!("--udp-bind: {e}"))?,
            "--remote" => args.remote = Some(val("--remote")?.parse().map_err(|e| format!("--remote: {e}"))?),
            "--serial" => args.serial = Some(val("--serial")?),
            "--baud" => args.baud = val("--baud")?.parse().map_err(|e| format!("--baud: {e}"))?,
            "--sysid" => args.sysid = val("--sysid")?.parse().map_err(|e| format!("--sysid: {e}"))?,
            "--status-json" => args.status_json = true,
            "--help" | "-h" => return Err("usage: companiond [--udp-bind A:P] [--remote A:P] \
                                           [--serial PATH --baud N] [--sysid N] [--status-json]".into()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("companiond: {e}");
            return ExitCode::from(2);
        }
    };

    // ---- transport + link -----------------------------------------------
    let (rx_half, tx_half, peer_tx) = if let Some(path) = &args.serial {
        match cc_link::transport::serial(path, args.baud) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("companiond: serial {path}: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        match cc_link::transport::udp(args.udp_bind, args.remote).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("companiond: udp bind {}: {e}", args.udp_bind);
                return ExitCode::from(1);
            }
        }
    };

    let mut link = cc_link::spawn(rx_half, tx_half, peer_tx, args.sysid);
    let mut frames = link.take_frames();

    // ---- boot-id bridge, timesync, ingest ---------------------------------
    // The boot-id watch is created here because its two ends belong to
    // different crates: cc-ingest writes it (STATE carries px4_boot_id),
    // cc-timesync invalidates on it (spec §5.4).
    let (boot_tx, boot_rx) = watch::channel(0u32);

    let runner = cc_timesync::runner::spawn(link.tx.clone(), boot_rx.clone());

    let (ingest_tx, ingest_rx) = mpsc::channel::<cc_link::LinkFrame>(512);
    let ingest = cc_ingest::spawn(
        ingest_rx,
        runner.snapshot.clone(),
        link.status.clone(),
        boot_tx,
    );

    // ---- demux: TIMESYNC replies → timesync; the rest → ingest -------------
    let replies = runner.replies.clone();
    tokio::spawn(async move {
        while let Some(frame) = frames.recv().await {
            if let MavMessage::TIMESYNC(t) = &frame.message {
                // tc1 != 0 marks a RESPONSE (PX4 filled its clock in);
                // requests echo through untouched channels elsewhere
                if t.tc1 != 0 {
                    let _ = replies
                        .send(cc_timesync::runner::Reply {
                            tc1_ns: t.tc1,
                            ts1_ns: t.ts1,
                            rx_ns: clock::now_ns(),
                        })
                        .await;
                }
                continue;
            }
            if ingest_tx.send(frame).await.is_err() {
                return; // ingest gone: shutting down
            }
        }
    });

    // ---- status loop --------------------------------------------------------
    let stats = ingest.stats.clone();
    let ts_watch = runner.snapshot.clone();
    let status_watch = link.status.clone();

    eprintln!(
        "companiond v0 up — transport {}, sysid {}",
        args.serial.as_deref().unwrap_or("udp"),
        args.sysid
    );

    let mut prev_counts = [0u64; 8];
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = tokio::signal::ctrl_c() => {
                eprintln!("companiond: shutdown");
                return ExitCode::SUCCESS;
            }
        }

        let ls = *status_watch.borrow();
        let ts = *ts_watch.borrow();
        let lstats = link.stats();
        let boot = *boot_rx.borrow();
        let mission = stats.mission_id.load(std::sync::atomic::Ordering::Relaxed);

        // per-stream Hz over the last interval
        let mut hz = [0f64; 8];
        for s in StreamId::ALL {
            let i = s as usize;
            let n = stats.stream_count(s);
            hz[i] = (n - prev_counts[i]) as f64; // 1 s interval → count == Hz
            prev_counts[i] = n;
        }

        let link_str = match ls.state {
            LinkState::Up => "UP",
            LinkState::Degraded => "DEGRADED",
            LinkState::Down => "DOWN",
        };
        let q_str = match ts.quality {
            Quality::Locked => "LOCKED",
            Quality::Degraded => "DEGRADED",
            Quality::Unlocked => "UNLOCKED",
        };

        if args.status_json {
            // fixed schema, hand-emitted (documented in tools/phase4/README.md)
            let mut s = String::with_capacity(1024);
            s.push_str(&format!(
                "{{\"t_ns\":{},\"link\":\"{}\",\"fc_hb_age_ms\":{},\"px4_boot_id\":{},\"mission_id\":{},",
                clock::now_ns(),
                link_str,
                ls.fc_heartbeat_age_ns.map_or("null".into(), |a| (a / 1_000_000).to_string()),
                boot,
                mission,
            ));
            s.push_str(&format!(
                "\"timesync\":{{\"q\":\"{}\",\"offset_ns\":{},\"rtt_us\":{},\"window\":{},\"rejected\":{}}},",
                q_str, ts.offset_ns, ts.rtt_ns / 1000, ts.window_len, ts.rejected
            ));
            s.push_str("\"streams\":{");
            for (i, st) in StreamId::ALL.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&format!(
                    "\"{}\":{{\"n\":{},\"hz\":{:.1},\"gaps\":{},\"stale\":{}}}",
                    st.name(),
                    stats.stream_count(*st),
                    hz[*st as usize],
                    stats.stream_gaps(*st),
                    stats.stream_stale(*st),
                ));
            }
            s.push_str("},");
            s.push_str(&format!(
                "\"counters\":{{\"frames_ok\":{},\"crc_errors\":{},\"garbage_bytes\":{},\"unknown_msg\":{},\"bad_payloads\":{},\"bad_source\":{},\"bad_schema\":{},\"tx_frames\":{},\"tx_errors\":{},\"p0_stalls\":{},\"rx_drops\":{}}}}}",
                lstats.frames_ok,
                lstats.crc_errors,
                lstats.garbage_bytes,
                lstats.unknown_msg_ids,
                lstats.bad_payloads,
                lstats.rx_bad_source,
                stats.bad_schema.load(std::sync::atomic::Ordering::Relaxed),
                lstats.tx_frames,
                lstats.tx_errors,
                lstats.p0_stalls,
                lstats.rx_channel_drops,
            ));
            println!("{s}");
        } else {
            println!(
                "[link {link_str}] state {:.0} imu {:.0} pwr {:.0} gps {:.0} est {:.0} act {:.0} Hz | gaps {} | crc {} | ts {q_str} off {:.3} ms rtt {:.3} ms | boot {boot} mission {mission}",
                hz[StreamId::State as usize],
                hz[StreamId::Imu as usize],
                hz[StreamId::Power as usize],
                hz[StreamId::Gps as usize],
                hz[StreamId::Estimator as usize],
                hz[StreamId::Actuator as usize],
                stats.total_gaps(),
                lstats.crc_errors,
                ts.offset_ns as f64 / 1e6,
                ts.rtt_ns as f64 / 1e6,
            );
        }
    }
}
