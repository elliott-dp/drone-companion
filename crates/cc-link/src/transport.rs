//! Transport halves: UDP (SITL / Ethernet, spec §2.2) and serial (TELEM3,
//! spec §2.1 — compiled now, bench-verified in Phase 8). Same interface for
//! both, as the dev plan requires; an enum keeps it dyn-free.
//!
//! UDP mirrors the topology Phase 3 proved: bind the local port (PX4's
//! CCFC instance sends here, default 24040), learn the peer address from
//! the first inbound datagram, transmit back to it. A configured remote
//! overrides peer learning (real Ethernet deployments, spec §2.2 static
//! IPs).

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::UdpSocket;
use tokio::sync::watch;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub enum RxHalf {
    Udp(Arc<UdpSocket>),
    Serial(ReadHalf<SerialStream>),
}

pub enum TxHalf {
    Udp {
        sock: Arc<UdpSocket>,
        peer_rx: watch::Receiver<Option<SocketAddr>>,
    },
    Serial(WriteHalf<SerialStream>),
}

impl RxHalf {
    /// Receive bytes; UDP also reports the datagram's source address so the
    /// RX task can learn the peer — but ONLY from datagrams that decode to
    /// valid frames (an arbitrary sender must not be able to hijack the
    /// peer by spraying garbage at our port).
    pub async fn recv(&mut self, buf: &mut [u8]) -> io::Result<(usize, Option<SocketAddr>)> {
        match self {
            RxHalf::Udp(sock) => {
                let (n, from) = sock.recv_from(buf).await?;
                Ok((n, Some(from)))
            }
            RxHalf::Serial(r) => Ok((r.read(buf).await?, None)),
        }
    }
}

impl TxHalf {
    pub async fn send(&mut self, frame: &[u8]) -> io::Result<()> {
        match self {
            TxHalf::Udp { sock, peer_rx } => {
                let peer = *peer_rx.borrow();
                match peer {
                    Some(addr) => {
                        sock.send_to(frame, addr).await?;
                        Ok(())
                    }
                    // no peer yet: nothing to talk to — drop silently is
                    // wrong; report so the caller counts a tx_error
                    None => Err(io::Error::new(io::ErrorKind::NotConnected, "no peer yet")),
                }
            }
            TxHalf::Serial(w) => {
                w.write_all(frame).await?;
                Ok(())
            }
        }
    }
}

/// Bind a UDP transport. The peer is learned by the RX task from the first
/// VALIDLY-DECODING datagram unless `remote` pins it. Returns the peer
/// watch sender for the link's RX task.
pub async fn udp(
    bind: SocketAddr,
    remote: Option<SocketAddr>,
) -> io::Result<(RxHalf, TxHalf, watch::Sender<Option<SocketAddr>>)> {
    let sock = Arc::new(UdpSocket::bind(bind).await?);
    let (peer_tx, peer_rx) = watch::channel(remote);
    Ok((
        RxHalf::Udp(sock.clone()),
        TxHalf::Udp { sock, peer_rx },
        peer_tx,
    ))
}

/// Open a serial transport, 8N1, no flow control (spec §2.1 defaults).
/// The peer sender is vestigial for serial (point-to-point wire).
pub fn serial(
    path: &str,
    baud: u32,
) -> io::Result<(RxHalf, TxHalf, watch::Sender<Option<SocketAddr>>)> {
    let stream = tokio_serial::new(path, baud).open_native_async()?;
    let (r, w) = tokio::io::split(stream);
    let (peer_tx, _keep) = watch::channel(None);
    Ok((RxHalf::Serial(r), TxHalf::Serial(w), peer_tx))
}
