//! companiond — the companion runtime daemon (spec §5.2).
//!
//! Wires cc-link → demux → { cc-timesync | cc-ingest } → broadcast, plus (new
//! in Phase 5) the mission supervisor: on the FC-heartbeat stub-ack it opens a
//! crash-safe mission dataset (`cc-mission-log`), streams `CC_MISSION_CONTEXT`,
//! and logs telemetry + a pre-decode raw capture. A 1 Hz status line
//! (`--status-json` for the machine schema) reports link, timesync, per-stream
//! rates, and the mission-log health.
//!
//! Configuration is layered (cc-config): built-in defaults → TOML file
//! (`--config`, `$CC_CONFIG`, `/etc/companiond/config.toml`) → environment
//! (`CC_<SECTION>__<FIELD>`) → CLI flags:
//!
//! ```text
//! companiond [--config PATH]
//!            [--udp-bind ADDR:PORT] [--remote ADDR:PORT]
//!            [--serial PATH --baud N] [--sysid N]
//!            [--vehicle-id N] [--mission-root DIR] [--disk-floor BYTES]
//!            [--param-snapshot real|stub|off] [--status-json]
//! ```

mod supervise;

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use cc_config::{Config, Overrides, TransportKind};
use cc_ingest::StreamId;
use cc_link::{clock, LinkState};
use cc_mission_log::LogHealth;
use cc_protocol::cc_dialect::MavMessage;
use cc_timesync::Quality;
use tokio::sync::{mpsc, watch};

/// Build-stamped software version (git describe; see build.rs).
const SW_VERSION: &str = env!("CC_SW_VERSION");

struct Cli {
    config_path: Option<std::path::PathBuf>,
    overrides: Overrides,
    /// Phase 6: cc-health-tx scripted severity scenario file.
    health_scenario: Option<std::path::PathBuf>,
}

fn parse_cli() -> Result<Cli, String> {
    let mut config_path = None;
    let mut health_scenario = None;
    let mut o = Overrides::default();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--config" => config_path = Some(std::path::PathBuf::from(val("--config")?)),
            "--health-scenario" => health_scenario = Some(std::path::PathBuf::from(val("--health-scenario")?)),
            "--udp-bind" => o.udp_bind = Some(val("--udp-bind")?),
            "--remote" => o.remote = Some(val("--remote")?),
            "--serial" => {
                o.transport_kind = Some("serial".into());
                o.serial_path = Some(val("--serial")?);
            }
            "--baud" => o.baud = Some(val("--baud")?.parse().map_err(|e| format!("--baud: {e}"))?),
            "--sysid" => o.sysid = Some(val("--sysid")?.parse().map_err(|e| format!("--sysid: {e}"))?),
            "--vehicle-id" => o.vehicle_id = Some(val("--vehicle-id")?.parse().map_err(|e| format!("--vehicle-id: {e}"))?),
            "--mission-root" => o.mission_root = Some(val("--mission-root")?),
            "--disk-floor" => o.disk_floor_bytes = Some(val("--disk-floor")?.parse().map_err(|e| format!("--disk-floor: {e}"))?),
            "--param-snapshot" => o.param_snapshot = Some(val("--param-snapshot")?),
            "--status-json" => o.status_json = Some(true),
            "--help" | "-h" => return Err(usage()),
            other => return Err(format!("unknown argument: {other}\n{}", usage())),
        }
    }
    Ok(Cli { config_path, overrides: o, health_scenario })
}

