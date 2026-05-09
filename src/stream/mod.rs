//! Sources of `RawPacket` bytes.
//!
//! [`udp_server`] is the live ingress: a tokio UDP listener that calls a
//! callback for each datagram with a monotonic receive timestamp. The
//! aggregator and CSV writer are deliberately decoupled from the listener
//! so they can be driven by other sources later (CSV replay, tests).

pub mod udp_server;

pub use udp_server::serve;
