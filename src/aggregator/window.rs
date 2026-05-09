//! Fixed-duration rolling buffer keyed by Forza's per-packet timestamp.
//!
//! Used by later phases (ride-height learner, steady-state classifier,
//! wheel-radius learner) that need to look back a window of frames.
//! Keeping the same data structure across learners means we only buffer
//! once.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RollingBuffer<T> {
    /// `(timestamp_ms, value)`. Timestamps are monotonic non-decreasing in
    /// normal operation; old entries are evicted when the window fills.
    samples: VecDeque<(u64, T)>,
    /// Window size in milliseconds.
    window_ms: u64,
}

impl<T> RollingBuffer<T> {
    pub fn new(window_ms: u64) -> Self {
        Self {
            samples: VecDeque::new(),
            window_ms,
        }
    }

    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    /// Push a sample tagged with `t_ms` and drop anything older than the
    /// window end.
    pub fn push(&mut self, t_ms: u64, value: T) {
        self.samples.push_back((t_ms, value));
        let cutoff = t_ms.saturating_sub(self.window_ms);
        while let Some(&(t, _)) = self.samples.front() {
            if t < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(u64, T)> {
        self.samples.iter()
    }

    /// Drop everything (e.g. on session reset).
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_and_evicts_old_samples() {
        let mut buf = RollingBuffer::<f32>::new(100);
        buf.push(0, 1.0);
        buf.push(50, 2.0);
        buf.push(150, 3.0);
        // 0 is older than 150 - 100 = 50; should be evicted (strict `<`).
        // 50 is exactly at cutoff and stays.
        assert_eq!(buf.len(), 2);
        let kept: Vec<_> = buf.iter().map(|(_, v)| *v).collect();
        assert_eq!(kept, vec![2.0, 3.0]);
    }

    #[test]
    fn reset_drops_all() {
        let mut buf = RollingBuffer::<i32>::new(1000);
        buf.push(0, 42);
        buf.clear();
        assert!(buf.is_empty());
    }
}
