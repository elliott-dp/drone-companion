# cc-ai-health

Online, unsupervised, **deterministic** health detection. Eight
closed-form statistical/physics-model algorithms consume the live
`TelemetryEvent` stream and produce the `severity`/`recommended_action`
conclusion that drives `CC_HEALTH_REPORT` — replacing the Phase 6 scripted
severity source. No machine learning anywhere: every finding traces back to
a named formula, and every algorithm ships a synthetic-trace test proving a
benign flight produces zero findings.

Full design rationale (framework derivation, per-algorithm method
derivations, the false-positive strategy, deviations from the original
blueprint): [`docs/phase7/phase7_ai_health.md`](../../docs/phase7/phase7_ai_health.md).

## Scope

| In this crate | Not in this crate |
|---|---|
| The 8 detection algorithms | Producing `TelemetryEvent` — `cc-ingest` |
| The deterministic 10 Hz evaluation grid (`Runner`) | Turning a conclusion into a paced `CC_HEALTH_REPORT` — `cc-health-tx` |
| Merging per-algorithm findings into one conclusion | Replaying a *recorded* mission through this crate's `Runner` — `cc-replay` |
| The shared flight-phase false-positive gate | |
| Pure deterministic statistics primitives (`stats/`) | |

## The determinism spine (the Phase 7 exit criterion)

This is the one property every other design choice in this crate serves:
**replaying a recorded mission must reproduce byte-identical findings**,
regardless of the host machine or how fast/slow it runs.

- **The only clock is `RxMeta.cc_receive_time_ns`** (an integer), treated as
  monotone logical time. The 10 Hz grid is anchored at
  `floor(first_rx / 100ms) * 100ms`. Live operation and `cc-replay` share
  the exact same code path (`Runner::run_events`), so there is no separate
  "replay mode" to drift from live behavior.
- **State folds in `HealthAlgorithm::on_event`, which always runs.
  `evaluate` takes `&self` and is a pure read.** A slow or entirely skipped
  `evaluate` call therefore cannot perturb any later finding — this is the
  actual mechanism that makes byte-identical replay possible, not a
  convention that has to be remembered.
- **`LinkStatus`/`StreamStale` events are never consumed.** They are not
  persisted by `cc-mission-log`, so consuming them would make replay
  diverge from live by construction (deviation D7). Link health is instead
  reconstructed purely from in-stream `seq_gap` and receive-timestamp gaps
  (see `algos::link`).
