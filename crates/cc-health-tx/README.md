# cc-health-tx

The companion health-report transmitter: turns a health conclusion — either
a scripted scenario timeline (v0) or a live `cc-ai-health` `Conclusion`
(Phase 7) — into rate-policed, hysteresis-smoothed `CC_HEALTH_REPORT`
messages, tracking the FC-side monitor's acknowledgment
(`CC_SAFETY_STATUS.last_report_sequence`).

## Scope

| In this crate | Not in this crate |
|---|---|
| Report pacing (severity-dependent rate) | Deciding *what* the severity/action should be — `cc-ai-health` (Phase 7) or a scenario file (v0) |
| Severity hysteresis (de-escalation debounce) | Consuming the report — PX4's `cc_safety_monitor` (separate repo) |
| ACK tracking against the monitor's echo | Sending the message on the wire — delegates to `cc-link::TxHandle` |
| Scripted scenario parsing (v0 test/demo source) | |

## Why this exists

The FC-side safety monitor (`cc_safety_monitor`) needs a *paced, bounded*
signal, not a raw firehose: too slow and a real CRITICAL takes too long to
land; too fast and it competes with telemetry for link bandwidth; too
twitchy and a momentarily-noisy detector flaps the vehicle's behavior. This
crate is the layer that turns "the current best health conclusion" (however
it was computed) into that disciplined signal.

## Rate policy (`policy::interval_ns`)

| Severity | Interval | |
|---|---|---|
| `Critical`, unacknowledged | 200 ms (5 Hz) | immediate on transition into `Critical`, then repeats until acked |
| `Critical`, acknowledged | 1000 ms (1 Hz) | keepalive while the condition persists |
| `Warn` | 250 ms (4 Hz) | within the spec's 2–5 Hz band |
| `Ok`, `Stale` | 1000 ms (1 Hz) | |

"Acknowledged" means the monitor's echoed `CC_SAFETY_STATUS.last_report_sequence`
has caught up to the sequence number of the last report this crate sent. A
severity **change** (an "edge") always sends immediately regardless of the
interval — a CRITICAL onset is never delayed by a stale 1 Hz cadence left
over from OK.

## Severity hysteresis (`policy::Hysteresis`)

```rust
pub struct Hysteresis { /* debounce_ns, current, pending */ }
impl Hysteresis {
    pub fn new(debounce_ns: i64) -> Self;
    pub fn current(&self) -> Severity;
    pub fn apply(&mut self, raw: Severity, now_ns: i64) -> Severity;
}
```

