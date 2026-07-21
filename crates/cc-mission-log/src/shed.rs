//! The disk-shedding ladder: a pure state machine mapping free-space to a
//! [`ShedStage`] with hysteresis, and each stage to per-stream write/drop
//! decisions.
//!
//! Spec ordering (§5.6): shed **raw first**, then Class B (IMU) + Class F
//! (Actuator), then Class C/D/E (Power/GPS/Estimator) — but **never** State,
//! Event, or SafetyStatus. The system therefore never silently stops writing
//! everything: at the deepest stage it still lands State + Event + Safety
//! rows and the operational drop log.
//!
//! Escalation (free falling) is immediate and may skip stages; de-escalation
//! (free rising) is one step per update and gated on a higher *resume*
//! threshold, so the ladder hystereses instead of chattering.

use cc_config::Disk;
use cc_ingest::StreamId;

/// Ladder stages, ordered `Normal < ShedRaw < ShedBf < ShedCrit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShedStage {
    Normal = 0,
    ShedRaw = 1,
    ShedBf = 2,
    ShedCrit = 3,
}

impl ShedStage {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn name(self) -> &'static str {
        match self {
            ShedStage::Normal => "NORMAL",
            ShedStage::ShedRaw => "SHED_RAW",
            ShedStage::ShedBf => "SHED_BF",
            ShedStage::ShedCrit => "SHED_CRIT",
        }
    }
}

/// Stateful ladder. Holds the thresholds and the current stage.
#[derive(Debug, Clone)]
pub struct ShedLadder {
    disk: Disk,
    stage: ShedStage,
}

impl ShedLadder {
    pub fn new(disk: Disk) -> Self {
        Self { disk, stage: ShedStage::Normal }
    }

    pub fn stage(&self) -> ShedStage {
        self.stage
    }

    /// Feed a fresh free-space reading; returns the (possibly changed) stage.
    pub fn update(&mut self, free_bytes: u64) -> ShedStage {
        let worst = self.worst_triggered(free_bytes);
        self.stage = if worst > self.stage {
            worst // escalate immediately, possibly skipping stages
        } else {
            self.recover_one_step(free_bytes) // hysteretic, one step
        };
        self.stage
    }

    /// The deepest stage whose shed-low threshold `free` has fallen below.
    fn worst_triggered(&self, free: u64) -> ShedStage {
        if free < self.disk.crit_low_bytes {
            ShedStage::ShedCrit
        } else if free < self.disk.bf_shed_low_bytes {
            ShedStage::ShedBf
        } else if free < self.disk.raw_shed_low_bytes {
            ShedStage::ShedRaw
        } else {
            ShedStage::Normal
        }
    }

    /// Recover at most one stage, only once free clears the current stage's
    /// resume threshold.
    fn recover_one_step(&self, free: u64) -> ShedStage {
        match self.stage {
            ShedStage::ShedCrit if free >= self.disk.crit_resume_bytes => ShedStage::ShedBf,
            ShedStage::ShedBf if free >= self.disk.bf_resume_bytes => ShedStage::ShedRaw,
            ShedStage::ShedRaw if free >= self.disk.raw_resume_bytes => ShedStage::Normal,
            other => other,
        }
    }

    /// Whether the raw_mavlink.bin capture may append at the current stage.
    pub fn raw_allowed(&self) -> bool {
        self.stage == ShedStage::Normal
    }

    /// Whether a telemetry stream may be written at the current stage.
    /// State, Event and SafetyStatus are the never-shed classes.
    pub fn stream_allowed(&self, s: StreamId) -> bool {
        stage_allows(self.stage, s)
    }
}

