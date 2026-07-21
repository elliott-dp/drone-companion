//! `HealthFinding`, availability, and the deterministic **merge** into a single
//! `HealthConclusion` (which drives the `cc-health-tx` report).
//!
//! Two merge rules the adversarial panel corrected:
//!
//! * **Dominant-finding, not max-of-actions.** `detail_code`/`value`/`limit`/
//!   `confidence` come from the single highest-severity finding (tie-broken by a
//!   fixed subsystem priority), never a synthetic mix.
//! * **Cause → safest-action, not enum-max.** On the `Action` enum `Land=4 <
//!   Rtl=5`, but for a battery- or estimator-critical cause **Land is more
//!   conservative than RTL** (RTL spends energy and trusts navigation). Each
//!   algorithm already picks the safe action for its own cause; the merge then
//!   cross-checks: **RTL is only kept when GPS and the estimator are healthy** —
//!   otherwise it is downgraded, because RTL that trusts a bad estimate is
//!   unsafe.

use cc_health_tx::{Action, Severity};
use cc_protocol::cc_dialect::CcSubsystem;

/// CC_HEALTH_FLAGS bits.
pub mod flags {
    pub const BATTERY: u32 = 1;
    pub const MOTOR: u32 = 2;
    pub const VIBRATION: u32 = 4;
    pub const GPS: u32 = 8;
    pub const ESTIMATOR: u32 = 16;
    pub const THERMAL: u32 = 32;
    pub const LINK: u32 = 64;
    pub const MISSION: u32 = 128;
    pub const STORAGE: u32 = 256;
    pub const AI_DEGRADED: u32 = 512;
    pub const TIMESYNC: u32 = 1024;
}

/// One algorithm's verdict for a tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthFinding {
    pub subsystem: CcSubsystem,
    pub flag_bit: u32,
    pub severity: Severity,
    pub action: Action,
    pub detail_code: u16,
    pub value: f32,
    pub limit: f32,
    pub confidence: u8,
}

/// What an algorithm returns from `evaluate`.
#[derive(Debug, Clone, Copy)]
pub enum AlgoOutput {
    /// A trusted verdict (its severity may still be OK — a healthy finding).
    Available(HealthFinding),
    /// Data present but the lane cannot be trusted this tick (warmup, low
    /// excitation, …). Sets `ai_degraded`, never a domain finding. `u16` = reason.
    Degraded(u16),
    /// Required data absent/stale. Same effect as Degraded.
    Unavailable(u16),
}

/// The merged conclusion fed to the report source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthConclusion {
    pub severity: Severity,
    pub action: Action,
    pub health_flags: u32,
    pub detail_code: u16,
    pub value: f32,
    pub limit: f32,
    pub confidence: u8,
}

impl Default for HealthConclusion {
    fn default() -> Self {
        Self {
            severity: Severity::Ok,
            action: Action::None,
            health_flags: 0,
            detail_code: 0,
            value: 0.0,
            limit: 0.0,
            confidence: 100,
        }
    }
}

fn sev_rank(s: Severity) -> u8 {
    match s {
        Severity::Ok => 0,
        Severity::Warn => 1,
        Severity::Stale => 2,
        Severity::Critical => 3,
    }
}

/// Fixed subsystem priority for tie-breaking equal-severity findings (most
/// safety-critical first). Lower = higher priority.
fn subsystem_priority(s: CcSubsystem) -> u8 {
    use CcSubsystem::*;
    match s {
        CC_SUBSYS_BATTERY => 0,
        CC_SUBSYS_MOTOR => 1,
        CC_SUBSYS_ESTIMATOR => 2,
        CC_SUBSYS_GPS => 3,
        CC_SUBSYS_VIBRATION => 4,
        CC_SUBSYS_THERMAL => 5,
        CC_SUBSYS_LINK => 6,
        CC_SUBSYS_MISSION => 7,
        _ => 8,
    }
}

