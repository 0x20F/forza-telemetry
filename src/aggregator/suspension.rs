//! Per-corner ride-height baseline learner.
//!
//! Forza's `SuspensionTravelMeters` is the absolute spring displacement,
//! measured from some unknown rig reference. Kingpin needs the *signed*
//! travel relative to the static ride height (positive = compression). To
//! get there we observe the car during cruise (low |a_lat|, low |a_long|,
//! low |yaw_rate|) and call the median of those samples the baseline. From
//! then on, `relative = current - baseline`.
//!
//! Sign convention notes:
//!
//! - Forza's `SuspensionTravelMeters` *increases with compression*, in the
//!   same direction as `NormalizedSuspensionTravel` (0..1, 0=stretch,
//!   1=compression).
//! - Subtracting the baseline therefore yields a signed value where
//!   positive means "currently more compressed than at rest", which is
//!   exactly what Kingpin's signed-relative travel expects.
//!
//! Window size and gates are chosen to be conservative: 30 cruise samples
//! at 60 Hz (~0.5 s of motion) is enough to produce a stable median
//! without delaying baseline emission for very long.

use std::collections::VecDeque;

use crate::decoder::Wheel;

/// Number of cruise samples per corner before a baseline is emitted.
pub const CRUISE_SAMPLES_REQUIRED: usize = 30;

/// Maximum absolute lateral acceleration tolerated as cruise (m/s^2).
pub const CRUISE_LAT_ACCEL_MAX: f32 = 1.0;

/// Maximum absolute longitudinal acceleration tolerated as cruise (m/s^2).
pub const CRUISE_LONG_ACCEL_MAX: f32 = 0.8;

/// Maximum absolute yaw rate tolerated as cruise (rad/s).
pub const CRUISE_YAW_RATE_MAX: f32 = 0.05;

#[derive(Debug)]
pub struct RideHeightLearner {
    samples: [VecDeque<f32>; 4],
    baseline: [Option<f32>; 4],
}

impl Default for RideHeightLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl RideHeightLearner {
    pub fn new() -> Self {
        Self {
            samples: [
                VecDeque::with_capacity(CRUISE_SAMPLES_REQUIRED),
                VecDeque::with_capacity(CRUISE_SAMPLES_REQUIRED),
                VecDeque::with_capacity(CRUISE_SAMPLES_REQUIRED),
                VecDeque::with_capacity(CRUISE_SAMPLES_REQUIRED),
            ],
            baseline: [None; 4],
        }
    }

    /// Feed one frame's per-corner suspension meters with the chassis
    /// gating signals. Samples that don't pass cruise gating are ignored;
    /// once we have `CRUISE_SAMPLES_REQUIRED` cruise samples per corner,
    /// the baseline for that corner is emitted (and refined in-place as
    /// more cruise samples arrive).
    pub fn ingest(
        &mut self,
        suspension_meters: Wheel<f32>,
        accel_lat: f32,
        accel_long: f32,
        yaw_rate: f32,
    ) {
        if !is_cruise(accel_lat, accel_long, yaw_rate) {
            return;
        }
        let arr = suspension_meters.into_array();
        for (i, sample) in arr.into_iter().enumerate() {
            let dq = &mut self.samples[i];
            dq.push_back(sample);
            if dq.len() > CRUISE_SAMPLES_REQUIRED {
                dq.pop_front();
            }
            if dq.len() >= CRUISE_SAMPLES_REQUIRED {
                self.baseline[i] = Some(median(dq));
            }
        }
    }

    pub fn baseline(&self) -> Wheel<Option<f32>> {
        Wheel::from_array(self.baseline)
    }

    /// Compute the per-corner signed relative travel using the current
    /// baseline. Returns `None` per corner that hasn't seen enough cruise
    /// samples yet.
    pub fn relative_travel(&self, current: Wheel<f32>) -> Wheel<Option<f32>> {
        let cur = current.into_array();
        let mut out = [None; 4];
        for i in 0..4 {
            if let Some(b) = self.baseline[i] {
                out[i] = Some(cur[i] - b);
            }
        }
        Wheel::from_array(out)
    }
}

pub fn is_cruise(accel_lat: f32, accel_long: f32, yaw_rate: f32) -> bool {
    accel_lat.abs() <= CRUISE_LAT_ACCEL_MAX
        && accel_long.abs() <= CRUISE_LONG_ACCEL_MAX
        && yaw_rate.abs() <= CRUISE_YAW_RATE_MAX
}

fn median(samples: &VecDeque<f32>) -> f32 {
    let mut v: Vec<f32> = samples.iter().copied().collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(v: f32) -> Wheel<f32> {
        Wheel::new(v, v, v, v)
    }

    #[test]
    fn baseline_emerges_after_k_cruise_frames() {
        let mut learner = RideHeightLearner::new();
        for _ in 0..(CRUISE_SAMPLES_REQUIRED - 1) {
            learner.ingest(wheel(0.05), 0.0, 0.0, 0.0);
        }
        assert!(learner.baseline().fl.is_none(), "should not emit early");
        learner.ingest(wheel(0.05), 0.0, 0.0, 0.0);
        let bl = learner.baseline();
        assert!((bl.fl.unwrap() - 0.05).abs() < 1e-5);
    }

    #[test]
    fn cruise_gating_excludes_active_frames() {
        let mut learner = RideHeightLearner::new();
        // High lateral g should be excluded.
        for _ in 0..50 {
            learner.ingest(wheel(0.05), 5.0, 0.0, 0.0);
        }
        assert!(learner.baseline().fl.is_none());
        // Once the car settles, samples count.
        for _ in 0..CRUISE_SAMPLES_REQUIRED {
            learner.ingest(wheel(0.05), 0.0, 0.0, 0.0);
        }
        assert!(learner.baseline().fl.is_some());
    }

    #[test]
    fn relative_travel_is_signed() {
        let mut learner = RideHeightLearner::new();
        for _ in 0..CRUISE_SAMPLES_REQUIRED {
            learner.ingest(wheel(0.05), 0.0, 0.0, 0.0);
        }
        let rel = learner.relative_travel(wheel(0.07));
        assert!((rel.fl.unwrap() - 0.02).abs() < 1e-5);
        let rel = learner.relative_travel(wheel(0.03));
        assert!((rel.fl.unwrap() - (-0.02)).abs() < 1e-5);
    }
}
