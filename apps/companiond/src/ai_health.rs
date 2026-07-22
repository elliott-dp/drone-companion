//! The live `cc-ai-health` driver task (dev-plan Phase 7, deviation D1).
//!
//! A **third, lossy broadcast subscriber** (after cc-ingest's own consumers and
//! the mission-log task): it folds every `TelemetryEvent` into the deterministic
//! [`Runner`], evaluates the eight algorithms on a 100 ms wall-clock tick, and
//! publishes the merged [`cc_health_tx::Conclusion`] on a `watch` channel that
//! [`cc_health_tx::spawn_ai`] turns into paced `CC_HEALTH_REPORT`s.
//!
//! Determinism note: this live driver ticks on the real-time interval, so its
//! findings are *not* required to be byte-identical to a later replay (which
//! ticks on the recorded event-time grid). The byte-identity guarantee is a
//! property of `cc-replay` re-running a *recording* — the exit criterion — not
//! of the live vs replayed comparison. Being a lossy subscriber, a lag drop
//! only means the runner missed some samples, exactly like any real link gap.

use cc_ai_health::algos::default_registry;
use cc_ai_health::finding::HealthConclusion;
use cc_ai_health::Runner;
use cc_ingest::TelemetryEvent;
use cc_link::clock;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

/// The 10 Hz evaluation cadence (matches [`cc_ai_health::TICK_NS`]).
const TICK: std::time::Duration = std::time::Duration::from_millis(100);

fn to_tx(c: &HealthConclusion) -> cc_health_tx::Conclusion {
    cc_health_tx::Conclusion {
        severity: c.severity,
        action: c.action,
        health_flags: c.health_flags,
        detail_code: c.detail_code,
        confidence: c.confidence,
    }
}

/// Spawn the AI-health driver. Returns a `watch::Receiver<Conclusion>` for the
/// report source and the task handle.
pub fn spawn(
    mut events: broadcast::Receiver<TelemetryEvent>,
) -> (watch::Receiver<cc_health_tx::Conclusion>, JoinHandle<()>) {
    let (conc_tx, conc_rx) = watch::channel(cc_health_tx::Conclusion::default());
    let handle = tokio::spawn(async move {
        let mut runner = Runner::new(default_registry());
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                r = events.recv() => match r {
                    Ok(ev) => runner.on_event(&ev),
                    // lossy subscriber: a lag is a dropped sample, keep going
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                _ = ticker.tick() => {
                    let conclusion = runner.tick(clock::now_ns());
                    // ignore send error (no receiver = report task gone → exit)
                    if conc_tx.send(to_tx(&conclusion)).is_err() {
                        break;
                    }
                }
            }
        }
    });
    (conc_rx, handle)
}
