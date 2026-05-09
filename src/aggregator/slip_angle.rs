//! Body slip angle: the angle between the car's heading and its velocity
//! vector, in the horizontal plane.
//!
//! Forza's local frame is X-right, Y-up, Z-forward, so the heading axis is
//! `+Z` and the slip is `atan2(vx, vz)`. Positive values mean the velocity
//! points to the driver's right relative to the heading (rear-of-car is
//! sliding left, classic oversteer attitude). At very low speed the angle
//! is meaningless, so we return `None` until the car is moving at least
//! 1 m/s in the horizontal plane.

const SPEED_GATE_M_S: f32 = 1.0;

/// Returns the body slip angle in radians, or `None` if the horizontal
/// speed is below the gate.
pub fn body_slip_angle(velocity_local: [f32; 3]) -> Option<f32> {
    let vx = velocity_local[0];
    let vz = velocity_local[2];
    let speed_sq = vx * vx + vz * vz;
    if speed_sq < SPEED_GATE_M_S * SPEED_GATE_M_S {
        return None;
    }
    Some(vx.atan2(vz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_gate_is_none() {
        assert_eq!(body_slip_angle([0.0, 0.0, 0.0]), None);
        assert_eq!(body_slip_angle([0.5, 0.0, 0.5]), None);
    }

    #[test]
    fn straight_forward_is_zero() {
        let v = body_slip_angle([0.0, 0.0, 30.0]).unwrap();
        assert!(v.abs() < 1e-6);
    }

    #[test]
    fn equal_lat_long_is_pi_over_four() {
        let v = body_slip_angle([10.0, 0.0, 10.0]).unwrap();
        assert!((v - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
    }

    #[test]
    fn pure_lateral_is_pi_over_two() {
        let v = body_slip_angle([10.0, 0.0, 0.0]).unwrap();
        assert!((v - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn negative_lateral_is_negative() {
        let v = body_slip_angle([-10.0, 0.0, 10.0]).unwrap();
        assert!(v < 0.0);
    }
}
