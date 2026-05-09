//! Forza Motorsport / Horizon UDP telemetry decoder, enricher, and recorder.
//!
//! The crate is organized around four boundaries:
//!
//! - [`decoder`] turns raw UDP bytes into a typed [`decoder::RawPacket`].
//! - `stream` (added in later phases) provides UDP and CSV-replay sources.
//! - `aggregator` (added in later phases) maintains a temporal window and
//!   enriches each packet with derivations the simulator does not emit
//!   directly (signed relative travels, body slip, normal loads, etc.).
//! - `csv_writer` (added in later phases) serializes enriched frames to the
//!   v2 CSV schema described in the project plan.
//!
//! The binary in `src/bin/record.rs` is a thin CLI on top of these layers.

pub mod aggregator;
pub mod car_db;
pub mod csv_writer;
pub mod decoder;
pub mod stream;

pub use decoder::{DecodeError, RawPacket, Source, Wheel, decode};
