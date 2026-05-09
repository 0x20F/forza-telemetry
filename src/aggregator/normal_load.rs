//! Per-corner normal-load reconstruction.
//!
//! Forza does not publish per-tire normal force, so we reconstruct it from
//! the static weight distribution plus longitudinal and lateral load
//! transfer. This is a textbook two-track / quasi-static model:
//!
//! ```text
//! delta_long      = m * a_long * cog_height / wheelbase
//! delta_lat_axle  = m_axle * a_lat * cog_height / track_axle
//!
//! F_front = m_front * g - delta_long
//! F_rear  = m_rear  * g + delta_long
//! F_fl    = F_front/2 - delta_lat_f/2
//! F_fr    = F_front/2 + delta_lat_f/2
//! F_rl    = F_rear /2 - delta_lat_r/2
//! F_rr    = F_rear /2 + delta_lat_r/2
//! ```
//!
//! The model assumes Forza's car-local frame (X right, Y up, Z forward),
//! so a positive `a_lat` shifts load to the right and a positive `a_long`
//! (forward acceleration) shifts load rearward. Sums are exactly `m * g`
//! by construction.
//!
//! Returns `None` when any of `mass`, `cog_height`, `wheelbase`,
//! `front_weight_bias`, `track_f`, `track_r` is missing from the
//! calibration; partial reconstruction would be confusing in the CSV.

use crate::car_db::CarCalibration;
use crate::decoder::Wheel;

/// Earth gravity, m/s^2.
pub const GRAVITY_M_S2: f32 = 9.80665;

/// Estimate per-corner normal load (Newtons) from chassis accelerations
/// and a calibration. `a_lat` and `a_long` are in car-local m/s^2.
pub fn estimate(a_lat: f32, a_long: f32, calib: &CarCalibration) -> Option<Wheel<f32>> {
    let mass = calib.mass?;
    let cog = calib.cog_height?;
    let wheelbase = calib.wheelbase?;
    let bias = calib.front_weight_bias?;
    let track_f = calib.track_f?;
    let track_r = calib.track_r?;

    let g = GRAVITY_M_S2;
    let m_front = mass * bias;
    let m_rear = mass * (1.0 - bias);

    let delta_long = mass * a_long * cog / wheelbase;
    let delta_lat_f = m_front * a_lat * cog / track_f;
    let delta_lat_r = m_rear * a_lat * cog / track_r;

    let f_front = m_front * g - delta_long;
    let f_rear = m_rear * g + delta_long;

    Some(Wheel {
        fl: f_front * 0.5 - delta_lat_f * 0.5,
        fr: f_front * 0.5 + delta_lat_f * 0.5,
        rl: f_rear * 0.5 - delta_lat_r * 0.5,
        rr: f_rear * 0.5 + delta_lat_r * 0.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calib() -> CarCalibration {
        CarCalibration {
            mass: Some(1000.0),
            cog_height: Some(0.5),
            wheelbase: Some(2.5),
            track_f: Some(1.5),
            track_r: Some(1.5),
            front_weight_bias: Some(0.55),
            ..Default::default()
        }
    }

    fn sum(w: Wheel<f32>) -> f32 {
        w.fl + w.fr + w.rl + w.rr
    }

    #[test]
    fn missing_calibration_returns_none() {
        let mut c = calib();
        c.mass = None;
        assert!(estimate(0.0, 0.0, &c).is_none());
    }

    #[test]
    fn static_loads_sum_to_mg_and_match_bias() {
        let c = calib();
        let w = estimate(0.0, 0.0, &c).unwrap();
        let mg = 1000.0 * GRAVITY_M_S2;
        assert!((sum(w) - mg).abs() < 1e-2);
        // Front gets 55% of the static weight.
        let front = w.fl + w.fr;
        let rear = w.rl + w.rr;
        assert!((front - mg * 0.55).abs() < 1e-2);
        assert!((rear - mg * 0.45).abs() < 1e-2);
        assert!((w.fl - w.fr).abs() < 1e-3);
        assert!((w.rl - w.rr).abs() < 1e-3);
    }

    #[test]
    fn longitudinal_acceleration_shifts_load_rearward() {
        let c = calib();
        let w0 = estimate(0.0, 0.0, &c).unwrap();
        let w = estimate(0.0, 5.0, &c).unwrap(); // accel forward
        let mg = 1000.0 * GRAVITY_M_S2;
        assert!((sum(w) - mg).abs() < 1e-2);
        assert!(w.fl < w0.fl);
        assert!(w.rl > w0.rl);
        // Symmetric left/right at zero a_lat.
        assert!((w.fl - w.fr).abs() < 1e-3);
        assert!((w.rl - w.rr).abs() < 1e-3);
        // Magnitude check: delta = m * a * h / wb = 1000 * 5 * 0.5 / 2.5 = 1000 N
        // distributed across the front axle, so each front corner drops 500.
        assert!((w0.fl - w.fl - 500.0).abs() < 1e-2);
    }

    #[test]
    fn lateral_acceleration_shifts_load_to_positive_x_side() {
        let c = calib();
        let w0 = estimate(0.0, 0.0, &c).unwrap();
        let w = estimate(5.0, 0.0, &c).unwrap(); // accel +X (right)
        let mg = 1000.0 * GRAVITY_M_S2;
        assert!((sum(w) - mg).abs() < 1e-2);
        assert!(w.fr > w0.fr);
        assert!(w.rr > w0.rr);
        assert!(w.fl < w0.fl);
        assert!(w.rl < w0.rl);
    }
}
