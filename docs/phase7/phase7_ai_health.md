# Phase 7 — `cc-ai-health` + `cc-replay`: the real detection algorithms

**Goal (dev plan):** real algorithms behind the (already-proven) safety loop.
Exit criteria: algorithms green on synthetic + replay fixtures; replay
deterministic (byte-identical findings for identical input); a documented
false-positive audit on benign data before the monitor is allowed to auto-act.

This is the analysis brain that replaces the Phase-6 *scripted* health source.
It runs online, unsupervised, on a single vehicle with no labels and no fleet
baseline — so it is **anomaly detection with change-point statistics and
physics models, not classification, and not ML**. The engineering bar is
**specificity first** (near-zero false positives on benign flight) and
**byte-identical determinism** so every recorded mission becomes a regression
fixture.

The design below is the synthesis of a four-lens expert panel (physics-informed,
embedded-pragmatism, statistical-rigor, false-positive-robustness) with every
fatal flaw the adversarial judges raised resolved. Deviations D-numbers record
where it departs from a naïve reading.

---

## Part A — Framework & cross-cutting contracts

### A.1 `HealthAlgorithm` trait — state advances on events, not on the clock

The single most important determinism rule (judge-mandated): **all detector
state is folded in `on_event`, which always runs; `evaluate` is a pure read
that produces at most one finding and never mutates detector state.** A slow or
skipped `evaluate` therefore cannot change any later finding — replay on a fast
box and a slow box produce identical output.

```rust
pub trait HealthAlgorithm {
    fn id(&self) -> AlgoId;                 // stable enum = iteration / tie-break order
    fn subsystem(&self) -> CcSubsystem;
    fn flag_bit(&self) -> u32;              // CC_HEALTH_FLAGS bit
    fn requirements(&self) -> DataReq;      // streams needed, warmup_samples, max_gap_ns
    fn deadline_ns(&self) -> i64;           // per-evaluate compute budget (prod-only guard)
    fn on_event(&mut self, ev: &TelemetryEvent);          // O(1), no alloc — folds ALL state
    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput;      // PURE read; &self, not &mut
    fn reset(&mut self);                    // px4_boot_id change
}
pub enum AlgoOutput { Available(HealthFinding), Degraded(u16), Unavailable(u16) }
```

`EvalCtx` carries the integer-ns tick, the shared `TelemetryWindows`, the shared
`FlightPhase`, and per-stream freshness. `evaluate` taking `&self` is enforced
by the type system: the finding is a function of already-folded state + windows,
nothing else.

### A.2 The runner, the clock, and the 10 Hz grid

The runner is a **lossy** subscriber of the `cc-ingest` `broadcast<TelemetryEvent>`
(like the mission logger — RX is never blocked; broadcast lag surfaces as
`seq_gap` the algorithms already tolerate because windows are **time-indexed**).

- **The only clock is `RxMeta.cc_receive_time_ns`** (i64), taken as a monotone
  logical time `t = max(t, ev.cc_receive_time_ns)`. No `SystemTime`, no `Instant`
  in any decision.
- **10 Hz grid** anchored at `t0 = floor(first_rx / 100_000_000) * 100_000_000`;
  `evaluate` fires at `t0 + k·100_000_000` as recorded events cross each
  boundary. In prod the boundary is realized by a 100 ms tokio interval whose
  payload is the ingest clock; in replay the recorded timestamps hit the same
  boundaries — **one code path**, so replay-to-replay is byte-identical.
- Per tick: fold every event with `cc_receive_time_ns < boundary` (already done
  incrementally in `on_event`), compute `FlightPhase`, then call each algo's
  `evaluate` in `AlgoId` order.

### A.3 Availability — two kinds of Degraded, only one is deterministic

Availability = `min(warmup, freshness, budget)`; an unavailable lane contributes
**only** `CC_HF_AI_DEGRADED` (bit 512), never a domain finding, never a false OK.

- **Data-driven Degraded** (deterministic, part of the byte-diff): warmup not
  met, a required stream stale (`boundary − last_sample_ns > max_gap_ns`),
  `NaN` in a required field, or a model self-gate (e.g. battery current
  excitation too low). Computed from data carried in the stream, so it is
  reproducible in replay.
