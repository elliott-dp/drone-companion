//! Mission supervisor: the Phase-5 handshake + mission-log lifecycle.
//!
//! Waits for the stub-ack (first FC heartbeat → link leaves DOWN), opens (or
//! resumes) the mission, writes the PX4 param snapshot, spawns the single
//! disk-touching mission-log task (a second, lossy broadcast subscriber + the
//! pre-decode raw tap), and streams `CC_MISSION_CONTEXT` at 1 Hz until
//! shutdown — at which point it seals the dataset and marks the mission
//! complete before returning.

use std::sync::Arc;
use std::time::Duration;

use cc_config::{Config, ParamSnapshotMode};
use cc_ingest::TelemetryEvent;
use cc_link::{LinkState, LinkStatus, Priority, TxHandle};
use cc_mission_log::env::RealEnv;
use cc_mission_log::params::ParamSnapshot;
use cc_mission_log::{LogHealth, Mission, OpenError};
use cc_protocol::cc_dialect::{MavMessage, CC_MISSION_CONTEXT_DATA};
use cc_protocol::mavlink_core::types::CharArray;
use cc_protocol::{dialect_hash, identity};
use tokio::sync::{broadcast, mpsc, oneshot, watch};

/// Run the supervisor to completion (spawned as a task by `main`).
#[allow(clippy::too_many_arguments)]
pub async fn run(
    cfg: Config,
    tx: TxHandle,
    mut link_status: watch::Receiver<LinkStatus>,
    events: broadcast::Sender<TelemetryEvent>,
    boot_rx: watch::Receiver<u32>,
    raw_rx: mpsc::Receiver<Vec<u8>>,
    health: Arc<LogHealth>,
    sw_version: String,
    mut shutdown: watch::Receiver<bool>,
) {
    // 1. Stub-ack: first FC heartbeat (link leaves DOWN). Bail if we are told
    //    to shut down before we ever see the FC. (Phase 6 flips the predicate
    //    to "CC_SAFETY_STATUS leaves UNKNOWN"; the config selects it.)
    loop {
        if *shutdown.borrow() {
            return;
        }
        if link_status.borrow().state != LinkState::Down {
            break;
        }
        tokio::select! {
            _ = link_status.changed() => {}
            _ = shutdown.changed() => {}
        }
    }

    // 2. Open (or resume) the mission. Below-floor = refuse to log but keep
    //    the link running (graceful, post-boot).
    let px4_boot_id = *boot_rx.borrow();
    let env = RealEnv::default();
    let mission = match Mission::open(
        cfg.clone(),
        env.clock.clone(),
        env.space.clone(),
        env.syncer.clone(),
        health.clone(),
        sw_version.clone(),
        px4_boot_id,
        |w| eprintln!("companiond mission-log: {w}"),
    ) {
        Ok(m) => m,
        Err(OpenError::BelowFloor { free, floor }) => {
            eprintln!("companiond: mission storage below floor ({free} < {floor}) — not logging");
            return;
        }
        Err(e) => {
            eprintln!("companiond: mission open failed: {e}");
            return;
        }
    };
    let mission_id = mission.mission_id();
    let cc_boot_id = mission.cc_boot_id();
    let mission_dir = mission.mission_dir().to_path_buf();
    eprintln!("companiond: logging mission {mission_id} → {}", mission_dir.display());

    // 3. PX4 param snapshot (Stub default; Real capture is a follow-up — the
    //    config field is present, and a stub keeps the safety-critical
    //    handshake window free of a PARAM_REQUEST_LIST flood).
    if cfg.handshake.param_snapshot != ParamSnapshotMode::Off {
        let snap = ParamSnapshot::stub(px4_boot_id, mission_id, env.clock.wall_unix_ns());
        if let Err(e) = snap.write_atomic(&mission_dir, &env.syncer) {
            eprintln!("companiond: param snapshot write failed: {e}");
        }
    }

    // 4. Spawn the single mission-log task (2nd broadcast subscriber + raw tap).
    let (log_shutdown_tx, log_shutdown_rx) = oneshot::channel();
    let log_join = cc_mission_log::task::spawn(
        mission,
        events.subscribe(),
        raw_rx,
        boot_rx.clone(),
        Duration::from_millis(500),
        log_shutdown_rx,
    );

    // 5. Stream CC_MISSION_CONTEXT at context_hz (P1) until shutdown.
    let period = Duration::from_secs_f64(1.0 / cfg.handshake.context_hz.max(0.1));
    let mut ctx_tick = tokio::time::interval(period);
    let ctx = MavMessage::CC_MISSION_CONTEXT(CC_MISSION_CONTEXT_DATA {
        mission_id,
        cc_boot_id,
        vehicle_id: cfg.general.vehicle_id,
        dialect_hash: dialect_hash::CC_DIALECT_HASH,
        sw_version: CharArray::<24>::from(sw_version.as_str()),
        schema_version: identity::CC_SCHEMA_VERSION,
    });
    loop {
        tokio::select! {
            _ = ctx_tick.tick() => tx.enqueue(Priority::P1, ctx.clone()),
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
        }
    }

    // 6. Clean shutdown: seal the final segment + mark complete, then wait.
    let _ = log_shutdown_tx.send(());
    let _ = log_join.await;
    eprintln!("companiond: mission {mission_id} finalized");
}