fn usage() -> String {
    "usage: companiond [--config PATH] [--udp-bind A:P] [--remote A:P] \
     [--serial PATH --baud N] [--sysid N] [--vehicle-id N] [--mission-root DIR] \
     [--disk-floor BYTES] [--param-snapshot real|stub|off] [--status-json] \
     [--health-scenario FILE]"
        .into()
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_cli() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("companiond: {e}");
            return ExitCode::from(2);
        }
    };
    let cfg = match Config::load(cli.config_path.as_deref(), cli.overrides.into_partial()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("companiond: {e}");
            return ExitCode::from(2);
        }
    };

    let sysid = cfg.transport.sysid;
    let status_json = cfg.general.status_json;

    // ---- transport + link (with pre-decode raw tap) ------------------------
    let (rx_half, tx_half, peer_tx) = match cfg.transport.kind {
        TransportKind::Serial => {
            let path = cfg.transport.serial_path.clone().unwrap_or_default();
            match cc_link::transport::serial(&path, cfg.transport.baud) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("companiond: serial {path}: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        TransportKind::Udp => {
            let bind: SocketAddr = match cfg.transport.udp_bind.parse() {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("companiond: --udp-bind {}: {e}", cfg.transport.udp_bind);
                    return ExitCode::from(2);
                }
            };
            let remote = match cfg.transport.remote.as_deref().map(str::parse).transpose() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("companiond: --remote: {e}");
                    return ExitCode::from(2);
                }
            };
            match cc_link::transport::udp(bind, remote).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("companiond: udp bind {bind}: {e}");
                    return ExitCode::from(1);
                }
            }
        }
    };

    // raw datagram tap → mission-log (lossy; never blocks RX)
    let (raw_tx, raw_rx) = mpsc::channel::<Vec<u8>>(256);
    let mut link = cc_link::spawn_with_raw_tap(rx_half, tx_half, peer_tx, sysid, Some(raw_tx));
    let mut frames = link.take_frames();

    // ---- boot-id bridge, timesync, ingest ----------------------------------
    let (boot_tx, boot_rx) = watch::channel(0u32);
    let runner = cc_timesync::runner::spawn(link.tx.clone(), boot_rx.clone());
    let (ingest_tx, ingest_rx) = mpsc::channel::<cc_link::LinkFrame>(512);
    let ingest = cc_ingest::spawn(ingest_rx, runner.snapshot.clone(), link.status.clone(), boot_tx);

    // report-ACK watch: the monitor's echoed CC_SAFETY_STATUS.last_report_sequence
    // (feeds cc-health-tx so a CRITICAL repeat stops once acknowledged).
    let (ack_tx, ack_rx) = watch::channel(0u32);

    // ---- demux: TIMESYNC replies → timesync; tap CC_SAFETY_STATUS ack;
    //      the rest → ingest --------------------------------------------------
    let replies = runner.replies.clone();
    tokio::spawn(async move {
        while let Some(frame) = frames.recv().await {
            if let MavMessage::TIMESYNC(t) = &frame.message {
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
            if let MavMessage::CC_SAFETY_STATUS(s) = &frame.message {
                let _ = ack_tx.send(s.last_report_sequence);
            }
            if ingest_tx.send(frame).await.is_err() {
                return;
            }
        }
    });

    // ---- mission supervisor (handshake + mission-log lifecycle) -------------
    let log_health = Arc::new(LogHealth::default());
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let supervisor = tokio::spawn(supervise::run(
        cfg.clone(),
        link.tx.clone(),
        link.status.clone(),
        ingest.events.clone(),
        boot_rx.clone(),
        raw_rx,
        log_health.clone(),
        SW_VERSION.to_string(),
        shutdown_rx,
        cli.health_scenario,
        ack_rx,
        ingest.stats.clone(),
    ));

    // ---- status loop --------------------------------------------------------
    let stats = ingest.stats.clone();
    let ts_watch = runner.snapshot.clone();
    let status_watch = link.status.clone();

    eprintln!(
        "companiond {SW_VERSION} up — transport {}, sysid {}, vehicle {}, mission-root {}",
        if cfg.transport.kind == TransportKind::Serial { "serial" } else { "udp" },
        sysid,
        cfg.general.vehicle_id,
        cfg.general.mission_root.display(),
    );

    let mut prev_counts = [0u64; 8];
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = tokio::signal::ctrl_c() => {
                eprintln!("companiond: shutdown — finalizing mission");
                let _ = shutdown_tx.send(true);
                // give the mission-log a bounded window to seal + mark complete
                let _ = tokio::time::timeout(Duration::from_secs(10), supervisor).await;
                return ExitCode::SUCCESS;
            }
        }

        let ls = *status_watch.borrow();
        let ts = *ts_watch.borrow();
        let lstats = link.stats();
        let boot = *boot_rx.borrow();
        let mission = stats.mission_id.load(std::sync::atomic::Ordering::Relaxed);
        let log = log_health.snapshot();

        let mut hz = [0f64; 8];
        for s in StreamId::ALL {
            let i = s as usize;
            let n = stats.stream_count(s);
            hz[i] = (n - prev_counts[i]) as f64;
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

        if status_json {
            let mut s = String::with_capacity(1200);
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
                "\"log\":{{\"shed_stage\":\"{}\",\"warn\":{},\"drops\":{},\"raw_drops\":{},\"parts\":{},\"write_errors\":{},\"lagged\":{},\"free_mib\":{}}},",
                log.stage_name(),
                log.warn,
                log.dropped.iter().sum::<u64>(),
                log.raw_dropped,
                log.parts_sealed,
                log.write_errors,
                log.lagged,
                log.last_free_bytes / (1024 * 1024),
            ));
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
                "[link {link_str}] state {:.0} imu {:.0} pwr {:.0} gps {:.0} est {:.0} act {:.0} Hz | gaps {} | crc {} | ts {q_str} off {:.3} ms | boot {boot} mission {mission} | log {} warn {} drops {}",
                hz[StreamId::State as usize],
                hz[StreamId::Imu as usize],
                hz[StreamId::Power as usize],
                hz[StreamId::Gps as usize],
                hz[StreamId::Estimator as usize],
                hz[StreamId::Actuator as usize],
                stats.total_gaps(),
                lstats.crc_errors,
                ts.offset_ns as f64 / 1e6,
                log.stage_name(),
                log.warn,
                log.total_dropped(),
            );
        }
    }
}