/// Merge algorithm outputs into one conclusion (pure, deterministic — iterate
/// in the caller's fixed AlgoId order).
pub fn merge(outputs: &[AlgoOutput], timesync_locked: bool) -> HealthConclusion {
    let mut any_degraded = false;
    let mut flags = 0u32;
    let mut dominant: Option<HealthFinding> = None;
    // remember whether nav-relevant subsystems are unhealthy (for the RTL gate)
    let mut gps_unhealthy = false;
    let mut estimator_unhealthy = false;

    for out in outputs {
        match out {
            AlgoOutput::Degraded(_) | AlgoOutput::Unavailable(_) => any_degraded = true,
            AlgoOutput::Available(f) => {
                if sev_rank(f.severity) > 0 {
                    flags |= f.flag_bit;
                    if f.subsystem == CcSubsystem::CC_SUBSYS_GPS {
                        gps_unhealthy = true;
                    }
                    if f.subsystem == CcSubsystem::CC_SUBSYS_ESTIMATOR {
                        estimator_unhealthy = true;
                    }
                    // dominant = highest severity, then subsystem priority
                    dominant = Some(match dominant {
                        None => *f,
                        Some(d) => {
                            let better = sev_rank(f.severity) > sev_rank(d.severity)
                                || (sev_rank(f.severity) == sev_rank(d.severity)
                                    && subsystem_priority(f.subsystem)
                                        < subsystem_priority(d.subsystem));
                            if better {
                                *f
                            } else {
                                d
                            }
                        }
                    });
                }
            }
        }
    }

    if any_degraded {
        flags |= flags::AI_DEGRADED;
    }
    if !timesync_locked {
        flags |= flags::TIMESYNC;
    }

    let mut c = HealthConclusion { health_flags: flags, ..Default::default() };
    if let Some(d) = dominant {
        c.severity = d.severity;
        c.detail_code = d.detail_code;
        c.value = d.value;
        c.limit = d.limit;
        c.confidence = d.confidence;
        // cause -> safest-action cross-check. Both RTL and position-Hold
        // (Loiter) *trust the navigation solution* — RTL flies a GPS course
        // home, Loiter holds a GPS/estimator position. When GPS or the
        // estimator is itself unhealthy, neither can be trusted, so both fall
        // back to Land: it lands promptly and relies only on the (independent)
        // height estimate, never on horizontal navigation.
        c.action = match d.action {
            Action::Rtl | Action::Hold if gps_unhealthy || estimator_unhealthy => Action::Land,
            other => other,
        };
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detail;

    fn finding(sub: CcSubsystem, bit: u32, sev: Severity, act: Action, code: u16) -> HealthFinding {
        HealthFinding {
            subsystem: sub,
            flag_bit: bit,
            severity: sev,
            action: act,
            detail_code: code,
            value: 1.0,
            limit: 2.0,
            confidence: 80,
        }
    }

    #[test]
    fn all_ok_is_ok_no_flags() {
        let outs = [
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_BATTERY, flags::BATTERY, Severity::Ok, Action::None, 0)),
        ];
        let c = merge(&outs, true);
        assert_eq!(c.severity, Severity::Ok);
        assert_eq!(c.health_flags, 0);
    }

    #[test]
    fn worst_severity_wins_and_dominates_detail() {
        let outs = [
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_GPS, flags::GPS, Severity::Warn, Action::WarnOnly, detail::GPS_EPH_HIGH)),
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_BATTERY, flags::BATTERY, Severity::Critical, Action::Land, detail::BATT_UNDERVOLTAGE_UNDER_LOAD)),
        ];
        let c = merge(&outs, true);
        assert_eq!(c.severity, Severity::Critical);
        assert_eq!(c.action, Action::Land);
        assert_eq!(c.detail_code, detail::BATT_UNDERVOLTAGE_UNDER_LOAD);
        // both non-OK flags OR'd
        assert_eq!(c.health_flags & flags::BATTERY, flags::BATTERY);
        assert_eq!(c.health_flags & flags::GPS, flags::GPS);
    }

    #[test]
    fn tie_breaks_by_subsystem_priority() {
        let outs = [
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_LINK, flags::LINK, Severity::Warn, Action::WarnOnly, detail::LINK_DROP_RATE_HIGH)),
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_BATTERY, flags::BATTERY, Severity::Warn, Action::WarnOnly, detail::BATT_SAG_BEYOND_MODEL)),
        ];
        let c = merge(&outs, true);
        // battery outranks link at equal severity
        assert_eq!(c.detail_code, detail::BATT_SAG_BEYOND_MODEL);
    }

    #[test]
    fn rtl_downgraded_when_nav_unhealthy() {
        // mission wants RTL and is the dominant (critical) finding; the
        // estimator is separately degraded (warn) -> RTL would trust a bad
        // estimate -> downgraded to Land.
        let outs = [
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_MISSION, flags::MISSION, Severity::Critical, Action::Rtl, detail::MISSION_POINT_OF_NO_RETURN)),
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_ESTIMATOR, flags::ESTIMATOR, Severity::Warn, Action::WarnOnly, detail::EST_VEL_BREACH)),
        ];
        let c = merge(&outs, true);
        assert_eq!(c.detail_code, detail::MISSION_POINT_OF_NO_RETURN, "mission is dominant");
        assert_eq!(c.action, Action::Land, "RTL unsafe with degraded estimator -> Land");
    }

    #[test]
    fn hold_downgraded_when_nav_unhealthy() {
        // Two equal criticals: mission (RTL) and GPS (Hold). GPS dominates by
        // subsystem priority, so the dominant action is Hold — but Loiter needs
        // a trustworthy position, which the critical GPS fault denies -> Land.
        let outs = [
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_MISSION, flags::MISSION, Severity::Critical, Action::Rtl, detail::MISSION_POINT_OF_NO_RETURN)),
            AlgoOutput::Available(finding(CcSubsystem::CC_SUBSYS_GPS, flags::GPS, Severity::Critical, Action::Hold, detail::GPS_FIX_DEGRADED)),
        ];
        let c = merge(&outs, true);
        assert_eq!(c.detail_code, detail::GPS_FIX_DEGRADED, "GPS dominates by priority");
        assert_eq!(c.action, Action::Land, "Hold unsafe with bad GPS -> Land");
    }

    #[test]
    fn degraded_and_timesync_set_flags() {
        let outs = [AlgoOutput::Degraded(detail::AVAIL_WARMUP)];
        let c = merge(&outs, false);
        assert_eq!(c.severity, Severity::Ok);
        assert_eq!(c.health_flags & flags::AI_DEGRADED, flags::AI_DEGRADED);
        assert_eq!(c.health_flags & flags::TIMESYNC, flags::TIMESYNC);
    }
}
