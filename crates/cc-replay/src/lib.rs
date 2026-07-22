//! `cc-replay` — re-run a recorded mission through the **same** deterministic
//! `cc-ai-health` Runner the live companion uses, and reproduce the exact
//! finding timeline (dev-plan Phase 7, Part C.2).
//!
//! This is the determinism *evidence* engine and the false-positive *audit*
//! engine:
//! * **run** — mission dir → finding timeline (JSON) + a canonical SHA-256.
//! * **diff** — two runs (e.g. two builds, or x86 CI vs aarch64 Jetson) → any
//!   byte difference in the canonical finding stream is a determinism failure.
//! * **audit** — aggregate WARN/CRITICAL rates over recorded **benign**
//!   missions; the gate on arming the algorithms (Part C.4) is ~zero here.
//!
//! The determinism guarantee is inherited, not re-implemented: [`read_mission`]
//! restores the exact live receive order, and `cc_ai_health::Runner::run_events`
//! folds it through the identical 10 Hz grid. Same bytes in → same bytes out.

pub mod reader;

pub use reader::{read_mission, ReplayError};

use cc_ai_health::algos::default_registry;
use cc_ai_health::finding::HealthConclusion;
use cc_ai_health::Runner;
use cc_health_tx::{Action, Severity};
use sha2::{Digest, Sha256};
use std::path::Path;

/// One row of the finding timeline: the merged conclusion at a 10 Hz boundary.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FindingRow {
    pub tick_ns: i64,
    pub severity: &'static str,
    pub action: &'static str,
    pub health_flags: u32,
    pub detail_code: u16,
    pub value: f32,
    pub limit: f32,
    pub confidence: u8,
}

fn sev_str(s: Severity) -> &'static str {
    match s {
        Severity::Ok => "ok",
        Severity::Warn => "warn",
        Severity::Critical => "critical",
        Severity::Stale => "stale",
    }
}
fn act_str(a: Action) -> &'static str {
    match a {
        Action::None => "none",
        Action::WarnOnly => "warn_only",
        Action::BlockOffboard => "block_offboard",
        Action::Hold => "hold",
        Action::Land => "land",
        Action::Rtl => "rtl",
    }
}

impl FindingRow {
    fn from_conclusion(tick_ns: i64, c: &HealthConclusion) -> Self {
        Self {
            tick_ns,
            severity: sev_str(c.severity),
            action: act_str(c.action),
            health_flags: c.health_flags,
            detail_code: c.detail_code,
            value: c.value,
            limit: c.limit,
            confidence: c.confidence,
        }
    }
    /// Fold this row into a canonical hasher (float bits, not text — bit-exact).
    fn hash_into(&self, h: &mut Sha256) {
        h.update(self.tick_ns.to_le_bytes());
        h.update(self.severity.as_bytes());
        h.update([0u8]);
        h.update(self.action.as_bytes());
        h.update([0u8]);
        h.update(self.health_flags.to_le_bytes());
        h.update(self.detail_code.to_le_bytes());
        h.update(self.value.to_bits().to_le_bytes());
        h.update(self.limit.to_bits().to_le_bytes());
        h.update([self.confidence]);
    }
}

/// The full finding timeline for one mission.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Timeline {
    pub rows: Vec<FindingRow>,
}

