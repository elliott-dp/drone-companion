//! The shared **flight-phase gate** — the single strongest false-positive
//! defence (adversarial-panel consensus).
//!
//! Aggressive-but-healthy flight legitimately drives vibration, innovation
//! ratios, output asymmetry and current draw well outside their calm-hover
//! ranges. Rather than each algorithm re-deriving "is this a real anomaly or
//! just a hard maneuver", the runner computes ONE flight phase from the State
//! stream and every detector consults it: anomaly detection is **suppressed by
//! construction** in transient / maneuver phases, and adaptive baselines update
//! **only** in a steady phase (so a maneuver can neither trip nor poison a
//! detector).
//!
//! The maneuver boundary is **hysteretic** with a required dwell — the review
//! flagged that a bare threshold leaves an unclassified dead-band that a gusty
//! hover chatters across. Enter `Maneuver` at `|ω| > ENTER` held for `DWELL`
//! frames; leave only below `LEAVE (< ENTER)` held for `DWELL` frames.
//!
//! Determinism: driven only by State-stream fields + `cc_receive_time_ns`
//! (integer). No wall clock.

/// Coarse flight phase. `is_steady()` gates baseline updates; anomaly detectors
/// are suppressed unless steady.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightPhase {
    /// Not armed — no flight anomalies are meaningful.
    Disarmed,
    /// Armed but within the post-arm transient window (takeoff spin-up).
    Transient,
    /// Aggressive attitude dynamics — detectors suppressed.
    Maneuver,
    /// Quasi-steady flight (hover or steady cruise) — detectors active.
    Steady,
}

impl FlightPhase {
    /// True when anomaly detectors must NOT produce a finding (and baselines
    /// must NOT adapt).
    pub fn suppresses(self) -> bool {
        !matches!(self, FlightPhase::Steady)
    }
    pub fn is_steady(self) -> bool {
        matches!(self, FlightPhase::Steady)
    }
}

// PX4 vehicle_status arming_state: 2 = ARMED.
const ARMING_STATE_ARMED: u8 = 2;

/// Rotation-rate hysteresis (rad/s) and dwell (consecutive State frames).
const OMEGA_ENTER: f64 = 0.35;
const OMEGA_LEAVE: f64 = 0.25;
const DWELL_FRAMES: u32 = 5;
/// Post-arm transient suppression window.
const TRANSIENT_NS: i64 = 3_000_000_000; // 3 s

/// Stateful classifier fed from the State stream.
#[derive(Debug, Clone)]
pub struct FlightPhaseTracker {
    phase: FlightPhase,
    armed_since_ns: Option<i64>,
    in_maneuver: bool,
    over_frames: u32,  // consecutive frames |ω| > ENTER
    under_frames: u32, // consecutive frames |ω| < LEAVE
    horizontal_speed: f64,
}

impl Default for FlightPhaseTracker {
    fn default() -> Self {
        Self {
            phase: FlightPhase::Disarmed,
            armed_since_ns: None,
            in_maneuver: false,
            over_frames: 0,
            under_frames: 0,
            horizontal_speed: 0.0,
        }
    }
}

