//! Async half of cc-timesync: request cadence, reply intake, snapshot
//! publication, reboot invalidation (spec §5.4).

use std::time::Duration;

use cc_link::{clock, Priority, TxHandle};
use cc_protocol::cc_dialect::{MavMessage, TIMESYNC_DATA};
use tokio::sync::{mpsc, watch};

use crate::{Filter, Snapshot};

/// Fast-lock: 10 Hz for the first 5 s after start/invalidation (spec §5.4).
const FAST_PERIOD: Duration = Duration::from_millis(100);
const FAST_WINDOW: Duration = Duration::from_secs(5);
/// Steady state: 1 Hz.
const SLOW_PERIOD: Duration = Duration::from_secs(1);

/// A TIMESYNC reply routed here by companiond's demux.
#[derive(Debug, Clone, Copy)]
pub struct Reply {
    pub tc1_ns: i64,
    pub ts1_ns: i64,
    /// CC receive time (cc-link clock) of the reply frame.
    pub rx_ns: i64,
}

pub struct Runner {
    pub replies: mpsc::Sender<Reply>,
    pub snapshot: watch::Receiver<Snapshot>,
}

/// Spawn the timesync task.
///
/// * `tx` — link TX handle; requests go out at P0 (spec §4: safety traffic
///   outranks data).
/// * `boot_id` — watch fed by cc-ingest; any change invalidates the filter
///   and re-enters fast-lock (spec §5.4 "FC reboot detection").
pub fn spawn(tx: TxHandle, mut boot_id: watch::Receiver<u32>) -> Runner {
    let (reply_tx, mut reply_rx) = mpsc::channel::<Reply>(64);
    let (snap_tx, snap_rx) = watch::channel(Snapshot::UNLOCKED);

    tokio::spawn(async move {
        let mut filter = Filter::new();
        let mut lock_phase_start = clock::now_ns();
        let mut last_tc1: i64 = 0;

        loop {
            let elapsed = clock::now_ns() - lock_phase_start;
            let period = if elapsed < FAST_WINDOW.as_nanos() as i64 {
                FAST_PERIOD
            } else {
                SLOW_PERIOD
            };

            tokio::select! {
                _ = tokio::time::sleep(period) => {
                    // request: tc1 = 0, ts1 = our monotonic ns (spec §5.4)
                    tx.enqueue(Priority::P0, MavMessage::TIMESYNC(TIMESYNC_DATA {
                        tc1: 0,
                        ts1: clock::now_ns(),
                    }));
                }

                Some(r) = reply_rx.recv() => {
                    // FC timestamp regression = reboot we haven't heard
                    // about via boot_id yet: invalidate (spec §5.4)
                    if last_tc1 != 0 && r.tc1_ns < last_tc1 - 1_000_000_000 {
                        filter.invalidate();
                        lock_phase_start = clock::now_ns();
                    }
                    last_tc1 = r.tc1_ns;

                    filter.add_reply(r.tc1_ns, r.ts1_ns, r.rx_ns);
                    let _ = snap_tx.send_replace(filter.estimate());
                }

                res = boot_id.changed() => {
                    if res.is_err() { return; }
                    filter.invalidate();
                    last_tc1 = 0;
                    lock_phase_start = clock::now_ns();
                    let _ = snap_tx.send_replace(Snapshot::UNLOCKED);
                }
            }
        }
    });

    Runner { replies: reply_tx, snapshot: snap_rx }
}