/// Pure stage→stream decision (also used by tests directly).
pub fn stage_allows(stage: ShedStage, s: StreamId) -> bool {
    // Never-shed classes: State (A), Event (G), SafetyStatus.
    if matches!(s, StreamId::State | StreamId::Event | StreamId::SafetyStatus) {
        return true;
    }
    match stage {
        ShedStage::Normal | ShedStage::ShedRaw => true,
        // B (Imu) + F (Actuator) shed here.
        ShedStage::ShedBf => !matches!(s, StreamId::Imu | StreamId::Actuator),
        // Also C/D/E (Power/Gps/Estimator) shed at the deepest stage.
        ShedStage::ShedCrit => !matches!(
            s,
            StreamId::Imu | StreamId::Actuator | StreamId::Power | StreamId::Gps | StreamId::Estimator
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk() -> Disk {
        // Compact thresholds for readable assertions (well-ordered + hysteretic).
        Disk {
            floor_bytes: 100,
            raw_shed_low_bytes: 40,
            raw_resume_bytes: 50,
            bf_shed_low_bytes: 30,
            bf_resume_bytes: 35,
            crit_low_bytes: 20,
            crit_resume_bytes: 25,
        }
    }

    #[test]
    fn full_ladder_walk_down_and_up() {
        let mut l = ShedLadder::new(disk());
        assert_eq!(l.update(100), ShedStage::Normal);
        assert_eq!(l.update(39), ShedStage::ShedRaw); // < raw_shed_low 40
        assert_eq!(l.update(29), ShedStage::ShedBf); // < bf_shed_low 30
        assert_eq!(l.update(19), ShedStage::ShedCrit); // < crit_low 20
        // recovery is one step at a time, gated on resume thresholds
        assert_eq!(l.update(26), ShedStage::ShedBf); // >= crit_resume 25
        assert_eq!(l.update(36), ShedStage::ShedRaw); // >= bf_resume 35
        assert_eq!(l.update(51), ShedStage::Normal); // >= raw_resume 50
    }

    #[test]
    fn escalation_can_skip_stages() {
        let mut l = ShedLadder::new(disk());
        assert_eq!(l.update(100), ShedStage::Normal);
        // free craters straight past every threshold in one reading
        assert_eq!(l.update(5), ShedStage::ShedCrit);
    }

    #[test]
    fn hysteresis_holds_between_shed_and_resume() {
        let mut l = ShedLadder::new(disk());
        l.update(39); // ShedRaw
        // free bounces back into the dead-band (>= shed_low 40 but < resume 50)
        assert_eq!(l.update(45), ShedStage::ShedRaw, "must not recover before resume");
        assert_eq!(l.update(50), ShedStage::Normal, "recovers at resume");
    }

    #[test]
    fn never_shed_state_event_safety_at_any_stage() {
        for stage in [ShedStage::Normal, ShedStage::ShedRaw, ShedStage::ShedBf, ShedStage::ShedCrit] {
            assert!(stage_allows(stage, StreamId::State), "state at {stage:?}");
            assert!(stage_allows(stage, StreamId::Event), "event at {stage:?}");
            assert!(stage_allows(stage, StreamId::SafetyStatus), "safety at {stage:?}");
        }
    }

    #[test]
    fn exact_shed_set_per_stage() {
        // NORMAL / SHED_RAW: all telemetry written
        for s in StreamId::ALL {
            assert!(stage_allows(ShedStage::Normal, s));
            assert!(stage_allows(ShedStage::ShedRaw, s));
        }
        // SHED_BF: imu + actuator dropped, rest kept
        assert!(!stage_allows(ShedStage::ShedBf, StreamId::Imu));
        assert!(!stage_allows(ShedStage::ShedBf, StreamId::Actuator));
        assert!(stage_allows(ShedStage::ShedBf, StreamId::Power));
        assert!(stage_allows(ShedStage::ShedBf, StreamId::Gps));
        assert!(stage_allows(ShedStage::ShedBf, StreamId::Estimator));
        // SHED_CRIT: also power/gps/estimator dropped
        assert!(!stage_allows(ShedStage::ShedCrit, StreamId::Power));
        assert!(!stage_allows(ShedStage::ShedCrit, StreamId::Gps));
        assert!(!stage_allows(ShedStage::ShedCrit, StreamId::Estimator));
        assert!(!stage_allows(ShedStage::ShedCrit, StreamId::Imu));
        assert!(!stage_allows(ShedStage::ShedCrit, StreamId::Actuator));
    }

    #[test]
    fn raw_allowed_only_in_normal() {
        let mut l = ShedLadder::new(disk());
        assert!(l.raw_allowed());
        l.update(39);
        assert!(!l.raw_allowed(), "raw sheds first, before any telemetry");
    }
}
