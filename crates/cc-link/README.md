# cc-link

The FC↔CC link layer: owns exactly one transport at a time (UDP for
SITL/Ethernet, serial for the TELEM3 bench link), decodes inbound bytes into
dialect frames, drains four strict-priority outbound queues, sends the
companion's own heartbeat, and derives link state from FC heartbeat age and
CRC-error rate. Built on `tokio`.

## Scope

| In this crate | Not in this crate |
|---|---|
| Transport ownership (UDP/serial), one at a time | Frame decoding itself — delegates to `cc-protocol::CcFrameDecoder` |
| Strict-priority TX queuing (4 classes) | Sequence-continuity / staleness tracking of *inbound* streams — `cc-ingest` |
| Companion HEARTBEAT emission | Telemetry semantics / typed events — `cc-ingest` |
| Link state (Up/Degraded/Down) from heartbeat age + CRC rate | Mission recording — `cc-mission-log` |
| Process-monotonic clock (`clock::now_ns`) shared by downstream crates | |

## Architecture

`spawn`/`spawn_with_raw_tap` starts four `tokio` tasks sharing state through
`Arc`/`watch`/`mpsc` channels — there is no actor loop to step manually; the
returned `Link` struct is the caller's only handle.

```text
  transport bytes                              TxHandle::enqueue(prio, msg)
        │                                                │
        ▼                                                ▼
  ┌───────────┐   CcFrameDecoder    ┌─────────────┐   4 bounded mpsc queues
  │  rx_task  │ ─────────────────►  │  frame_tx   │   (P0..P3, strict priority)
  └───────────┘   + source gate     │  mpsc(512)  │        │
        │                           └─────────────┘        ▼
        │ updates last_fc_hb                          ┌───────────┐
        ▼                                              │  tx_task  │ ──► transport
  ┌────────────┐  heartbeat age + crc rate             └───────────┘
  │ state_task │ ─────────────────► watch<LinkStatus>
  └────────────┘

  heartbeat_task: enqueues a P0 HEARTBEAT every 1s
```

### RX path (`rx_task`)

1. Read bytes from the transport (`RxHalf::recv`).
2. If a raw tap sender was supplied (`spawn_with_raw_tap`), the exact wire
   bytes are `try_send`-copied to it **before** decoding — this is how
   `cc-mission-log`'s `raw_mavlink.bin` gets ground-truth bytes independent
   of the decoder. The send is lossy (drops silently if the channel is full
   or absent) so it can never back-pressure the RX path.
3. Feed bytes to `CcFrameDecoder::push`; for every decoded frame, run
   `cc_protocol::validate::validate_source_on_cc` (rejects wrong system ID,
   or a message from the wrong component for its direction) and count
   `rx_bad_source` on failure.
4. A decoded `HEARTBEAT` updates `last_fc_hb` (an atomic timestamp read by
   `state_task`).