- **Deadline-driven Degraded** (prod-only, **excluded** from the golden diff):
  the runner times `evaluate` on a monotonic instant and, on overrun, drops the
  (already-computed) finding and sets `ai_degraded`. Because bounded fixed-size
  windows guarantee no real overrun, this is an overload alarm, not normal flow;
  and because state was already folded in `on_event`, a dropped finding changes
  nothing downstream. **In replay the deadline guard is disabled entirely** (D6)
  — determinism over an unknowable host speed.

### A.4 The shared FlightPhase gate — the master FP suppressor

The single strongest false-positive defence (judge-unanimous): a **centralized**
flight-phase context every detector consults. `FlightPhase ∈ {Disarmed, Takeoff,
Hover, Cruise, Maneuver, Landing}`, derived from `arming_state`, `nav_state`,
`|angular_velocity|`, `|velocity_ned|`, and altitude rate — **with hysteresis**
so a gusty hover cannot chatter across the boundary (a dead-band is a bug: enter
Maneuver at `|ω|>0.35`, leave only below `0.25`, and require N consecutive
frames each way). Anomaly detectors are **suppressed by construction** in
`Takeoff/Maneuver/Landing`, and adaptive baselines update **only** on gated
(quasi-steady) frames — so aggressive-but-healthy flight never feeds or trips a
detector. This gates the *absolute* vibration/estimator/gps WARNs too, not just
motor_balance (a judge fix — the absolute backstops were the FP hole).

### A.5 Anti-masking: absolute + frozen-relative + whole-flight drift

Every adaptive detector is built from three layers (judge-unanimous best idea):

1. a **relative** change-detector on a robust baseline (EWMA-median + MAD, or a
   physics residual), **frozen** while a finding is active or while disarmed
   (so an anomaly can never be absorbed into its own baseline);
2. an **absolute** PX4-ground-truth backstop (needs no warmup — covers the
   early-mission window when the adaptive baseline is UNAVAILABLE);
3. an **independent whole-flight drift channel** (a slow CUSUM/EWMA vs the
   first-converged value) so a slow-onset fault the short window would adapt
   away is still caught.

### A.6 HealthFinding, merge, and the action table

```rust
pub struct HealthFinding {
    subsystem: CcSubsystem, flag_bit: u32,
    severity: Severity, action: Action,     // reuse cc_health_tx::{Severity, Action}
    detail_code: u16, value: f32, limit: f32, confidence: u8,
}
```

Per-algorithm **severity** uses an **asymmetric streak** model: escalate after
`N` consecutive breaching 10 Hz frames, de-escalate only after `M > N` clear
frames — a single noisy frame never moves severity. **Confidence** is
`round_half_even(100 · clamp01(specificity · margin · observability ·
warmup_fraction))` computed in f64 and quantized with scaled-integer
round-half-even (no float rounding-mode drift); ground-truth triggers
(`battery.warning`, `estimator_valid`, `fix_type`, `failsafe_flags`) score high,
pure statistical anomalies lower, weakly-observable lanes are capped (motor ≤ 70).

**Merge → conclusion** (pure, fixed `AlgoId` order):
- `severity = max` over Available findings. STALE is never fabricated here —
  companion blindness surfaces as `link_quality` degraded/CRITICAL on real
  timestamp gaps; the FC monitor owns STALE (report-gap).
- `health_flags = OR` of every Available non-OK finding's bit, `+ ai_degraded`
  if any lane Degraded/Unavailable, `+ timesync` if `age == UnknownOffset`.
- `detail_code / value / limit / confidence` = the **dominant** finding
  (highest severity, tie-break by subsystem priority), *not* a max-of-actions.
- `recommended_action` comes from an explicit **cause → safest-action table**
  (judge fix — enum-max is wrong: for battery/estimator-critical, **Land is
  more conservative than RTL** because RTL spends energy and trusts navigation).
  `RTL` is reserved to `mission_risk` and only when GPS **and** estimator are
  healthy; a bad estimator or bad GPS caps the action at `Hold`.

The conclusion drives the existing `cc-health-tx` `ReportSource` (pacing,
escalation-immediate/de-escalation-debounce, `CC_SAFETY_STATUS` ACK) via a new
`HealthSource` trait — the scripted `Scenario` stays behind the same trait for
injection tests (D1). `CC_AI_DIAGNOSTIC` is emitted log-only ≤ 2 Hz by a
deterministic severity-priority round-robin (CRITICAL preempts).

### A.7 Determinism recipe (the exit criterion)

