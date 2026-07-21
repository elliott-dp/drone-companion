//! Report rate policy + severity hysteresis (pure, unit-tested).
//!
//! Rate policy (spec §CC_HEALTH_REPORT, dev-plan 6.2): OK 1 Hz; WARN 2–5 Hz;
//! CRITICAL immediate on transition then 5 Hz **until acknowledged** by the
//! monitor (CC_SAFETY_STATUS.last_report_sequence catching up to our latest
//! sent sequence), after which it drops to a 1 Hz keepalive while the condition
//! persists. STALE 1 Hz.
//!
//! Hysteresis: escalation is immediate; de-escalation to a lower severity is
//! held for `debounce_ns` of continuous lower input so a flapping source does
//! not oscillate the reported severity (the monitor has its own OK_COUNT
//! hysteresis on top; the two are set so they never fight).

use crate::scenario::Severity;

/// Report interval for a severity, in nanoseconds.
pub fn interval_ns(sev: Severity, acked: bool) -> i64 {
    match sev {
        Severity::Critical => {
            if acked {
                1_000_000_000 // 1 Hz keepalive once the monitor has acked
            } else {
                200_000_000 // 5 Hz until acknowledged
            }
        }
        Severity::Warn => 250_000_000,          // 4 Hz (within 2–5 Hz)
        Severity::Ok | Severity::Stale => 1_000_000_000, // 1 Hz
    }
}

/// Severity de-escalation debounce.
#[derive(Debug, Clone)]
pub struct Hysteresis {
    debounce_ns: i64,
    current: Severity,
    // when the (lower) candidate first appeared; None while stable/escalating
    pending_since: Option<(Severity, i64)>,
}

impl Hysteresis {
    pub fn new(debounce_ns: i64) -> Self {
        Self { debounce_ns, current: Severity::Ok, pending_since: None }
    }

    pub fn current(&self) -> Severity {
        self.current
    }

    /// Feed the raw scenario severity at time `now_ns`; returns the smoothed
    /// severity to report.
    pub fn apply(&mut self, raw: Severity, now_ns: i64) -> Severity {
        if raw >= self.current {
            // escalation (or same): immediate, clears any pending de-escalation
            self.current = raw;
            self.pending_since = None;
        } else {
            // de-escalation candidate: adopt only after it holds for debounce_ns
            match self.pending_since {
                Some((sev, since)) if sev == raw => {
                    if now_ns - since >= self.debounce_ns {
                        self.current = raw;
                        self.pending_since = None;
                    }
                }
                _ => self.pending_since = Some((raw, now_ns)),
            }
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_rate_is_5hz_until_acked_then_1hz() {
        assert_eq!(interval_ns(Severity::Critical, false), 200_000_000);
        assert_eq!(interval_ns(Severity::Critical, true), 1_000_000_000);
        assert_eq!(interval_ns(Severity::Ok, false), 1_000_000_000);
        assert!(interval_ns(Severity::Warn, false) <= 500_000_000); // >= 2 Hz
        assert!(interval_ns(Severity::Warn, false) >= 200_000_000); // <= 5 Hz
    }

    #[test]
    fn escalation_immediate_deescalation_debounced() {
        let mut h = Hysteresis::new(1_000_000_000); // 1 s debounce
        assert_eq!(h.apply(Severity::Ok, 0), Severity::Ok);
        // escalate immediately
        assert_eq!(h.apply(Severity::Critical, 1_000_000_000), Severity::Critical);
        // OK candidate appears but must hold for 1 s
        assert_eq!(h.apply(Severity::Ok, 1_500_000_000), Severity::Critical);
        assert_eq!(h.apply(Severity::Ok, 2_000_000_000), Severity::Critical); // 0.5s < 1s
        assert_eq!(h.apply(Severity::Ok, 2_600_000_000), Severity::Ok); // >= 1s held
    }

    #[test]
    fn flap_resets_deescalation_timer() {
        let mut h = Hysteresis::new(1_000_000_000);
        h.apply(Severity::Critical, 0);
        h.apply(Severity::Ok, 1_000_000_000); // pending OK
        // a CRITICAL blip re-escalates and clears pending
        assert_eq!(h.apply(Severity::Critical, 1_500_000_000), Severity::Critical);
        // OK must restart its full debounce
        assert_eq!(h.apply(Severity::Ok, 2_000_000_000), Severity::Critical);
        assert_eq!(h.apply(Severity::Ok, 3_100_000_000), Severity::Ok);
    }
}
