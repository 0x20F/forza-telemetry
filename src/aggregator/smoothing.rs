//! First-order exponential moving average (lowpass).
//!
//! The standard discrete-time first-order filter
//! `y[n] = alpha * x[n] + (1 - alpha) * y[n-1]`
//! with `alpha = dt / (tau + dt)`. `tau` is the time constant, in seconds.
//! When the previous value is unknown we initialise to the current sample
//! so the first frame is sane (no zero-spike).

#[derive(Debug, Clone, Copy)]
pub struct Ema {
    pub tau: f32,
    pub value: Option<f32>,
}

impl Ema {
    /// `tau` is the lowpass time constant in seconds.
    pub fn new(tau: f32) -> Self {
        Self { tau, value: None }
    }

    /// Apply one sample with the elapsed time since the previous sample.
    /// Returns the post-update value.
    pub fn update(&mut self, dt_s: f32, sample: f32) -> f32 {
        let updated = match self.value {
            None => sample,
            Some(prev) => {
                let alpha = lowpass_alpha(dt_s, self.tau);
                alpha * sample + (1.0 - alpha) * prev
            }
        };
        self.value = Some(updated);
        updated
    }

    pub fn current(&self) -> Option<f32> {
        self.value
    }
}

/// Standard alpha factor for a first-order lowpass with time constant `tau`.
/// Returns 0 for `dt <= 0` so re-emitting a previous sample is a no-op.
pub fn lowpass_alpha(dt_s: f32, tau: f32) -> f32 {
    if dt_s <= 0.0 || tau <= 0.0 {
        return 0.0;
    }
    dt_s / (tau + dt_s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_initialises_value() {
        let mut ema = Ema::new(0.2);
        assert_eq!(ema.update(0.016, 5.0), 5.0);
        assert_eq!(ema.current(), Some(5.0));
    }

    #[test]
    fn dt_zero_is_a_noop_after_init() {
        let mut ema = Ema::new(0.2);
        ema.update(0.016, 5.0);
        let v = ema.update(0.0, 100.0);
        assert!((v - 5.0).abs() < 1e-6);
    }

    #[test]
    fn converges_to_constant_input() {
        let mut ema = Ema::new(0.1);
        ema.update(0.016, 0.0);
        // ~30 frames at 60 Hz = 0.5s, well past 5*tau.
        for _ in 0..120 {
            ema.update(0.016, 9.81);
        }
        let v = ema.current().unwrap();
        assert!((v - 9.81).abs() < 0.01, "v = {v}");
    }

    #[test]
    fn alpha_bounded_in_zero_one() {
        assert_eq!(lowpass_alpha(0.0, 0.2), 0.0);
        assert!(lowpass_alpha(0.016, 0.2) > 0.0);
        assert!(lowpass_alpha(0.016, 0.2) < 1.0);
        assert!(lowpass_alpha(1000.0, 0.2) < 1.0);
    }
}
