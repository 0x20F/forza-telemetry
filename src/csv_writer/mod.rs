//! v2 CSV writer.
//!
//! The v2 schema covers raw decoded fields, cheap per-frame normalizations
//! (steer/pedal/ratio columns), and reserved columns for temporal
//! derivations populated by later aggregator phases. Each capture starts
//! with a comment line declaring the schema version and source format so
//! downstream tools never have to guess.

mod enriched;
mod session_writer;

pub use enriched::EnrichedFrame;
pub use session_writer::{LearnedValues, SessionMeta, SessionSummary, SessionWriter};

/// Schema version embedded in the comment header of every capture file.
pub const SCHEMA_VERSION: u32 = 2;
