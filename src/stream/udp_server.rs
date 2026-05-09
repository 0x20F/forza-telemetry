//! Async UDP listener for Forza Data Out packets.
//!
//! A single `serve` loop binds, then for each incoming datagram calls the
//! provided callback with a borrowed byte slice and a monotonic receive
//! timestamp (nanoseconds since `serve` started). The caller owns shutdown:
//! pass any future (typically `ctrl_c()`) and the loop exits cleanly when it
//! resolves.
//!
//! Buffer is fixed at 2048 bytes; the largest documented Forza packet is
//! 331 bytes, so we have ~6x headroom for any future format growth.

use std::future::Future;
use std::io;
use std::time::Instant;

use tokio::net::{ToSocketAddrs, UdpSocket};

const BUFFER_BYTES: usize = 2048;

/// Run a UDP receive loop until `shutdown` resolves.
///
/// `on_packet` is called for every received datagram with the bytes (slice
/// into an internally-owned buffer) and the elapsed nanoseconds since the
/// loop started. The callback is `FnMut`, so it can mutate any state the
/// caller passes in (e.g. a `SessionWriter`).
pub async fn serve<A, F, S>(addr: A, mut on_packet: F, shutdown: S) -> io::Result<()>
where
    A: ToSocketAddrs,
    F: FnMut(&[u8], u64),
    S: Future<Output = ()>,
{
    let socket = UdpSocket::bind(addr).await?;
    let mut buf = vec![0u8; BUFFER_BYTES];
    let started = Instant::now();

    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            res = socket.recv(&mut buf) => {
                let n = res?;
                let recv_time_ns = started.elapsed().as_nanos() as u64;
                on_packet(&buf[..n], recv_time_ns);
            }
        }
    }
}