**Escalation is immediate**; **de-escalation is held for `debounce_ns`** of
continuous lower-severity input before being adopted. A severity that flaps
back up during the debounce window resets the pending de-escalation's timer
entirely (it must hold for the *full* debounce from the point it last
stabilized, not accumulate partial credit across flaps). This exists so a
detector that flickers CRITICAL→OK→CRITICAL for one sample does not bounce
the reported severity (and therefore the FC's response) on every flicker.
The monitor has its own independent `OK_COUNT` recovery hysteresis on the FC
side; the two debounce windows are configured so they don't fight each
other (neither masks the other's effect).

## Public API

### `ReportSource` — the pacing/hysteresis/ACK state machine

```rust
pub struct SelfTelemetry {
    pub link_rtt_ms: u16, pub telemetry_age_ms: u16,
    pub companion_loop_ms: u16, pub dropped_rx_count: u16,
}
pub struct Conclusion {
    pub severity: Severity, pub action: Action,
    pub health_flags: u32, pub detail_code: u16, pub confidence: u8,
}

impl ReportSource {
    pub fn new(scenario: Scenario, mission_id: u32, boot_id: u32, hysteresis_debounce_ns: i64) -> Self;
    pub fn last_sent_sequence(&self) -> u32;

    // v0: scripted scenario, sampled at `elapsed_ns` since scenario start.
    pub fn tick(&mut self, elapsed_ns: i64, now_ns: i64, last_acked_seq: u32, tel: SelfTelemetry)
        -> Option<CC_HEALTH_REPORT_DATA>;

    // Phase 7: driven by a live cc-ai-health Conclusion.
    pub fn tick_with_conclusion(&mut self, c: Conclusion, now_ns: i64, last_acked_seq: u32, tel: SelfTelemetry)
        -> Option<CC_HEALTH_REPORT_DATA>;
}
```

`Conclusion` is a **primitive-only** mirror of `cc-ai-health::HealthConclusion`
— this crate must not depend on `cc-ai-health` (that crate already depends
on this one for its `Severity`/`Action` types), so the caller flattens its
conclusion into this struct on the way in.

`tick` (v0) reports the scenario's scripted conclusion directly; the
hysteresis only governs the **report rate**, not the conclusion content —
appropriate for a deterministic, non-noisy scripted source.

`tick_with_conclusion` (Phase 7) goes further: the **conclusion itself** is
de-escalation-smoothed, not just its rate. While the smoothed severity is
still latched above the incoming one, the *held* conclusion (whichever one
last matched the smoothed severity) keeps being reported — so severity,
action, and detail code always correspond to each other, never an
inconsistent mix of an old severity with a new detail code. A live AI
detector is expected to be noisier than a scripted timeline, so this
stronger guarantee matters for it specifically.

Both methods:

1. Sample/accept the current conclusion (raw or AI) and pass it through
   `Hysteresis::apply`.
2. Compute `acked` from the monitor's echoed sequence vs. the last sequence
   this source sent.
3. Compute the due interval via `policy::interval_ns(smoothed, acked)`.
4. Send now if this is a severity edge, or the interval has elapsed since
   the last send; otherwise return `None`.
5. On send: increment the internal sequence, stamp
   `companion_timestamp_us`/`mission_id`/`companion_boot_id`, and build the
   full `CC_HEALTH_REPORT_DATA` (self-telemetry fields are informational —
   the monitor keys its decision on severity/action, not on
   `link_rtt_ms`/etc.).

### `spawn` / `spawn_ai` — the async wrapper

```rust
pub fn spawn(source: ReportSource, tx: TxHandle, ack_rx: watch::Receiver<u32>,
             tel_fn: impl Fn() -> SelfTelemetry + Send + 'static) -> JoinHandle<()>;

pub fn spawn_ai(source: ReportSource, tx: TxHandle, ack_rx: watch::Receiver<u32>,
                 conc_rx: watch::Receiver<Conclusion>,
                 tel_fn: impl Fn() -> SelfTelemetry + Send + 'static) -> JoinHandle<()>;
```

Both tick at 20 Hz internally (fine-grained enough to realize the 5 Hz
CRITICAL rate exactly) and enqueue any due report at TX **priority P1**
(below P0 heartbeat/timesync, above bulk — health reports matter, but not
more than link liveness). `spawn` drives `ReportSource::tick` from a
scripted `Scenario`; `spawn_ai` drives `tick_with_conclusion` from a
`watch::Receiver<Conclusion>` published by the `cc-ai-health` Runner.

### `scenario::Scenario` — the v0 scripted severity timeline

```rust
pub enum Severity { Ok = 0, Warn = 1, Critical = 2, Stale = 3 } // mirrors CC_SEVERITY, ordered
pub enum Action { None, WarnOnly, BlockOffboard, Hold, Land, Rtl } // mirrors CC_RECOMMENDED_ACTION

pub struct Sample { pub severity: Severity, pub action: Action, pub flags: u32, pub confidence: u8 }

impl Scenario {
    pub fn from_toml_str(s: &str) -> Result<Scenario, String>;
    pub fn nominal() -> Scenario;                        // one-shot OK, the default
    pub fn sample_at(&self, elapsed_ns: i64) -> Sample;   // last event at/before elapsed_ns
    pub fn is_empty(&self) -> bool;
}
```

TOML format — a list of timed events, sorted by `t_s` at parse time:

```toml
[[event]]
t_s = 0.0
severity = "ok"        # ok | warn | critical | stale
action = "none"        # none | warn_only | block_offboard | hold | land | rtl
flags = ["battery"]    # CC_HEALTH_FLAGS domains active
confidence = 100

[[event]]
t_s = 120.0
severity = "critical"
action = "land"
flags = ["battery"]
confidence = 90
```

`flags` names map to `CC_HEALTH_FLAGS` bits: `battery`=1, `motor`=2,
`vibration`=4, `gps`=8, `estimator`=16, `thermal`=32, `link`=64,
`mission`=128, `storage`=256, `ai_degraded`=512, `timesync`=1024.
`sample_at` returns the conclusion of the last event whose `t_s` has
passed — `Sample::default()` (OK) before the first event fires. This is the
crate's SITL-test and demo-harness drive mechanism: a scenario file scripts
a fault timeline without needing a real fault or a live AI detector.

## Testing

`lib.rs` (integration-level, exercising `ReportSource` end to end):

- `first_report_is_immediate_and_carries_the_conclusion` — the very first
  tick always sends, regardless of rate policy.
- `conclusion_carries_detail_code_and_flags` — the AI path (`tick_with_conclusion`)
  forwards `detail_code`/`health_flags`/`confidence`; v0's `tick` hard-codes
  `detail_code: 0`.
- `conclusion_deescalation_is_smoothed_but_escalation_is_immediate` — a
  single OK tick after a CRITICAL does not drop the reported severity or its
  detail code; only after the full debounce window does OK commit.
- `critical_repeats_at_5hz_until_acked_then_1hz` — verifies the exact rate
  transition at the moment the monitor's ACK catches up.
- `ok_scenario_paces_at_1hz` — steady-state OK reporting cadence.
- `severity_change_forces_an_immediate_report` — an edge always overrides
  the current rate interval.

`policy.rs`:

- `critical_rate_is_5hz_until_acked_then_1hz` — the interval table itself.
- `escalation_immediate_deescalation_debounced` — basic hysteresis behavior.
- `flap_resets_deescalation_timer` — a re-escalation during a pending
  de-escalation restarts the debounce from zero, rather than crediting
  partial elapsed time toward the original candidate.

## Dependencies

`cc-protocol` (`CC_HEALTH_REPORT_DATA`, schema version), `cc-link`
(`TxHandle`, `Priority::P1`, the shared clock), `serde` + `toml` (scenario
parsing), `tokio` (the spawn tasks). Dev-only: `tokio`'s `test-util` feature.