impl Timeline {
    /// Canonical SHA-256 over the whole finding stream — the determinism token.
    pub fn hash(&self) -> String {
        let mut h = Sha256::new();
        h.update((self.rows.len() as u64).to_le_bytes());
        for r in &self.rows {
            r.hash_into(&mut h);
        }
        let d = h.finalize();
        let mut s = String::with_capacity(64);
        for b in d {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// Rows whose severity is WARN or CRITICAL (the audit's numerator).
    pub fn findings(&self) -> impl Iterator<Item = &FindingRow> {
        self.rows.iter().filter(|r| r.severity == "warn" || r.severity == "critical")
    }
}

/// Drive a pre-ordered event stream through a fresh Runner → timeline.
pub fn replay_events(events: &[cc_ingest::TelemetryEvent]) -> Timeline {
    let mut runner = Runner::new(default_registry());
    let rows = runner
        .run_events(events)
        .into_iter()
        .map(|(t, c)| FindingRow::from_conclusion(t, &c))
        .collect();
    Timeline { rows }
}

/// Read a mission directory and replay it → timeline.
pub fn run_mission(mission_dir: &Path) -> Result<Timeline, ReplayError> {
    let events = read_mission(mission_dir)?;
    Ok(replay_events(&events))
}

/// Compare two timelines; returns human-readable differences (empty = identical).
pub fn diff(a: &Timeline, b: &Timeline) -> Vec<String> {
    let mut diffs = Vec::new();
    if a.rows.len() != b.rows.len() {
        diffs.push(format!("row count differs: {} vs {}", a.rows.len(), b.rows.len()));
    }
    for (i, (ra, rb)) in a.rows.iter().zip(b.rows.iter()).enumerate() {
        if ra != rb {
            diffs.push(format!("tick {i} @ {}ns differs: {ra:?} vs {rb:?}", ra.tick_ns));
            if diffs.len() >= 20 {
                diffs.push("… (truncated)".into());
                break;
            }
        }
    }
    diffs
}

/// Aggregate false-positive statistics over one or more (benign) timelines.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct AuditStats {
    pub missions: usize,
    pub ticks: usize,
    pub warn_ticks: usize,
    pub critical_ticks: usize,
    /// `(detail_code, count)` for every non-OK finding, sorted by code.
    pub by_detail: Vec<(u16, usize)>,
}

impl AuditStats {
    pub fn warn_rate(&self) -> f64 {
        if self.ticks == 0 {
            0.0
        } else {
            (self.warn_ticks + self.critical_ticks) as f64 / self.ticks as f64
        }
    }
}

/// Build audit statistics from a set of timelines.
pub fn audit(timelines: &[Timeline]) -> AuditStats {
    let mut st = AuditStats { missions: timelines.len(), ..Default::default() };
    let mut hist: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for tl in timelines {
        st.ticks += tl.rows.len();
        for r in tl.rows.iter() {
            match r.severity {
                "warn" => st.warn_ticks += 1,
                "critical" => st.critical_ticks += 1,
                _ => continue,
            }
            *hist.entry(r.detail_code).or_default() += 1;
        }
    }
    st.by_detail = hist.into_iter().collect();
    st
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::{AgeInfo, RxMeta, TelemetryEvent};
    use cc_protocol::cc_dialect::CC_TELEMETRY_POWER_DATA;

    fn power(cc_ns: i64) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            voltage: 15.8,
            current: 10.0,
            remaining: 0.7,
            temperature: 30.0,
            cell_count: 4,
            connected: 1,
            schema_version: 1,
            ..Default::default()
        };
        TelemetryEvent::Power(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    #[test]
    fn replay_hash_is_stable_and_diff_clean() {
        let events: Vec<_> = (0..200).map(|i| power(i * 100_000_000 + 3_000_000)).collect();
        let a = replay_events(&events);
        let b = replay_events(&events);
        assert_eq!(a.hash(), b.hash(), "same events → same hash");
        assert!(diff(&a, &b).is_empty(), "same events → no diff");
        assert!(!a.rows.is_empty());
    }

    #[test]
    fn audit_of_benign_stream_has_zero_findings() {
        let events: Vec<_> = (0..200).map(|i| power(i * 100_000_000 + 3_000_000)).collect();
        let tl = replay_events(&events);
        let st = audit(std::slice::from_ref(&tl));
        // a bare power stream never arms a detector (no arm/flight) → no findings
        assert_eq!(st.warn_ticks + st.critical_ticks, 0);
        assert_eq!(st.warn_rate(), 0.0);
    }
}
