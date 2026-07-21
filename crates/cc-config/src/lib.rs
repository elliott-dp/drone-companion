//! `cc-config` — layered configuration for companiond (dev-plan Phase 5.1).
//!
//! Precedence, lowest to highest: **built-in defaults → config file (TOML) →
//! environment (`CC_<SECTION>__<FIELD>`) → CLI overrides**. The merge is
//! per-field: a CLI flag overrides only its own field, never a whole section.
//!
//! The crate is pure and synchronous. The only I/O is one optional file read
//! in [`Config::load`]; the actual merge ([`Config::merge`]) is a pure
//! function over three [`PartialConfig`] layers so every precedence rule is
//! unit-testable without touching the filesystem or the process environment.

mod layer;
mod model;
mod validate;

pub use layer::PartialConfig;
pub use model::{
    Compression, Config, Disk, General, Handshake, MissionLog, ParamSnapshotMode, Transport,
    TransportKind,
};

use std::path::{Path, PathBuf};

/// Default config path when neither `--config` nor `$CC_CONFIG` is set.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/companiond/config.toml";

/// Everything that can go wrong loading a configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The config file is present but could not be read or parsed.
    File(String),
    /// An environment variable could not be parsed to its field type.
    Env(String),
    /// A field carried an invalid value (bad enum string, failed invariant).
    Value(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::File(m) => write!(f, "config file: {m}"),
            ConfigError::Env(m) => write!(f, "config env: {m}"),
            ConfigError::Value(m) => write!(f, "config value: {m}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Pure merge of three pre-parsed layers on top of the built-in defaults,
    /// followed by validation. This is the whole precedence engine; the file
    /// and environment reads in [`Config::load`] are the only impurity.
    pub fn merge(
        file: PartialConfig,
        env: PartialConfig,
        cli: PartialConfig,
    ) -> Result<Config, ConfigError> {
        let mut merged = file;
        merged.overlay(env);
        merged.overlay(cli);

        let mut cfg = Config::default();
        cfg.apply(merged)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Resolve the full configuration: read the config file (if any), read the
    /// environment, and merge the CLI overrides on top.
    ///
    /// * `explicit_path` — from a `--config` flag; when `None`, `$CC_CONFIG`
    ///   then [`DEFAULT_CONFIG_PATH`] are tried.
    /// * A **missing** config file is not an error (defaults + env + CLI still
    ///   apply). A **present but malformed** file is a hard error.
    pub fn load(explicit_path: Option<&Path>, cli: PartialConfig) -> Result<Config, ConfigError> {
        let path = resolve_config_path(explicit_path);
        let file = match path.as_deref().map(read_optional_file).transpose()? {
            Some(Some(text)) => PartialConfig::from_toml_str(&text)?,
            _ => PartialConfig::default(),
        };
        let env = PartialConfig::from_env(|k| std::env::var(k).ok())?;
        Config::merge(file, env, cli)
    }
}

fn resolve_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("CC_CONFIG") {
        return Some(PathBuf::from(p));
    }
    Some(PathBuf::from(DEFAULT_CONFIG_PATH))
}

/// `Ok(None)` if the file does not exist; `Ok(Some(text))` if read;
/// `Err` if it exists but cannot be read.
fn read_optional_file(path: &Path) -> Result<Option<String>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ConfigError::File(format!("{}: {e}", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> PartialConfig {
        PartialConfig::default()
    }

    #[test]
    fn defaults_are_valid_and_expected() {
        let cfg = Config::merge(none(), none(), none()).unwrap();
        assert_eq!(cfg.general.vehicle_id, 1);
        assert_eq!(cfg.transport.kind, TransportKind::Udp);
        assert_eq!(cfg.transport.udp_bind, "0.0.0.0:24040");
        assert_eq!(cfg.mission_log.flush_rows, 5_000);
        assert_eq!(cfg.disk.floor_bytes, 5 * 1024 * 1024 * 1024);
        assert_eq!(cfg.handshake.param_snapshot, ParamSnapshotMode::Stub);
    }

    #[test]
    fn each_layer_wins_at_its_level() {
        // vehicle_id set by file; udp_bind set by env; sysid set by cli.
        let file = PartialConfig::from_toml_str(
            "[general]\nvehicle_id = 7\n[transport]\nudp_bind = \"0.0.0.0:1\"\nsysid = 2\n",
        )
        .unwrap();
        let env = PartialConfig::from_env(|k| match k {
            "CC_TRANSPORT__UDP_BIND" => Some("127.0.0.1:9".into()),
            _ => None,
        })
        .unwrap();
        let mut cli = PartialConfig::default();
        cli.transport.sysid = Some(42);

        let cfg = Config::merge(file, env, cli).unwrap();
        assert_eq!(cfg.general.vehicle_id, 7, "file wins where only file sets it");
        assert_eq!(cfg.transport.udp_bind, "127.0.0.1:9", "env overrides file");
        assert_eq!(cfg.transport.sysid, 42, "cli overrides file");
    }

    #[test]
    fn cli_overrides_env_overrides_file_same_field() {
        let file =
            PartialConfig::from_toml_str("[mission_log]\nflush_rows = 100\n").unwrap();
        let env = PartialConfig::from_env(|k| {
            (k == "CC_MISSION_LOG__FLUSH_ROWS").then(|| "200".to_string())
        })
        .unwrap();
        let mut cli = PartialConfig::default();
        cli.mission_log.flush_rows = Some(300);

        assert_eq!(Config::merge(file.clone(), env.clone(), cli).unwrap().mission_log.flush_rows, 300);
        assert_eq!(Config::merge(file.clone(), env, none()).unwrap().mission_log.flush_rows, 200);
        assert_eq!(Config::merge(file, none(), none()).unwrap().mission_log.flush_rows, 100);
    }

    #[test]
    fn malformed_toml_is_a_hard_error() {
        let err = PartialConfig::from_toml_str("this is = = not toml").unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));
    }

    #[test]
    fn unknown_field_rejected() {
        let err = PartialConfig::from_toml_str("[general]\nnope = 1\n").unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));
    }

    #[test]
    fn env_nested_section_field_parses() {
        let env = PartialConfig::from_env(|k| match k {
            "CC_DISK__FLOOR_BYTES" => Some("3221225472".into()), // 3 GiB, above the seg cap
            "CC_TRANSPORT__KIND" => Some("serial".into()),
            "CC_TRANSPORT__SERIAL_PATH" => Some("/dev/ttyTHS1".into()),
            _ => None,
        })
        .unwrap();
        let cfg = Config::merge(none(), env, none()).unwrap();
        assert_eq!(cfg.disk.floor_bytes, 3_221_225_472);
        assert_eq!(cfg.transport.kind, TransportKind::Serial);
        assert_eq!(cfg.transport.serial_path.as_deref(), Some("/dev/ttyTHS1"));
    }

    #[test]
    fn env_unparseable_value_errors() {
        let err = PartialConfig::from_env(|k| {
            (k == "CC_TRANSPORT__BAUD").then(|| "not-a-number".to_string())
        })
        .unwrap_err();
        assert!(matches!(err, ConfigError::Env(_)));
    }

    #[test]
    fn bad_enum_string_errors_uniformly() {
        let mut cli = PartialConfig::default();
        cli.mission_log.compression = Some("brotli".into());
        let err = Config::merge(none(), none(), cli).unwrap_err();
        assert!(matches!(err, ConfigError::Value(m) if m.contains("compression")));
    }

    #[test]
    fn validate_rejects_zero_vehicle_id() {
        let mut cli = PartialConfig::default();
        cli.general.vehicle_id = Some(0);
        assert!(matches!(Config::merge(none(), none(), cli), Err(ConfigError::Value(_))));
    }

    #[test]
    fn validate_rejects_serial_without_path() {
        let mut cli = PartialConfig::default();
        cli.transport.kind = Some("serial".into());
        assert!(matches!(Config::merge(none(), none(), cli), Err(ConfigError::Value(_))));
    }

    #[test]
    fn validate_rejects_floor_below_segment_cap() {
        let mut cli = PartialConfig::default();
        cli.disk.floor_bytes = Some(1024); // far below the 2 GiB seg cap
        let err = Config::merge(none(), none(), cli).unwrap_err();
        assert!(matches!(err, ConfigError::Value(m) if m.contains("floor_bytes")));
    }

    #[test]
    fn validate_rejects_non_hysteretic_thresholds() {
        let mut cli = PartialConfig::default();
        cli.disk.raw_resume_bytes = Some(1024); // below raw_shed_low default (2 GiB)
        assert!(matches!(Config::merge(none(), none(), cli), Err(ConfigError::Value(_))));
    }

    #[test]
    fn validate_rejects_unordered_ladder() {
        // Push crit above bf so crit < bf < raw is violated.
        let mut cli = PartialConfig::default();
        cli.disk.crit_low_bytes = Some(3 * 1024 * 1024 * 1024);
        cli.disk.crit_resume_bytes = Some(4 * 1024 * 1024 * 1024);
        assert!(matches!(Config::merge(none(), none(), cli), Err(ConfigError::Value(_))));
    }
}