impl FlightPhaseTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update from a State sample: `cc_ns` receive time, `arming_state`,
    /// angular-velocity magnitude `omega_mag` (rad/s), horizontal speed
    /// `h_speed` (m/s). `NaN` kinematics are treated as maneuver (conservative:
    /// suppress rather than risk a bad classification).
    pub fn on_state(&mut self, cc_ns: i64, arming_state: u8, omega_mag: f64, h_speed: f64) {
        self.horizontal_speed = if h_speed.is_nan() { 0.0 } else { h_speed };

        if arming_state != ARMING_STATE_ARMED {
            self.phase = FlightPhase::Disarmed;
            self.armed_since_ns = None;
            self.in_maneuver = false;
            self.over_frames = 0;
            self.under_frames = 0;
            return;
        }

        // note the arm edge for the transient window
        let armed_since = *self.armed_since_ns.get_or_insert(cc_ns);

        // hysteretic maneuver detection with dwell
        let omega = if omega_mag.is_nan() { f64::INFINITY } else { omega_mag };
        if omega > OMEGA_ENTER {
            self.over_frames = self.over_frames.saturating_add(1);
            self.under_frames = 0;
            if self.over_frames >= DWELL_FRAMES {
                self.in_maneuver = true;
            }
        } else if omega < OMEGA_LEAVE {
            self.under_frames = self.under_frames.saturating_add(1);
            self.over_frames = 0;
            if self.under_frames >= DWELL_FRAMES {
                self.in_maneuver = false;
            }
        } else {
            // dead-band: hold both counters so neither transition fires
            self.over_frames = 0;
            self.under_frames = 0;
        }

        self.phase = if cc_ns.saturating_sub(armed_since) < TRANSIENT_NS {
            FlightPhase::Transient
        } else if self.in_maneuver {
            FlightPhase::Maneuver
        } else {
            FlightPhase::Steady
        };
    }

    pub fn phase(&self) -> FlightPhase {
        self.phase
    }

    /// Steady + moving forward → cruise (vs hover). Some algorithms scope their
    /// baselines to cruise only.
    pub fn is_cruise(&self) -> bool {
        self.phase == FlightPhase::Steady && self.horizontal_speed > 2.0
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(t: &mut FlightPhaseTracker, cc: i64, arm: u8, w: f64, v: f64, n: usize) {
        for i in 0..n {
            t.on_state(cc + i as i64 * 40_000_000, arm, w, v);
        }
    }

    #[test]
    fn disarmed_then_transient_then_steady() {
        let mut t = FlightPhaseTracker::new();
        t.on_state(0, 0, 0.0, 0.0);
        assert_eq!(t.phase(), FlightPhase::Disarmed);
        // arm at t=1e9; within 3 s → Transient
        t.on_state(1_000_000_000, 2, 0.0, 0.0);
        assert_eq!(t.phase(), FlightPhase::Transient);
        // after the transient window, low ω → Steady
        t.on_state(5_000_000_000, 2, 0.05, 0.0);
        assert_eq!(t.phase(), FlightPhase::Steady);
    }

    #[test]
    fn maneuver_hysteresis_with_dwell() {
        let mut t = FlightPhaseTracker::new();
        // arm and pass the transient window at steady
        t.on_state(0, 2, 0.05, 0.0);
        feed(&mut t, 4_000_000_000, 2, 0.05, 0.0, 3);
        assert_eq!(t.phase(), FlightPhase::Steady);
        // a single high-ω frame does NOT enter maneuver (needs DWELL)
        t.on_state(5_000_000_000, 2, 0.5, 0.0);
        assert_eq!(t.phase(), FlightPhase::Steady);
        // sustained high ω → Maneuver
        feed(&mut t, 6_000_000_000, 2, 0.5, 0.0, DWELL_FRAMES as usize);
        assert_eq!(t.phase(), FlightPhase::Maneuver);
        // dead-band (0.3) does NOT immediately leave
        t.on_state(7_000_000_000, 2, 0.3, 0.0);
        assert_eq!(t.phase(), FlightPhase::Maneuver);
        // sustained low ω → back to Steady
        feed(&mut t, 8_000_000_000, 2, 0.05, 0.0, DWELL_FRAMES as usize);
        assert_eq!(t.phase(), FlightPhase::Steady);
    }

    #[test]
    fn disarm_resets_and_suppresses() {
        let mut t = FlightPhaseTracker::new();
        t.on_state(0, 2, 0.05, 0.0); // arm edge at t=0
        feed(&mut t, 4_000_000_000, 2, 0.05, 0.0, 3); // past the transient window
        assert!(t.phase().is_steady());
        t.on_state(9_000_000_000, 0, 0.0, 0.0);
        assert!(t.phase().suppresses());
    }

    #[test]
    fn cruise_vs_hover() {
        let mut t = FlightPhaseTracker::new();
        t.on_state(0, 2, 0.05, 8.0); // arm edge at t=0
        feed(&mut t, 4_000_000_000, 2, 0.05, 8.0, 3); // past the transient window
        assert!(t.phase().is_steady());
        assert!(t.is_cruise());
    }
}