Integer-ns logical clock; fixed grid; total event order `(cc_receive_time_ns,
StreamId, seq)`; fixed `AlgoId`/index/channel iteration; **no `HashMap` in the
finding path** (arrays / enum-indexed tables / `BTreeMap`); all f64 reductions
in fixed ring-index order, narrowed to f32 only at emit; `f32::total_cmp` for
NaN-ordered medians; **histogram-based** median/MAD (avoids sort tie-break
ambiguity *and* the MAD=0 → z=∞ hazard on quantized fields — every robust scale
has an ε floor + an absolute backstop); `libm` for the few needed transcendentals
(bit-reproducible cross-arch); **no fma / fast-math / rayon**; state folded only
in `on_event`; DATA-driven Degraded deterministic, DEADLINE-driven excluded.
Fixed-schedule (data-independent) reconditioning of any long-running accumulator.
Test: run a fixture mission twice, canonically serialize the finding stream,
assert equal SHA-256 (same discipline as `golden_roundtrip`/`dialect_hash`).
Cross-arch (x86-64 CI ↔ aarch64 Jetson) byte-identity is a separate gate.

---

## Part B — the algorithms (method · math · thresholds · FP-guard)

Detail-code namespace: `subsystem_block(0x1000·k) + code`. Each ships synthetic
traces: benign→no finding (an explicit FP proof), injected fault→timely finding,
double-run→byte-identical. Until the benign-corpus FP audit (Part C.4) passes,
**every CRITICAL is advisory — the FC monitor does not auto-act** (`CC_MON_*`
stays warn-only), per the exit criterion.

### B.1 battery_model (bit 1) — the highest-value lane
Physics: `v_cell = OCV(SoC) − I·R_int`. **OCV LUT** (nonlinear, captures the
LiPo knee below ~20% SoC so end-of-discharge sag is *predicted, not flagged* —
judge fix); `R_int(SoC,T)` with an **Arrhenius** temperature term (cold packs
legitimately show high R — predicted, not flagged). Detectors: (1) **sag beyond
model** — residual `r = v_cell − v_pred`, robust `z = r/(1.4826·MAD)`, one-sided
Page-Hinkley on negative z; (2) **R growth** — CUSUM on `R̂/R_baseline`;
(3) **imminent brownout** — predict `v_cell` at hover current vs the `3.3 V·cell`
floor; (4) **consumed-vs-remaining** consistency + gauge monotonicity;
(5) ground-truth `battery.warning` echo. **Self-gate to Degraded when `var(I)`
is below an OLS-conditioning floor** (hover is near-collinear in `[1, C, I]` →
R unidentifiable — judge fix). Severity/action: WARN→WarnOnly; CRITICAL→**Land**.
FP-guard: `I > I_min`, model absorbs load current, freeze-on-anomaly, 20 s +
≥50 high-current samples warmup, `connected==0`/`remaining==NaN`→Degraded.

### B.2 vibration_anomaly (bit 4)
Inputs are **three different metrics** (verified): `[0]` accel (m/s²), `[1]`
gyro (rad/s), `[2]` delta-angle coning — treated per-metric (judge fix; not
x/y/z). Only the accel metric carries the PX4 absolute backstop (WARN ≥30,
CRITICAL ≥60 m/s²), **maneuver-gated** so acro flight doesn't trip it (judge
fix). Primary detector: throttle-normalized residual (RLS of metric on throttle
proxy, slope ≥0) → PH-upward on the robust z of the residual, per metric,
frozen-on-anomaly. Clipping: `Δclipping_count/Δt` (cumulative → rate), a
high-specificity signal (WARN ~2/s, CRITICAL ~20/s), boot-reset aware.
CRITICAL when ≥2 metrics trip together or clip rate high or accel≥60. Action
CRITICAL→Land.

### B.3 estimator_consistency (bit 16)
Inputs are the EKF's own normalized innovation test ratios (<1 pass, >1
rejected). Value-add = **early sustained-drift detection before 1.0** via
one-sided CUSUM (`μ0≈0.3, k≈0.15, h≈2.0`) — a single glitch never accumulates.
Absolute breach at ratio>1.0 sustained. **WARN threshold sits at/above PX4's
own rejection semantics, not at 0.5/0.7** (judge fix — healthy dynamic flight
rides 0.5–0.8). Innovation-flag bit streak counters. `airspeed_test_ratio` is
NaN on multicopters → that channel Unavailable, never a fault. Multi-channel
CRITICAL requires **independent** causes — vel+pos share a GPS cause, so they do
**not** corroborate each other (judge fix). Action caps at **Hold/BlockOffboard**
(a bad estimator makes navigation itself untrustworthy — never RTL).

