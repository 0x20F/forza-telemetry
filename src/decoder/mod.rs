//! UDP packet decoder for the Forza Data Out protocol.
//!
//! Forza's `Data Out` feature emits little-endian, fixed-layout UDP packets
//! that come in four flavors distinguished by length. We dispatch on length
//! and decode into [`RawPacket`], a flat snapshot of the wire.
//!
//! - 232 bytes: Sled (FM7/FM/FH base telemetry only).
//! - 311 bytes: Dash (FM7 / FM 2023 base, sled + dash extras).
//! - 324 bytes: Dash Horizon (FH4 / FH5, sled + 12-byte gap + dash extras).
//! - 331 bytes: FM 2023 extras (Dash + 4 tire wear floats + track ordinal).

mod bytes;
pub mod formats;
mod packet;
mod units;

pub use formats::Source;
pub use packet::{RawPacket, Wheel};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unsupported packet length: {0} bytes")]
    UnsupportedLength(usize),
    #[error("packet truncated while reading offset {offset} ({needed} bytes needed)")]
    Truncated { offset: usize, needed: usize },
}

/// Decode a Forza Data Out UDP packet.
///
/// `recv_time_ns` is supplied by the listener so we have a monotonic reference
/// independent of Forza's own timestamp (which can wrap during long sessions).
pub fn decode(bytes: &[u8], recv_time_ns: u64) -> Result<RawPacket, DecodeError> {
    let source = Source::from_len(bytes.len())?;
    formats::decode_with_source(bytes, source, recv_time_ns)
}
