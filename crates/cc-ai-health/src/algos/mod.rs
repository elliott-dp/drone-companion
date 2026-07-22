//! The eight health-detection algorithms (dev-plan Phase 7, Part B).
//!
//! Every algorithm is a self-contained [`crate::HealthAlgorithm`]: it folds its
//! own streaming estimators in `on_event` and reads them in `evaluate`. There is
//! **no shared window buffer** — each detector keeps exactly the O(1) state it
//! needs (an EWMA, a robust ring, a CUSUM, an RLS), which is both cheaper and a
//! cleaner determinism story than a common ring the algorithms would index into
//! (simplification vs the blueprint's optional `window.rs`, noted in Part C).
//!
//! Shared discipline (all eight):
//! * **Warm-up before any finding** — an algorithm returns `Degraded(warmup)`
//!   until it has enough samples/time to have a baseline.
//! * **Flight-phase gate** — adaptive baselines update **only** when
//!   `phase.is_steady()`; detectors are suppressed outside Steady (the master
//!   false-positive defence, [`crate::phase`]).
//! * **Freeze-on-anomaly** — once a detector trips, its baseline stops adapting
//!   so the anomaly cannot be learned as the new normal.
//! * **NaN = missing** — a NaN input never becomes a fault; it degrades the lane.
//! * **Advisory until audited** — every CRITICAL here is advisory until the
//!   benign-corpus false-positive audit (Part C.4) passes; the FC monitor stays
//!   warn-only on these lanes meanwhile.

use crate::{EvalCtx, HealthAlgorithm};
use cc_ingest::StreamId;

pub mod battery;
pub mod estimator;
pub mod gps;
pub mod link;
pub mod mission;
pub mod motor;
pub mod thermal;
pub mod vibration;

/// A required stream is *fresh* if it was seen within 4× its nominal period
/// (the ingest watchdog rule, spec §5.5). Event-driven streams (no nominal
/// period) count as fresh once seen at all.
pub(crate) fn fresh(ctx: &EvalCtx, sid: StreamId) -> bool {
    match sid.nominal_period_ns() {
        Some(p) => ctx.stream_fresh(sid, 4 * p),
        None => ctx.stream_seen(sid),
    }
}

/// Build the fixed-order algorithm registry the [`crate::Runner`] evaluates and
/// merges each tick. Order is deterministic; `merge` is order-independent for
/// the conclusion but a fixed order keeps `CC_AI_DIAGNOSTIC` round-robin stable.
pub fn default_registry() -> Vec<Box<dyn HealthAlgorithm>> {
    vec![
        Box::new(battery::BatteryModel::new()),
        Box::new(vibration::VibrationAnomaly::new()),
        Box::new(estimator::EstimatorConsistency::new()),
        Box::new(gps::GpsQuality::new()),
        Box::new(motor::MotorBalance::new()),
        Box::new(link::LinkQuality::new()),
        Box::new(thermal::ThermalMonitor::new()),
        Box::new(mission::MissionRisk::new()),
    ]
}
