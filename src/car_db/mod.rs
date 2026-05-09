//! Static per-car calibration data.
//!
//! Forza emits dynamic state but no static vehicle parameters (mass, COG
//! height, track widths, etc). The aggregator needs those to reconstruct
//! per-corner normal loads. This module loads a hand-curated TOML keyed by
//! CarOrdinal at compile time, with optional per-session overrides from a
//! user-provided file.

mod lookup;

pub use lookup::{lookup, CarCalibration, BUNDLED_DEFAULTS};
