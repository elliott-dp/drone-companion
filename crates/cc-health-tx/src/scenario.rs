//! The scripted severity timeline (dev-plan Phase 6.2): a config file of timed
//! events driving the companion's health conclusion. Pure and unit-tested; the
//! task layer samples it against elapsed time.
//!
//! ```toml
//! [[event]]
//! t_s = 0.0
//! severity = "ok"        # ok | warn | critical | stale
//! action = "none"        # none | warn_only | block_offboard | hold | land | rtl
//! flags = ["battery"]    # CC_HEALTH_FLAGS domains active
//! confidence = 100
//!
//! [[event]]
//! t_s = 120.0
//! severity = "critical"
//! action = "land"
//! flags = ["battery"]
//! confidence = 90
//! ```

use serde::Deserialize;

/// Companion severity (mirrors CC_SEVERITY).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Ok = 0,
    Warn = 1,
    Critical = 2,
    Stale = 3,
}

/// Advisory recommended action (mirrors CC_RECOMMENDED_ACTION).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None = 0,
    WarnOnly = 1,
    BlockOffboard = 2,
    Hold = 3,
    Land = 4,
    Rtl = 5,
}

/// One health-flag domain (CC_HEALTH_FLAGS bit).
fn flag_bit(name: &str) -> Option<u32> {
    Some(match name.to_ascii_lowercase().as_str() {
        "battery" => 1,
        "motor" => 2,
        "vibration" => 4,
        "gps" => 8,
        "estimator" => 16,
        "thermal" => 32,
        "link" => 64,
        "mission" => 128,
        "storage" => 256,
        "ai_degraded" => 512,
        "timesync" => 1024,
        _ => return None,
    })
}

fn parse_severity(s: &str) -> Result<Severity, String> {
    match s.to_ascii_lowercase().as_str() {
        "ok" => Ok(Severity::Ok),
        "warn" => Ok(Severity::Warn),
        "critical" => Ok(Severity::Critical),
        "stale" => Ok(Severity::Stale),
        other => Err(format!("bad severity {other:?}")),
    }
}

fn parse_action(s: &str) -> Result<Action, String> {
    match s.to_ascii_lowercase().as_str() {
        "none" => Ok(Action::None),
        "warn_only" | "warn" => Ok(Action::WarnOnly),
        "block_offboard" => Ok(Action::BlockOffboard),
        "hold" => Ok(Action::Hold),
        "land" => Ok(Action::Land),
        "rtl" => Ok(Action::Rtl),
        other => Err(format!("bad action {other:?}")),
    }
}

#[derive(Debug, Deserialize)]
struct RawEvent {
    t_s: f64,
    severity: String,
    #[serde(default = "default_action")]
    action: String,
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: u8,
}

fn default_action() -> String {
    "none".into()
}
fn default_confidence() -> u8 {
    100
}

#[derive(Debug, Deserialize)]
struct RawScenario {
    #[serde(default)]
    event: Vec<RawEvent>,
}

/// The health conclusion active at a moment in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub severity: Severity,
    pub action: Action,
    pub flags: u32,
    pub confidence: u8,
}

impl Default for Sample {
    fn default() -> Self {
        Sample { severity: Severity::Ok, action: Action::None, flags: 0, confidence: 100 }
    }
}

#[derive(Debug, Clone, Copy)]
struct Event {
    t_ns: i64,
    sample: Sample,
}

/// A parsed, time-sorted scenario.
#[derive(Debug, Clone)]
pub struct Scenario {
    events: Vec<Event>,
}

impl Scenario {
    pub fn from_toml_str(s: &str) -> Result<Scenario, String> {
        let raw: RawScenario = toml::from_str(s).map_err(|e| e.to_string())?;
        let mut events = Vec::with_capacity(raw.event.len());
        for e in raw.event {
            let mut flags = 0u32;
            for f in &e.flags {
                flags |= flag_bit(f).ok_or_else(|| format!("bad flag {f:?}"))?;
            }
            events.push(Event {
                t_ns: (e.t_s * 1e9) as i64,
                sample: Sample {
                    severity: parse_severity(&e.severity)?,
                    action: parse_action(&e.action)?,
                    flags,
                    confidence: e.confidence.min(100),
                },
            });
        }
        events.sort_by_key(|e| e.t_ns);
        Ok(Scenario { events })
    }

    /// A one-shot OK scenario (the default when no file is given).
    pub fn nominal() -> Scenario {
        Scenario {
            events: vec![Event { t_ns: 0, sample: Sample::default() }],
        }
    }

    /// The health conclusion at `elapsed_ns`: the last event whose time has
    /// passed (OK before the first event).
    pub fn sample_at(&self, elapsed_ns: i64) -> Sample {
        let mut cur = Sample::default();
        for e in &self.events {
            if e.t_ns <= elapsed_ns {
                cur = e.sample;
            } else {
                break;
            }
        }
        cur
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_step_through_timeline() {
        let s = Scenario::from_toml_str(
            r#"
            [[event]]
            t_s = 0.0
            severity = "ok"
            [[event]]
            t_s = 10.0
            severity = "critical"
            action = "land"
            flags = ["battery", "motor"]
            confidence = 90
        "#,
        )
        .unwrap();
        assert_eq!(s.sample_at(0).severity, Severity::Ok);
        assert_eq!(s.sample_at(9_000_000_000).severity, Severity::Ok);
        let c = s.sample_at(10_000_000_000);
        assert_eq!(c.severity, Severity::Critical);
        assert_eq!(c.action, Action::Land);
        assert_eq!(c.flags, 1 | 2); // battery|motor
        assert_eq!(c.confidence, 90);
    }

    #[test]
    fn out_of_order_events_are_sorted() {
        let s = Scenario::from_toml_str(
            "[[event]]\nt_s=10.0\nseverity=\"warn\"\n[[event]]\nt_s=0.0\nseverity=\"ok\"\n",
        )
        .unwrap();
        assert_eq!(s.sample_at(0).severity, Severity::Ok);
        assert_eq!(s.sample_at(10_000_000_000).severity, Severity::Warn);
    }

    #[test]
    fn bad_severity_and_flag_rejected() {
        assert!(Scenario::from_toml_str("[[event]]\nt_s=0\nseverity=\"nope\"\n").is_err());
        assert!(Scenario::from_toml_str("[[event]]\nt_s=0\nseverity=\"ok\"\nflags=[\"xyz\"]\n").is_err());
    }

    #[test]
    fn empty_before_first_event_is_ok() {
        let s = Scenario::from_toml_str("[[event]]\nt_s=5.0\nseverity=\"warn\"\n").unwrap();
        assert_eq!(s.sample_at(0).severity, Severity::Ok);
    }
}
