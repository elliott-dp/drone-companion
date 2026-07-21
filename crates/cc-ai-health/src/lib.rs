//! `cc-ai-health` — online, unsupervised, **deterministic** companion
//! health-detection (dev-plan Phase 7). It replaces the Phase-6 scripted
//! severity source: it runs change-point-statistics + physics-model algorithms
//! over the live `TelemetryEvent` stream and emits the `severity` +
//! `recommended_action` that go into `CC_HEALTH_REPORT`, which the deterministic
//! FC safety monitor turns into policy.
//!
//! Design (framework, determinism contract, per-algorithm methods, false-
//! positive strategy, deviations): `docs/phase7/phase7_ai_health.md`.
//!
//! ## Engineering bar
//! * **Specificity first** — ~zero false positives on benign flight is the exit
//!   criterion; every CRITICAL is advisory until a documented FP audit.
//! * **Byte-identical determinism** — `cc-replay` re-runs a recorded mission and
//!   must produce identical findings, so every recorded mission is a regression
//!   fixture. All state is folded on the event stream (never the wall clock);
//!   see [`stats`] for the determinism primitives.
//! * **No ML** — closed-form statistical/physics change-detection only; a
//!   trained model would add a non-determinism + provenance surface for no
//!   specificity gain.

pub mod stats;
