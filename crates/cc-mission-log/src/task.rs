//! The single disk-touching async task.
//!
//! It is the **only** writer of the mission dataset and a *lossy* subscriber
//! of the telemetry broadcast: if a slow disk makes it lag, the broadcast
//! drops the oldest events (counted as `lagged`) rather than back-pressuring
//! the RX path (spec §5.2). It also owns the raw-frame tap, the PX4 boot-id
//! watch (segment rotation), and the periodic tick (time-cap seals, disk
//! polling, size/time rotation).

use std::time::Duration;

use cc_ingest::TelemetryEvent;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::Mission;

/// Spawn the mission-log task, consuming the opened [`Mission`]. Fire
/// `shutdown` for a clean finalize (manifest marked complete); the returned
/// handle completes once finalize has run.
pub fn spawn(
    mut mission: Mission,
    mut events_rx: broadcast::Receiver<TelemetryEvent>,
    mut raw_rx: mpsc::Receiver<Vec<u8>>,
    mut boot_rx: watch::Receiver<u32>,
    tick_period: Duration,
    mut shutdown: oneshot::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick_period);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;

                // Clean shutdown wins over draining more events.
                _ = &mut shutdown => break,

                res = events_rx.recv() => match res {
                    Ok(ev) => mission.on_event(&ev),
                    Err(broadcast::error::RecvError::Lagged(n)) => mission.note_lag(n),
                    Err(broadcast::error::RecvError::Closed) => break,
                },

                maybe = raw_rx.recv() => {
                    match maybe {
                        Some(frame) => mission.on_raw(&frame),
                        None => { /* raw tap closed; keep logging telemetry */ }
                    }
                }

                changed = boot_rx.changed() => {
                    if changed.is_ok() {
                        let b = *boot_rx.borrow();
                        let _ = mission.on_boot_change(b);
                    }
                }

                _ = ticker.tick() => {
                    let _ = mission.tick();
                }
            }
        }

        // Seal the final segment and mark the mission complete.
        let _ = mission.finalize();
    })
}
