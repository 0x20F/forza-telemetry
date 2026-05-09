//! Temporal enrichment of decoded packets.
//!
//! The aggregator runs after the decoder and before the CSV writer. It is
//! pure (no I/O, no async, no global state) so the same code path serves
//! the live UDP loop and offline replays.
//!
//! Phase 4 covers the basics: a fixed-size rolling buffer, EMAs for the
//! visual lowpasses (`acceleration_long_lp`, `acceleration_lat_lp`,
//! `yaw_rate_lp`) and a body slip angle. Later phases hang ride-height,
//! normal-load, wheel-radius and steady-state learners off the same
//! `AggregatorSession`.

pub mod normal_load;
pub mod session;
pub mod slip_angle;
pub mod smoothing;
pub mod steady_state;
pub mod suspension;
pub mod wheel_radius;
pub mod window;

pub use session::AggregatorSession;
pub use smoothing::Ema;
pub use steady_state::Classifier as SteadyStateClassifier;
pub use suspension::RideHeightLearner;
pub use wheel_radius::RadiusLearner;