5. Forward the frame on a bounded channel (`FRAME_DEPTH = 512`) to the
   consumer (companiond's demux → `cc-ingest`); a full channel counts
   `rx_channel_drops` rather than blocking.
6. For UDP, the peer address is learned **only from a datagram that decoded
   to at least one valid frame** — an arbitrary sender spraying garbage at
   the bound port cannot hijack where the TX side replies.

### TX path (`tx_task`)

Four `mpsc` queues, drained by a `tokio::select!` with `biased;` — polled
top-down on every wakeup, so P0 always wins when non-empty and a lower
class only transmits when every class above it is empty **at that frame
boundary** (preemptive per-frame, not per-burst). Queue depths: P0 is
deliberately shallow (`P0_DEPTH = 16` — a deep P0 would hide a dead link),
P1–P3 are `P_DEPTH = 128`.

```rust
pub enum Priority {
    P0, // HEARTBEAT, TIMESYNC, CC_HEALTH_REPORT — never silently dropped
    P1, // CC_MISSION_CONTEXT, acks/session
    P2, // CC_AI_DIAGNOSTIC
    P3, // bulk/debug (development only)
}
```

`TxHandle::enqueue` uses `try_send`. A full **P0** queue is treated as a
**link-down condition, not a queueing condition**: it increments
`p0_stalls` and pushes `true` on an internal `link_down` watch channel that
`state_task` observes immediately (not just on its 500 ms tick). A full
P1–P3 queue drops the message and counts `tx_errors`.

### Heartbeat (`heartbeat_task`)

Emits `HEARTBEAT { mavtype: MAV_TYPE_ONBOARD_CONTROLLER, autopilot:
MAV_AUTOPILOT_INVALID, system_status: MAV_STATE_ACTIVE }` at 1 Hz on P0.

### Link state (`state_task`)

Recomputed every 500 ms, or immediately on a P0 stall:

```rust
pub enum LinkState { Down, Up, Degraded }
```

- `Down` — no FC heartbeat ever seen, heartbeat age > 5 s (`HB_DOWN_NS`), or
  a P0 stall is currently flagged.
- `Degraded` — heartbeat age > 2.5 s (`HB_DEGRADED_NS`).
- `Up` — otherwise.

Published on a `watch::Receiver<LinkStatus>`:

```rust
pub struct LinkStatus {
    pub state: LinkState,
    pub fc_heartbeat_age_ns: Option<i64>,
    pub crc_errors: u64,
}
```

`cc-ingest` forwards this into the `TelemetryEvent` stream as `LinkStatus`.

## Public API

```rust
pub enum Priority { P0, P1, P2, P3 }
pub enum LinkState { Down, Up, Degraded }
pub struct LinkStatus { pub state: LinkState, pub fc_heartbeat_age_ns: Option<i64>, pub crc_errors: u64 }
pub struct LinkCounters { /* atomics: rx_bad_source, tx_frames, tx_errors, p0_stalls, rx_channel_drops */ }
pub struct LinkStatsSnapshot { /* merged decoder + link counters, point-in-time */ }
pub type LinkFrame = cc_protocol::framing::DecodedFrame<CcMavMessage>;

pub struct TxHandle { /* Clone */ }
impl TxHandle {
    pub fn enqueue(&self, prio: Priority, msg: CcMavMessage);
}

pub struct Link {
    pub tx: TxHandle,
    pub status: watch::Receiver<LinkStatus>,
    pub counters: Arc<LinkCounters>,
    // ...
}
impl Link {
    pub fn take_frames(&mut self) -> mpsc::Receiver<LinkFrame>; // once
    pub fn stats(&self) -> LinkStatsSnapshot;
}

pub fn spawn(rx_half, tx_half, peer_tx, sysid: u8) -> Link;
pub fn spawn_with_raw_tap(rx_half, tx_half, peer_tx, sysid: u8, raw_tap: Option<mpsc::Sender<Vec<u8>>>) -> Link;
```

### `transport`

```rust
pub enum RxHalf { Udp(Arc<UdpSocket>), Serial(ReadHalf<SerialStream>) }
pub enum TxHalf { Udp { sock, peer_rx }, Serial(WriteHalf<SerialStream>) }

pub async fn udp(bind: SocketAddr, remote: Option<SocketAddr>)
    -> io::Result<(RxHalf, TxHalf, watch::Sender<Option<SocketAddr>>)>;
pub fn serial(path: &str, baud: u32)
    -> io::Result<(RxHalf, TxHalf, watch::Sender<Option<SocketAddr>>)>;
```

UDP binds the local port and learns the peer from the first validly-decoding
inbound datagram unless `remote` pins it (static-IP deployments). Serial
opens 8N1, no flow control, and is the same enum-dispatched interface as UDP
— `rx_task`/`tx_task` are transport-agnostic. Serial is compiled and
constructible now; bench verification against a real TELEM3 UART is Phase 8
scope (no serial reconnect-with-backoff logic yet — that lands with the
bench work). A dropped/errored UDP `recv` backs off 100 ms and retries
(transient by nature); the same retry loop currently also covers serial
errors.

### `clock`

```rust
pub fn now_ns() -> i64;
```

Process-monotonic nanoseconds since first call (epoch = process start via
`OnceLock<Instant>`). This is the single time source every companion crate
stamps `cc_receive_time_ns` against (identity envelope, spec §3.4) — never
wall-clock time, so ages and offsets stay comparable regardless of NTP/RTC
state. Wall time only enters the system in `cc-mission-log`, paired
alongside the monotonic value for human-readable logs.

## Testing

`tests/link_behavior.rs` spins up a real `Link` over loopback UDP against a
simulated FC peer (`FcPeer`, a raw socket the test drives directly) and
covers:

- `rx_decodes_and_source_gate_drops` — a valid FC-sourced frame is delivered
  to the consumer; a frame from the wrong component is dropped and counted.
- `garbage_between_frames_is_resynced_and_counted` — non-frame bytes
  interleaved with valid frames are absorbed by the decoder's resync logic
  without losing the valid frames.
- `companion_heartbeat_flows_at_one_hz_and_link_state_tracks_fc` — the
  companion's own HEARTBEAT is observed by the peer at ~1 Hz, and
  `LinkStatus.state` transitions correctly as the simulated FC's heartbeats
  start, stop, and resume.

## Dependencies

`cc-protocol` (frame decoding, validation, dialect types), `tokio` (runtime,
channels, UDP), `tokio-serial` (serial transport).