### B.4 gps_quality (bit 8)
Per-indicator monitors: `fix_type<3`; `satellites_used` floor + downward CUSUM;
`eph/epv` PH-upward on `log`(heavy-tailed) + absolute; `noise_per_ms`/
`jamming_indicator` per-receiver EWMA+MAD upward step (no fixed absolute);
GPS↔EKF speed divergence (gated on `estimator_valid`). **CRITICAL requires ≥2
independent corroborating indicators** (no single noisy field auto-acts).
Environment (foliage/urban-canyon) gating so marginal-but-benign GNSS doesn't
WARN-spam (judge fix). Action CRITICAL→**Hold** (GNSS loss → RTL is unsafe).

### B.5 motor_balance (bit 2) — correlation-only, honest
No ESC/RPM/current telemetry → correlation-based, **WARN-only/advisory** until
HITL. Steady-flight gate; residual = per-motor excess over collective mean.
**Heading-invariance discriminator** (the panel's sharpest idea): wind
asymmetry *rotates* with heading, a motor fault is *body-frame-stationary* —
correlate the excess-command pattern against heading and keep only the
body-fixed component. CRITICAL (if ever armed post-audit) requires the
**vibration** corroborator specifically (power-elevation is *not* independent of
the rate residual under wind — judge fix), plus actuator-saturation. Confidence
hard-capped ≤ 70; documented reduced observability.

### B.6 link_quality (bit 64)
**Reconstructed purely from in-stream, replayable fields** — `RxMeta.seq_gap`
and `cc_receive_time_ns` inter-arrival vs `StreamId::nominal_period_ns()`. **No
RTT** (`link_rtt_ms` isn't a `TelemetryEvent` field) and **no `IngestStats`
atomics** (a live side-channel, not in the mission log → non-replayable) and
**no `StreamStale`/`LinkStatus` events** (dropped by cc-mission-log →
non-replayable) — all three were data/determinism holes the judges caught (D7).
Per-stream drop-rate EWMA + upward CUSUM, effective-rate ratio, jitter. Reports
the degraded-but-alive band; full loss is the FC monitor's STALE domain.

### B.7 thermal_monitor (bit 32)
Battery + IMU temperature only — **no Jetson SoC temp exists in the telemetry
contract** (D2; flagged as a candidate telemetry addition). Median-of-3 despike
+ EWMA; absolute limits + rate-of-rise (`dT/dt`) **armed only above a T-floor**
so cold-start warm-up isn't flagged. Battery high-T + rapid rise → CRITICAL Land
(runaway). NaN temp → channel Unavailable.

### B.8 mission_risk (bit 128)
Energy-to-home reserve the FC doesn't itself run: `distance_home`, cruise
power/speed EWMAs (updated only in gated cruise), projected `remaining` at home
vs a reserve. WARN <20%, CRITICAL (point-of-no-return) <~10% → **RTL** (the one
lane allowed RTL, and only when GPS+estimator healthy). Config-driven (pack
capacity, reserve %, geofence — D4). Advisory until FP audit.

---

## Part C — crate layout, cc-replay, determinism, FP audit

### C.1 `crates/cc-ai-health`
`src/lib.rs` (trait, Runner, AlgoId registry, 10 Hz scheduler, Availability) ·
`window.rs` (per-stream f64 rings + RxMeta; deterministic index-order views;
10 Hz frame aggregator for time-aligned cross-stream series) · `phase.rs`
(shared FlightPhase gate with hysteresis) · `finding.rs` (HealthFinding, merge,
cause→action table, CC_AI_DIAGNOSTIC scheduler) · `detail.rs` (detail-code
namespace) · `stats/` (pure, unit-tested primitives: `ewma`, `cusum`,
`page_hinkley`, `robust` [histogram median/MAD, NaN=missing enforced here],
`rls` [fixed 3×3 forgetting], `welford`/co-moment) · `algos/` (the 8, each with
synthetic-trace tests). Decision cores are pure; the async runner is a thin
tokio wrapper over `Runner::tick(now_ns, &windows)`.

### C.2 `crates/cc-replay` + `apps/replay-mission`
Reads a `cc-mission-log` mission dir → k-way merge-sorts the per-stream Parquet
parts by `cc_receive_time_ns` (with `StreamId`/seq tie-break) into one ordered
`TelemetryEvent` stream → drives the **same** Runner. Modes: `run` (finding
timeline → JSON + `ai_health.parquet`), `diff` (two builds → nonzero exit on any
byte difference in the canonical finding stream), `audit` (aggregate FP stats
over benign missions). Determinism test: run fixture twice → equal SHA-256.
Every recorded SITL/bench mission becomes a committed fixture with a golden hash.

### C.3 companiond integration (D1)
`ReportSource` gains `tick_with_conclusion(HealthConclusion, now, acked, tel)`;
companiond spawns the `cc-ai-health` Runner (a second lossy broadcast subscriber)
whose merged conclusion drives it, replacing the scripted `Scenario` in prod.
The `HealthConclusion` extends the v0 `Sample` with `detail_code/value/limit`
(v0 hardcoded `detail_code:0`), and the conclusion-level de-escalation smoothing
is **added** (v0's Hysteresis only paced the rate — judge fix).

### C.4 The false-positive audit is the gate on autonomy
Synthetic traces that inject a detector's own signature are circular; the audit
runs `cc-replay audit` over recorded **benign** missions — including windy hover
and aggressive-but-healthy segments — and asserts ~zero WARN/CRITICAL. **The FC
monitor stays advisory (`CC_MON_CRIT_ACT` warn-only) on these algorithms until a
documented audit passes** (dev-plan exit criterion + the Phase-9 flight ladder).

### C.5 Dependencies & deviations
Rust-native: `cc-ingest`, `cc-protocol`, `cc-health-tx` (reuse), `cc-timesync`,
`cc-config`, `tokio` (runtime only), `sha2` (golden hash), `+ libm` (the one new
dep — deterministic cross-arch transcendentals). `cc-replay` adds
`cc-mission-log` + `arrow`/`parquet` (=59, pinned). **No ML** — every algorithm
is closed-form statistical/physics change-detection; a trained NN would add a
nondeterminism surface, a provenance problem, and unexplainable findings for
zero specificity gain. The ONNX door stays open per-algorithm only if a future
signal proves un-modelable analytically (none here), behind the same
determinism + FP-audit bar.

Deviations: **D1** HealthSource refactor of `ReportSource` (scripted source
retained for injection tests). **D2** thermal = battery+IMU only (no Jetson temp
in contract). **D3** `libm` added to the deliberately-pinned dep set. **D4**
`mission_risk` config surface. **D5** `CC_AI_DIAGNOSTIC` 2 Hz cap via
deterministic priority round-robin (the report path is unthrottled). **D6**
deadline guard is prod-only (measured host time → DEGRADED) and disabled in
replay (byte-identity over host speed). **D7** `link_quality` uses only in-stream
replayable fields (`seq_gap`, timestamp gaps) — never RTT, `IngestStats` atomics,
or the unlogged `StreamStale`/`LinkStatus` events.

---

## Part D — Results

### D.0 What shipped

Two crates and one app, all `cargo clippy`-clean:

| Component | What it is | Tests |
|---|---|---|
| `cc-ai-health` `stats/` | 5 pure deterministic primitives (`ewma`, `robust`, `cusum`, `page_hinkley`, `rls`) + helpers | 26 |
| `cc-ai-health` framework | `HealthAlgorithm` trait, `FlightPhaseTracker`, 10 Hz `Runner`, `merge`, detail codes | 12 |
| `cc-ai-health` `algos/` | the 8 algorithms (battery, vibration, estimator, gps, motor, link, thermal, mission) | 53 |
| `cc-ai-health` integration | full-registry benign trace + system determinism | 2 |
| `cc-replay` + `replay-mission` | mission-dir → ordered events → Runner → timeline; run/diff/audit | 4 |
| `cc-health-tx` (Phase-7 additions) | `Conclusion` + `tick_with_conclusion` + `spawn_ai` | 3 of 13 |

**`cc-ai-health` total: 93 tests. Whole workspace: green.**

### D.1 The determinism spine held

Every algorithm ships a **double-run byte-identity** test, and two system-level
proofs close the loop:

* `benign_flight_produces_no_warn_or_critical` — all 8 algorithms driven through
  the `Runner` over a healthy 45 s multi-stream trace produce **zero**
  WARN/CRITICAL (the pipeline-level false-positive proof).
* `full_registry_replay_is_byte_identical` — the same trace FNV-hashes
  identically across two runs.
* `cc-replay` `parquet_roundtrip_reproduces_in_memory_findings` — a benign
  mission written as real `cc-mission-log` Parquet parts, read back, and
  replayed yields **the same SHA-256** as replaying the in-memory events: the
  Parquet reconstruction is lossless *for findings*, and
  `run_mission(dir).hash == run_mission(dir).hash` on re-run.

The split that makes this hold — **state folds in `on_event` (always runs);
`evaluate(&self)` is a pure read** — is enforced by the trait signature, so a
slow or skipped evaluate cannot perturb a later finding.

### D.2 Bugs found and fixed while building

* **Inverted Page-Hinkley `Direction::Down`** (latent in the committed
  `stats/` core): `contrib = mean − x − delta` made a *steady* signal drift the
  statistic down unboundedly, tripping after ~`lambda/delta` samples — the old
  "down" test only "passed" because it fed a constant `−1.0`. Corrected to the
  symmetric `contrib = x − mean + delta`; added a steady-never-trips regression
  across both directions. This was surfaced by the battery sag detector
  false-positiving on a flat healthy residual.
* **`RobustScale::z` NaN cascade** in `gps_quality`: `z` is NaN on an empty
  window and `NaN < STEP` is `false`, so the first adaptive-baseline update was
  skipped and the window stayed empty forever. Guarded so warm-up populates the
  baseline while a genuine step is still withheld.
* **Thermal rate-floor too low**: a 40 °C rate-arming floor let a fast cold-start
  warm-up ramp trip runaway; raised to above normal operating temperature so
  `dT/dt` only arms when the pack/sensor is genuinely hot.

### D.3 The false-positive audit gate (unchanged exit criterion)

`replay-mission audit <benign missions…>` aggregates WARN/CRITICAL over recorded
benign flights and **fails on any CRITICAL or > 0.5 % WARN**. The synthetic
benign traces already return zero findings; the audit over *recorded* windy-hover
and aggressive-but-healthy SITL/flight missions is the real gate. **Until a
documented audit passes, every CRITICAL here is advisory — the FC monitor stays
warn-only on these lanes** (`CC_MON_*`), and the live `--ai-health` source logs
itself as "advisory until FP audit". This is the Phase-9 flight-ladder
precondition, not a Phase-7 claim.

### D.4 Deviations — as realized

| Dev | Status |
|---|---|
| **D1** | `ReportSource::tick_with_conclusion` + `spawn_ai`; companiond `--ai-health` drives the live Runner as a third lossy subscriber. Scripted `--health-scenario` retained for the SITL suite. Conclusion-level de-escalation smoothing **added** (v0 only paced the rate). |
| **D2** | thermal = battery + IMU only (no Jetson SoC temp in the contract). |
| **D3** | `libm` added — the one new dep, for bit-reproducible cross-arch `log`/`exp`. |
| **D4** | `mission_risk` capacity / reserve / nominal-RTL-speed are module constants, flagged as the `cc-config` surface. |
| **D5** | `CC_AI_DIAGNOSTIC` scheduling deferred (report path carries `detail_code`/`value`/`limit` sources today). |
| **D6** | no host-time deadline guard was added; the pure `on_event`/`evaluate` split already removes the nondeterminism it was meant to catch. |
| **D7** | `link_quality` uses only replayable in-stream fields (`seq_gap`, inter-arrival vs `nominal_period_ns`) — never RTT, `IngestStats`, or the unlogged `StreamStale`/`LinkStatus`. Enforced structurally: the Runner ignores those variants. |

Design simplification vs the blueprint: **no `window.rs`** — each detector keeps
exactly the O(1) streaming state it needs, which is cheaper and a cleaner
determinism story than a shared ring; and `cc-replay` decodes only the six
periodic streams (Event/SafetyStatus cannot change a finding).

### D.5 Not yet done (follow-on)

* The benign-corpus FP audit over **recorded** missions (needs a corpus of
  windy/aggressive benign SITL + flight logs) — the gate on arming.
* `CC_AI_DIAGNOSTIC` emission (value/limit evidence stream) + `ai_health.parquet`
  sink (D5).
* Live SITL validation of the `--ai-health` path end-to-end (host build + tests
  pass; a SITL soak is the next integration step).