- Every stats primitive uses fixed-order floating-point reduction and
  `libm` (not the host's native transcendentals) specifically so a
  different CPU architecture cannot produce a different rounding result.

## Architecture

```text
TelemetryEvent (from cc-ingest)
        │
        ▼
  Runner::on_event  ── always runs ──►  every algorithm's on_event (folds state)
        │                                          + FlightPhaseTracker::on_state
        │
  Runner::tick(now_ns)  ── pure read ──►  every algorithm's evaluate(&self, ctx)
        │                                          │
        │                                    [AlgoOutput; 8]
        ▼                                          │
   EvalCtx { now_ns, phase, last_seen_ns, ◄─────────┘
             timesync_locked }
                                                    │
                                              finding::merge()
                                                    ▼
                                          HealthConclusion
                                     (→ cc-health-tx::Conclusion)
```

### `HealthAlgorithm` — the trait every detector implements

```rust
pub trait HealthAlgorithm: Send {
    fn subsystem(&self) -> CcSubsystem;
    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase); // O(1), no allocation
    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput;                 // pure read
    fn reset(&mut self);                                             // on px4_boot_id change
}
```

### `Runner` — the scheduler

```rust
pub struct EvalCtx {
    pub now_ns: i64,
    pub phase: FlightPhase,
    pub last_seen_ns: [i64; 8],   // per-StreamId, 0 = never
    pub timesync_locked: bool,
}
impl EvalCtx {
    pub fn stream_fresh(&self, s: StreamId, max_gap_ns: i64) -> bool;
    pub fn stream_seen(&self, s: StreamId) -> bool;
}

pub const TICK_NS: i64 = 100_000_000; // 10 Hz

impl Runner {
    pub fn new(algos: Vec<Box<dyn HealthAlgorithm>>) -> Self;
    pub fn on_event(&mut self, ev: &TelemetryEvent);
    pub fn tick(&self, now_ns: i64) -> HealthConclusion;
    pub fn run_events(&mut self, events: &[TelemetryEvent]) -> Vec<(i64, HealthConclusion)>;
}
```

`on_event` updates the logical clock, per-stream freshness, the shared
flight-phase tracker (from `State` events only), and folds the event into
every algorithm — then, on a `px4_boot_id` change, resets every algorithm
and the phase tracker (a fault-free fresh start after an FC reboot).
`run_events` is what both `cc-replay` and this crate's own tests use: it
walks a **time-ordered** event slice, firing `tick()` at every 10 Hz grid
boundary crossed, using only state folded from events at-or-before that
boundary — the same bucketing an async 100 ms-interval driver realizes live.

## The flight-phase gate (`phase`) — the primary false-positive defense

```rust
pub enum FlightPhase { Disarmed, Transient, Maneuver, Steady }
impl FlightPhase {
    pub fn suppresses(self) -> bool; // true unless Steady
    pub fn is_steady(self) -> bool;
}
```

Aggressive-but-healthy flight legitimately drives vibration, EKF innovation
ratios, actuator asymmetry, and current draw well outside their calm-hover
ranges. Rather than every algorithm re-deriving "is this a real anomaly or
just a hard maneuver", the `Runner` computes **one** flight phase from the
`State` stream and every detector consults it: anomaly detection is
suppressed by construction outside `Steady`, and adaptive baselines update
**only** in `Steady` (so a maneuver can neither trip a detector nor poison
its learned baseline).

The maneuver boundary is hysteretic with a required dwell: enter `Maneuver`
at `|ω| > 0.35 rad/s` held for 5 consecutive `State` frames; leave only
below `0.25 rad/s`, also held for 5 frames — a bare threshold would leave an
unclassified dead-band a gusty hover chatters across. A post-arm `Transient`
window (3 s) additionally suppresses detection during takeoff spin-up. All
of this is driven only by `State`-stream fields and `cc_receive_time_ns` —
no wall clock.

## Merging findings into one conclusion (`finding::merge`)

```rust
pub mod flags {
    pub const BATTERY: u32 = 1;    pub const MOTOR: u32 = 2;
    pub const VIBRATION: u32 = 4;  pub const GPS: u32 = 8;
    pub const ESTIMATOR: u32 = 16; pub const THERMAL: u32 = 32;
    pub const LINK: u32 = 64;      pub const MISSION: u32 = 128;
    pub const STORAGE: u32 = 256;  pub const AI_DEGRADED: u32 = 512;
    pub const TIMESYNC: u32 = 1024;
}

pub struct HealthFinding { pub subsystem: CcSubsystem, pub flag_bit: u32,
    pub severity: Severity, pub action: Action, pub detail_code: u16,
    pub value: f32, pub limit: f32, pub confidence: u8 }

pub enum AlgoOutput {
    Available(HealthFinding), // a trusted verdict, possibly OK
    Degraded(u16),            // lane untrustworthy this tick (warmup, low excitation, ...)
    Unavailable(u16),         // required data absent/stale
}

pub struct HealthConclusion { pub severity: Severity, pub action: Action,
    pub health_flags: u32, pub detail_code: u16, pub value: f32,
    pub limit: f32, pub confidence: u8 }

pub fn merge(outputs: &[AlgoOutput], timesync_locked: bool) -> HealthConclusion;
```

Two rules an adversarial design review corrected:

1. **Dominant-finding, not max-of-fields.** The reported
   `detail_code`/`value`/`limit`/`confidence` come from the single
   highest-severity finding (ties broken by a fixed subsystem priority:
   battery > motor > estimator > gps > vibration > thermal > link >
   mission), never a synthetic average or mix across findings. Every
   non-OK finding still OR's its `flag_bit` into `health_flags`, so the bit
   field reflects everything currently unhealthy even though only one
   finding's detail is reported.
2. **Cause → safest-action, not enum-max.** On the `Action` enum,
   `Land = 4 < Rtl = 5` numerically — but for a battery- or
   estimator-critical cause, **Land is the more conservative choice**, not
   RTL (RTL spends energy flying home and trusts the navigation solution).
   Each algorithm already picks a safe action for its own cause; `merge`
   then cross-checks: **both RTL and Hold (Loiter) require a trustworthy
   navigation solution** (RTL flies a GPS course home, Loiter holds a
   GPS/estimator position) — so when GPS or the estimator is itself
   unhealthy, either action is downgraded to `Land`, which needs only the
   independent height estimate.

`AlgoOutput::Degraded`/`Unavailable` (from any algorithm) OR the timesync
snapshot not being `Locked` sets `AI_DEGRADED`/`TIMESYNC` in `health_flags`
respectively — these never become a domain-specific finding on their own.

## The 8 algorithms (`algos/`)

Shared discipline across all eight: warm-up before any finding
(`AlgoOutput::Degraded(warmup)` until enough samples/time accumulate),
adaptive baselines freeze once a detector trips (so the anomaly can't be
learned as the new normal), NaN input degrades a lane rather than becoming
a fault, and every `CRITICAL` here is advisory until a benign-corpus
false-positive audit passes (the FC monitor stays warn-only on these lanes
until then, per the Phase 7 exit criterion). There is deliberately **no
shared window buffer** — each detector keeps exactly the O(1) streaming
state it needs (an EWMA, a robust ring, a CUSUM, an RLS), simpler and a
cleaner determinism story than a common ring the algorithms would index
into.

### `battery` — `BatteryModel` (bit 1)

Physics model: `v_cell = OCV(SoC) − I·R_int(SoC, T)`. `OCV` is a nonlinear
lookup table capturing the LiPo end-of-discharge knee below ~20% SoC, so a
healthy landing's terminal sag is *predicted*, not flagged. `R_int` carries
an Arrhenius temperature term, so a cold pack's higher resistance is
likewise predicted. Five detectors: sag-beyond-model (robust z-score +
one-sided Page-Hinkley on the residual), internal-resistance growth
(one-sided CUSUM on `R̂/R_model`), imminent brownout (projected voltage at
sustained hover current vs. the 3.3 V/cell floor), gauge
consistency (`consumed_mah`/`remaining` must be monotone), and a PX4
`battery.warning` echo. **Self-gates to `Degraded`** whenever `var(I)` is
below a conditioning floor — hover current is too collinear to identify
`R_int` reliably, so the lane admits it can't tell rather than guess.
`CRITICAL → Land`.

### `vibration` — `VibrationAnomaly` (bit 4)

The three `vibration_metric[3]` entries are **different physical
quantities**, not an xyz vector: `[0]` accel (m/s²), `[1]` gyro (rad/s),
`[2]` delta-angle coning — each gets its own throttle-normalized RLS
residual + one-sided Page-Hinkley, so vibration that rises with rotor speed
is expected and only the residual *unexplained by throttle* trips. Only the
accel metric carries PX4's documented absolute limits (WARN ≥ 30,
CRITICAL ≥ 60 m/s²), and that absolute rule is maneuver-gated (acro flight
legitimately spikes it). Clipping-count *rate* (not the raw cumulative
count) is a near-false-positive-free independent saturation signal.
`CRITICAL` requires ≥ 2 metrics tripping together, or clip rate ≥ 20/s, or
accel ≥ 60. `CRITICAL → Land`.

