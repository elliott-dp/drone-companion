//! Cross-field invariants, run once after the layers are merged.

use crate::model::{Config, TransportKind};
use crate::ConfigError;

impl Config {
    /// Reject configurations that are internally inconsistent or unsafe.
    /// Every check has a matching unit test in `lib.rs`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.general.vehicle_id == 0 {
            return Err(ConfigError::Value(
                "general.vehicle_id must be non-zero (spec §7: must match the PX4 param)".into(),
            ));
        }

        if self.transport.kind == TransportKind::Serial && self.transport.serial_path.is_none() {
            return Err(ConfigError::Value(
                "transport.kind = serial requires transport.serial_path".into(),
            ));
        }

        if self.mission_log.flush_rows == 0 {
            return Err(ConfigError::Value("mission_log.flush_rows must be > 0".into()));
        }
        if self.mission_log.flush_secs == 0 {
            return Err(ConfigError::Value("mission_log.flush_secs must be > 0".into()));
        }

        // The startup floor must leave room for at least one full segment,
        // otherwise a mission can never even open below the cap.
        if self.disk.floor_bytes < self.mission_log.seg_cap_bytes {
            return Err(ConfigError::Value(format!(
                "disk.floor_bytes ({}) must be >= mission_log.seg_cap_bytes ({})",
                self.disk.floor_bytes, self.mission_log.seg_cap_bytes
            )));
        }

        // Each resume threshold must sit strictly above its shed threshold,
        // or the shedding ladder would chatter instead of hysteresing.
        check_hysteresis("raw", self.disk.raw_shed_low_bytes, self.disk.raw_resume_bytes)?;
        check_hysteresis("bf", self.disk.bf_shed_low_bytes, self.disk.bf_resume_bytes)?;
        check_hysteresis("crit", self.disk.crit_low_bytes, self.disk.crit_resume_bytes)?;

        // The ladder stages must be strictly ordered: crit < bf < raw, so a
        // falling free-space value crosses them in the documented sequence.
        if !(self.disk.crit_low_bytes < self.disk.bf_shed_low_bytes
            && self.disk.bf_shed_low_bytes < self.disk.raw_shed_low_bytes)
        {
            return Err(ConfigError::Value(
                "disk shed thresholds must be strictly ordered: crit < bf < raw".into(),
            ));
        }

        if self.handshake.context_hz <= 0.0 {
            return Err(ConfigError::Value("handshake.context_hz must be > 0".into()));
        }

        Ok(())
    }
}

fn check_hysteresis(name: &str, shed_low: u64, resume: u64) -> Result<(), ConfigError> {
    if resume <= shed_low {
        return Err(ConfigError::Value(format!(
            "disk.{name}_resume_bytes ({resume}) must be > disk.{name}_shed_low ({shed_low})"
        )));
    }
    Ok(())
}
