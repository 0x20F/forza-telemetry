//! Per-corner normal-load reconstruction.
//!
//! Forza does not publish per-tire normal force, so we reconstruct it
//! from a textbook quasi-static two-track model: static weight
//! distribution plus longitudinal and lateral load transfer.
//!
//! Static axle masses from front weight bias `phi`:
//!
//! \[
//! m_{\mathrm{front}} = \phi\, m, \qquad
//! m_{\mathrm{rear}}  = (1 - \phi)\, m
//! \]
//!
//! Longitudinal (front&harr;rear) and per-axle lateral (left&harr;right)
//! transfers:
//!
//! \[
//! \Delta_{\mathrm{long}} = \frac{m\, a_{\mathrm{long}}\, h}{L},
//! \qquad
//! \Delta_{\mathrm{lat,F}} = \frac{m_{\mathrm{front}}\, a_{\mathrm{lat}}\, h}{t_F},
//! \qquad
//! \Delta_{\mathrm{lat,R}} = \frac{m_{\mathrm{rear}}\,  a_{\mathrm{lat}}\, h}{t_R}
//! \]
//!
//! Per-axle vertical loads, then per-corner:
//!
//! \[
//! F_{\mathrm{front}} = m_{\mathrm{front}}\, g - \Delta_{\mathrm{long}},
//! \qquad
//! F_{\mathrm{rear}}  = m_{\mathrm{rear}}\,  g + \Delta_{\mathrm{long}}
//! \]
//!
//! \[
//! \begin{aligned}
//! F_{\mathrm{FL}} &= \tfrac{1}{2} F_{\mathrm{front}} + \tfrac{1}{2} \Delta_{\mathrm{lat,F}} &
//! F_{\mathrm{FR}} &= \tfrac{1}{2} F_{\mathrm{front}} - \tfrac{1}{2} \Delta_{\mathrm{lat,F}} \\
//! F_{\mathrm{RL}} &= \tfrac{1}{2} F_{\mathrm{rear}}  + \tfrac{1}{2} \Delta_{\mathrm{lat,R}} &
//! F_{\mathrm{RR}} &= \tfrac{1}{2} F_{\mathrm{rear}}  - \tfrac{1}{2} \Delta_{\mathrm{lat,R}}
//! \end{aligned}
//! \]
//!
//! By construction `F_FL + F_FR + F_RL + F_RR = m * g`.
//!
//! The model assumes Forza's car-local frame (X right, Y up, Z forward).
//! A positive `a_lat` is body-frame centripetal acceleration to the right
//! (i.e. a right-hand turn); the body's inertia tilts it left, so load
//! shifts to the LEFT-side wheels (the outside of the turn). A positive
//! `a_long` (forward acceleration) shifts load rearward.
//!
//! Returns `None` when any of `mass`, `cog_height`, `wheelbase`,
//! `front_weight_bias`, `track_f`, `track_r` is missing from the
//! calibration; partial reconstruction would silently bias the outputs
//! and confuse downstream consumers, so empty CSV cells are preferred.
//!
//! See `docs/math.md#per-corner-normal-load` for the full derivation,
//! variable table, and sign-convention discussion.

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
        fl: f_front * 0.5 + delta_lat_f * 0.5,
        fr: f_front * 0.5 - delta_lat_f * 0.5,
        rl: f_rear * 0.5 + delta_lat_r * 0.5,
        rr: f_rear * 0.5 - delta_lat_r * 0.5,
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
    fn positive_lateral_acceleration_shifts_load_left() {
        // Body-frame `a_lat = +X` is centripetal to the right (a right-hand
        // turn). The chassis tilts left, so left-side wheels gain load.
        let c = calib();
        let w0 = estimate(0.0, 0.0, &c).unwrap();
        let w = estimate(5.0, 0.0, &c).unwrap();
        let mg = 1000.0 * GRAVITY_M_S2;
        assert!((sum(w) - mg).abs() < 1e-2);
        assert!(w.fl > w0.fl);
        assert!(w.rl > w0.rl);
        assert!(w.fr < w0.fr);
        assert!(w.rr < w0.rr);
    }
}