### `estimator` — `EstimatorConsistency` (bit 16)

Watches PX4's own EKF innovation test ratios. Rather than a fixed
sub-1.0 threshold (which would false-positive on healthy dynamic flight
riding 0.5–0.8), each channel learns its **own** adaptive baseline (EWMA in
`Steady` flight only) and a one-sided CUSUM watches for drift *above* that
learned baseline — the absolute hard rule stays at PX4's own rejection
boundary (`ratio > 1.0`, sustained). Velocity and position innovations
share a GNSS cause and therefore do **not** corroborate each other;
`CRITICAL` requires breaches in **two independent** cause groups (GNSS,
height, mag). Action ceiling `Warn → BlockOffboard`, `Critical → Hold` —
never RTL, since a bad estimator makes navigation itself untrustworthy.

### `gps` — `GpsQuality` (bit 8)

A panel of per-indicator detectors grouped by independent cause: geometry
(`fix_type`, `satellites_used`, `eph`/`epv` — the latter two step-detected
on `ln(x)` since they're heavy-tailed), RF (`noise_per_ms`,
`jamming_indicator` — **adaptive only, no fixed absolute**, since acceptable
noise floors are site-specific), and consistency (GPS vs. EKF horizontal
speed, gated on `estimator_valid`). `fix_type < 3` sustained is PX4's
**definitive** loss of 3D fix and reaches `CRITICAL` alone; otherwise
`CRITICAL` needs two independent groups bad simultaneously, one group alone
is `WARN`. Action `Critical → Hold` (RTL without reliable GNSS is unsafe).

