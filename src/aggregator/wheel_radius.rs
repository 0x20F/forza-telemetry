//! Auto-calibrate effective wheel radius by observing free-rolling
//! wheels during coast.
//!
//! On coast (throttle and brake both off, vehicle speed above a small
//! gate), an undriven wheel obeys `v = omega * r` to within drag
//! effects, so per frame
//!
//! \[
//! r_n = \frac{|v_n|}{\bar{\omega}_n},
//! \qquad
//! \bar{\omega}_n = \tfrac{1}{2}\bigl(\omega_{n,L} + \omega_{n,R}\bigr)
//! \]
//!
//! is a clean estimate of the effective rolling radius. We collect per
//! axle samples in a fixed-size ring and emit the median once we have
//! enough:
//!
//! \[
//! \hat{r} = \mathrm{median}\bigl(\{\, r_n \mid n \in \text{coast}\,\}\bigr)
//! \]
//!
//! Forza's `DrivetrainType` tells us which wheels are undriven and so
//! which `r_hat` we can learn cleanly:
//!
//! - 0: FWD &rarr; rear wheels are undriven; learn `radius_r`.
//! - 1: RWD &rarr; front wheels are undriven; learn `radius_f`.
//! - 2: AWD &rarr; in *pure coast* no axle is loaded, so we learn both
//!   axles. Anything outside `0..=2` is treated as AWD (most permissive
//!   guess) so we still learn something on unknown drivetrains.
//!
//! See `docs/math.md#wheel-radius-auto-calibration` for the full
//! derivation and the gate thresholds.

use std::collections::VecDeque;

use crate::decoder::Wheel;

pub const COAST_SAMPLES_REQUIRED: usize = 30;
pub const MIN_COAST_SPEED_M_S: f32 = 5.0;
pub const PEDAL_OFF_THRESHOLD_NORM: f32 = 0.05;
pub const MIN_ABS_OMEGA_RAD_S: f32 = 1.0;

#[derive(Debug)]
pub struct RadiusLearner {
    samples_f: VecDeque<f32>,
    samples_r: VecDeque<f32>,
    radius_f: Option<f32>,
    radius_r: Option<f32>,
}

impl Default for RadiusLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl RadiusLearner {
    pub fn new() -> Self {
        Self {
            samples_f: VecDeque::with_capacity(COAST_SAMPLES_REQUIRED),
            samples_r: VecDeque::with_capacity(COAST_SAMPLES_REQUIRED),
            radius_f: None,
            radius_r: None,
        }
    }

    /// Feed one frame. `accel_norm`/`brake_norm` are the cheap-derivation
    /// pedal values in `[0, 1]`. `drivetrain_type` follows Forza's
    /// convention (0 FWD, 1 RWD, 2 AWD); anything else is treated as AWD
    /// (learn both axles) since that's the most permissive guess.
    pub fn ingest(
        &mut self,
        speed_m_s: f32,
        wheel_omega: Wheel<f32>,
        drivetrain_type: i32,
        accel_norm: f32,
        brake_norm: f32,
    ) {
        if speed_m_s.abs() < MIN_COAST_SPEED_M_S {
            return;
        }
        if accel_norm > PEDAL_OFF_THRESHOLD_NORM || brake_norm > PEDAL_OFF_THRESHOLD_NORM {
            return;
        }

        // Anything outside Forza's documented 0/1/2 codes is treated as
        // "unknown drivetrain"; we err on the side of learning both axles.
        let unknown = !(0..=2).contains(&drivetrain_type);
        let learn_front = unknown || drivetrain_type == 1 || drivetrain_type == 2;
        let learn_rear = unknown || drivetrain_type == 0 || drivetrain_type == 2;

        if learn_front {
            let mean = 0.5 * (wheel_omega.fl + wheel_omega.fr);
            if mean.abs() >= MIN_ABS_OMEGA_RAD_S {
                let r = speed_m_s.abs() / mean.abs();
                push_capped(&mut self.samples_f, r, COAST_SAMPLES_REQUIRED);
                if self.samples_f.len() >= COAST_SAMPLES_REQUIRED {
                    self.radius_f = Some(median(&self.samples_f));
                }
            }
        }
        if learn_rear {
            let mean = 0.5 * (wheel_omega.rl + wheel_omega.rr);
            if mean.abs() >= MIN_ABS_OMEGA_RAD_S {
                let r = speed_m_s.abs() / mean.abs();
                push_capped(&mut self.samples_r, r, COAST_SAMPLES_REQUIRED);
                if self.samples_r.len() >= COAST_SAMPLES_REQUIRED {
                    self.radius_r = Some(median(&self.samples_r));
                }
            }
        }
    }

    pub fn radius_f(&self) -> Option<f32> {
        self.radius_f
    }

    pub fn radius_r(&self) -> Option<f32> {
        self.radius_r
    }
}

fn push_capped(dq: &mut VecDeque<f32>, sample: f32, cap: usize) {
    dq.push_back(sample);
    if dq.len() > cap {
        dq.pop_front();
    }
}

fn median(samples: &VecDeque<f32>) -> f32 {
    let mut v: Vec<f32> = samples.iter().copied().collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn omega_for(speed: f32, radius: f32) -> Wheel<f32> {
        let w = speed / radius;
        Wheel::new(w, w, w, w)
    }

    #[test]
    fn rwd_learns_front_radius_in_coast() {
        let mut l = RadiusLearner::new();
        let r_target = 0.34;
        for _ in 0..COAST_SAMPLES_REQUIRED {
            l.ingest(30.0, omega_for(30.0, r_target), 1, 0.0, 0.0);
        }
        assert!((l.radius_f().unwrap() - r_target).abs() < 1e-4);
    }

    #[test]
    fn fwd_learns_rear_radius_in_coast() {
        let mut l = RadiusLearner::new();
        let r_target = 0.32;
        for _ in 0..COAST_SAMPLES_REQUIRED {
            l.ingest(30.0, omega_for(30.0, r_target), 0, 0.0, 0.0);
        }
        assert!((l.radius_r().unwrap() - r_target).abs() < 1e-4);
        assert!(l.radius_f().is_none());
    }

    #[test]
    fn awd_learns_both() {
        let mut l = RadiusLearner::new();
        for _ in 0..COAST_SAMPLES_REQUIRED {
            l.ingest(30.0, omega_for(30.0, 0.34), 2, 0.0, 0.0);
        }
        assert!(l.radius_f().is_some());
        assert!(l.radius_r().is_some());
    }

    #[test]
    fn pedal_input_blocks_learning() {
        let mut l = RadiusLearner::new();
        for _ in 0..COAST_SAMPLES_REQUIRED {
            l.ingest(30.0, omega_for(30.0, 0.34), 1, 0.5, 0.0);
        }
        assert!(l.radius_f().is_none());
    }

    #[test]
    fn low_speed_blocks_learning() {
        let mut l = RadiusLearner::new();
        for _ in 0..COAST_SAMPLES_REQUIRED {
            l.ingest(2.0, omega_for(2.0, 0.34), 1, 0.0, 0.0);
        }
        assert!(l.radius_f().is_none());
    }
}
