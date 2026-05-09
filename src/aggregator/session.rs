//! `AggregatorSession` ties the temporal learners together and produces an
//! `EnrichedFrame` per ingested `RawPacket`.
//!
//! Phase 4 wires up:
//!
//! - Body slip angle (`atan2(vx, vz)` above the speed gate).
//! - Lowpassed longitudinal acceleration (Z-forward axis).
//! - Lowpassed lateral acceleration (X-right axis).
//! - Lowpassed yaw rate (Y-axis angular velocity).
//!
//! Later phases will hang the ride-height learner, normal-load estimator,
//! wheel-radius learner and steady-state classifier off the same session.

use crate::car_db::CarCalibration;
use crate::csv_writer::EnrichedFrame;
use crate::decoder::RawPacket;

use super::normal_load;
use super::slip_angle::body_slip_angle;
use super::smoothing::Ema;
use super::steady_state::Classifier as SteadyStateClassifier;
use super::suspension::RideHeightLearner;
use super::wheel_radius::RadiusLearner;

/// Time constant used for the visual lowpasses (longitudinal/lateral
/// acceleration, yaw rate). 200 ms is the sweet spot for look-dev
/// smoothing: removes per-frame noise without lagging visible transients.
const VISUAL_LOWPASS_TAU_S: f32 = 0.2;

/// Stateful per-session aggregator. Construct one per recording; the
/// learners assume monotonic ingestion.
#[derive(Debug)]
pub struct AggregatorSession {
    last_recv_time_ns: Option<u64>,
    accel_long_lp: Ema,
    accel_lat_lp: Ema,
    yaw_rate_lp: Ema,
    ride_height: RideHeightLearner,
    radius: RadiusLearner,
    steady_state: SteadyStateClassifier,
    calibration: CarCalibration,
}

impl Default for AggregatorSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatorSession {
    pub fn new() -> Self {
        Self::with_calibration(CarCalibration::default())
    }

    /// Construct a session that already knows the static vehicle parameters
    /// for normal-load reconstruction. When the calibration is empty (all
    /// `None`), normal load columns stay empty in the resulting CSV.
    pub fn with_calibration(calibration: CarCalibration) -> Self {
        Self {
            last_recv_time_ns: None,
            accel_long_lp: Ema::new(VISUAL_LOWPASS_TAU_S),
            accel_lat_lp: Ema::new(VISUAL_LOWPASS_TAU_S),
            yaw_rate_lp: Ema::new(VISUAL_LOWPASS_TAU_S),
            ride_height: RideHeightLearner::new(),
            radius: RadiusLearner::new(),
            steady_state: SteadyStateClassifier::new(),
            calibration,
        }
    }

    pub fn calibration(&self) -> &CarCalibration {
        &self.calibration
    }

    /// Phase-7 learned values, useful for the sidecar TOML written at
    /// session close.
    pub fn learned_radii(&self) -> (Option<f32>, Option<f32>) {
        (self.radius.radius_f(), self.radius.radius_r())
    }

    /// Compute every Phase-4 derivation and produce an EnrichedFrame ready
    /// to be written to CSV.
    pub fn ingest(&mut self, packet: &RawPacket) -> EnrichedFrame {
        let dt_s = match self.last_recv_time_ns {
            Some(prev) if packet.recv_time_ns > prev => {
                ((packet.recv_time_ns - prev) as f64 / 1_000_000_000.0) as f32
            }
            _ => 0.0,
        };
        self.last_recv_time_ns = Some(packet.recv_time_ns);

        // Forza convention: car-local X = right, Y = up, Z = forward,
        // angular velocity X = pitch, Y = yaw, Z = roll.
        let lp_long = self.accel_long_lp.update(dt_s, packet.acceleration_local[2]);
        let lp_lat = self.accel_lat_lp.update(dt_s, packet.acceleration_local[0]);
        let lp_yaw = self.yaw_rate_lp.update(dt_s, packet.angular_velocity[1]);

        // Phase 5: ride-height learner uses raw (unsmoothed) chassis
        // signals so it gates on the actual quiescent state, not on lagged
        // values that would let active frames sneak in.
        self.ride_height.ingest(
            packet.suspension_travel_meters,
            packet.acceleration_local[0],
            packet.acceleration_local[2],
            packet.angular_velocity[1],
        );

        let mut frame = EnrichedFrame::from_raw(packet);
        frame.body_slip_angle_rad = body_slip_angle(packet.velocity_local);
        frame.acceleration_long_lp = Some(lp_long);
        frame.acceleration_lat_lp = Some(lp_lat);
        frame.yaw_rate_lp = Some(lp_yaw);

        let baseline = self.ride_height.baseline();
        frame.ride_height_baseline_fl = baseline.fl;
        frame.ride_height_baseline_fr = baseline.fr;
        frame.ride_height_baseline_rl = baseline.rl;
        frame.ride_height_baseline_rr = baseline.rr;

        let rel = self.ride_height.relative_travel(packet.suspension_travel_meters);
        frame.suspension_travel_relative_fl = rel.fl;
        frame.suspension_travel_relative_fr = rel.fr;
        frame.suspension_travel_relative_rl = rel.rl;
        frame.suspension_travel_relative_rr = rel.rr;

        // Phase 6: per-corner normal load reconstruction. We feed raw
        // accelerations rather than the lowpassed values so the load
        // estimate matches the same instantaneous chassis state the rest
        // of the row describes.
        if let Some(load) = normal_load::estimate(
            packet.acceleration_local[0],
            packet.acceleration_local[2],
            &self.calibration,
        ) {
            frame.normal_load_fl = Some(load.fl);
            frame.normal_load_fr = Some(load.fr);
            frame.normal_load_rl = Some(load.rl);
            frame.normal_load_rr = Some(load.rr);
        }

        // Phase 7: wheel-radius auto-cal + steady-state classification.
        if let (Some(speed), Some(accel_n), Some(brake_n)) = (
            packet.speed,
            frame.accel_normalized,
            frame.brake_normalized,
        ) {
            self.radius.ingest(
                speed,
                packet.wheel_rotation_speed,
                packet.drivetrain_type,
                accel_n,
                brake_n,
            );
        }
        frame.wheel_radius_learned_f = self.radius.radius_f();
        frame.wheel_radius_learned_r = self.radius.radius_r();

        let st = self.steady_state.classify(packet.recv_time_ns, packet);
        frame.steady_state_flags = Some(st.flags);
        frame.time_in_state_ms = Some(st.time_in_state_ms);

        frame
    }
}