### `motor` — `MotorBalance` (bit 2)

Honestly scoped: there is no ESC RPM/current/temperature telemetry, so this
lane is **advisory-only** (confidence hard-capped ≤ 70), watching only
whether the mixer persistently commands one motor harder than the
collective mean. The **heading-invariance discriminator**: wind-driven
asymmetry is earth-fixed (the loaded motor rotates with heading as the
vehicle yaws), while a weak motor's asymmetry is body-fixed (constant
regardless of heading). The lane accumulates each motor's excess across
**heading diversity** and only attributes an offset to a specific motor
once enough heading coverage has been seen for the earth-fixed wind
component to average out; with the vehicle pointing one way the whole
flight, it reports `HEADING_STATIONARY_ASYM` (low confidence) rather than
guess. Also watches sustained actuator saturation. **Never emits
`CRITICAL`** — that would require the (separate) vibration corroborator
plus saturation, deliberately left as a cross-lane, post-audit extension.

### `link` — `LinkQuality` (bit 64)

Reconstructed **purely from fields that are actually recorded to disk**:
`RxMeta.seq_gap` and inter-arrival time vs. each stream's
`StreamId::nominal_period_ns()`. Deliberately never uses `link_rtt_ms`
(not a `TelemetryEvent` field), `IngestStats` atomics (a live side-channel
absent from the mission log), or `LinkStatus`/`StreamStale` events (dropped
by `cc-mission-log`) — using any of them would make live and replayed
findings diverge (D7). Reports the degraded-but-alive band (`WARN`); a
fully dead link is the deterministic FC safety monitor's STALE-timeout
domain, not this lane's.

### `thermal` — `ThermalMonitor` (bit 32)

Battery pack and IMU die temperature only — there is no Jetson SoC
temperature in the telemetry contract (deviation D2). Each channel is
median-of-3 despiked (one garbage sample can't move the verdict) and
lightly EWMA-smoothed; two rules run on the smoothed signal: an absolute
limit, and a rate-of-rise `dT/dt` that is **armed only above a temperature
floor** so a normal cold-start warm-up ramp is never flagged as thermal
runaway. A battery that is both hot *and* rising fast is the runaway
signature (`CRITICAL → Land`); an absolute battery over-temp is likewise
`CRITICAL`; IMU over-temperature degrades sensor bias but is not
immediately vehicle-fatal (`WARN` ceiling).

### `mission` — `MissionRisk` (bit 128)

Closes a gap the FC's own battery failsafe can't: it's a *local* SoC
threshold, unaware that "18% remaining" may not be enough to fly 900 m back
home into a headwind. This lane learns cruise current draw and cruise
horizontal speed online (both EWMAs, updated only in `Steady` flight),
tracks distance from the arm position, and projects
`soc_at_home = (remaining·capacity − I_cruise·(distance/v_rtl)) / capacity`.
`< 20%` projected → `WARN`; `< 10%` → `CRITICAL`, the *point of no return*.
This is the **only** lane allowed to recommend RTL — and even here, the
merge's nav-health cross-check still downgrades it to `Land` if GPS or the
estimator is unhealthy. Only assessed once meaningfully away from home
(near home, the battery lane already covers low SoC directly).

## Stats primitives (`stats/`)

Pure, deterministic, individually unit-tested (including a double-run
byte-identity test per primitive):

```rust
pub fn clamp01(x: f64) -> f64;              // NaN -> 0.0
pub fn ln(x: f64) -> f64;                   // libm, domain-guarded
pub fn logistic(x: f64) -> f64;             // libm
pub fn confidence_percent(conf01: f64) -> u8; // round-half-even, scaled-integer
pub fn sort_total(v: &mut [f64]);           // f64::total_cmp, NaN sorts last
```

- **`Ewma`** — exponential moving average with West's incremental variance,
  freeze support, NaN-reject.
- **`RobustScale`** — rolling median + MAD (median absolute deviation) over
  a fixed-capacity ring, `1.4826·MAD` scale with a configurable floor
  (guards against a near-zero MAD blowing up a z-score), `z(x)` helper.
- **`Cusum`** — one-sided cumulative-sum change-point detector with slack
  and threshold, `CusumTrip::{None, Up, Down}`, `excess()` for a normalized
  over-threshold magnitude.
- **`PageHinkley`** — one-sided Page-Hinkley change-point detector
  (`Direction::{Up, Down}`), the running-mean-deviation statistic with a
  slack term.
- **`Rls3`** — fixed 3×3 recursive least squares with forgetting factor;
  `update` returns the residual, `p_trace()` is a conditioning proxy (used
  by `battery` to self-gate on identifiability).

## Public API surface (`lib.rs` re-exports)

```rust
pub use finding::{merge, AlgoOutput, HealthConclusion, HealthFinding};
pub mod algos;   // default_registry() -> Vec<Box<dyn HealthAlgorithm>>
pub mod detail;  // the detail-code namespace (subsystem_block(1000·k) + code)
pub mod finding;
pub mod phase;
pub mod stats;
```

`algos::default_registry()` returns the eight algorithms in the fixed order
`battery, vibration, estimator, gps, motor, link, thermal, mission` — this
order doesn't affect the merged conclusion (which is order-independent by
design) but keeps a future `CC_AI_DIAGNOSTIC` round-robin scheduler stable.

## Testing

Framework: `Runner`'s grid-bucketing and merge plumbing
(`grid_fires_at_100ms_boundaries`, `replay_is_byte_identical` in `lib.rs`),
plus a **full-registry integration test** driving all eight algorithms
together through a 45 s synthetic benign multi-stream trace and asserting
**zero** WARN/CRITICAL (`benign_flight_produces_no_warn_or_critical`) and a
system-level byte-identical-replay proof
(`full_registry_replay_is_byte_identical`, FNV-style hash over the whole
finding timeline). `phase.rs` and `finding.rs` each carry focused unit
tests for the hysteretic maneuver dwell and the two corrected merge rules
respectively. Every algorithm file follows the same pattern: a synthetic
"healthy" trace that must produce `Ok`, one or more synthetic fault
injections that must produce the documented finding, and a double-run
determinism test.

## Dependencies

`cc-ingest` (`TelemetryEvent`, `StreamId`), `cc-protocol`
(`CcSubsystem`), `cc-health-tx` (`Severity`, `Action` — reused rather than
redefined, since this crate's conclusions feed directly into that crate's
`Conclusion`), `cc-config` (referenced for future config-driven thresholds,
deviation D4), `libm` (the one dependency added specifically for
cross-architecture bit-reproducible transcendentals).
