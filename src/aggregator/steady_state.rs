//! Coarse chassis state classifier.
//!
//! Emits a u32 bitmask of "what is the car doing right now" plus
//! `time_in_state_ms`, the wall-clock duration the bitmask has been
//! stable. Useful for downstream filters that only want to look at,
//! say, settled straight-line frames.
//!
//! Flag predicates (with `~a` and `~b` the normalized accel and brake
//! pedals in `[0, 1]`):
//!
//! | Bit | Flag           | Predicate |
//! | --- | -------------- | --- |
//! | 0   | `COASTING`     | `~a < 0.05` and `~b < 0.05` |
//! | 1   | `ACCELERATING` | `~a > 0.05` |
//! | 2   | `BRAKING`      | `~b > 0.05` |
//! | 3   | `CORNERING`    | `|a_lat| > 1.0` or `|omega_y| > 0.05` |
//! | 4   | `STEADY`       | `|a_lat| < 0.5` and `|a_long| < 0.5` and `|omega_y| < 0.03` |
//!
//! Time in state is computed by tracking the receive time when the
//! bitmask last changed:
//!
//! \[
//! T_n = \left\lfloor (t_n - t_{\mathrm{state\_started}}) / 10^{6} \right\rfloor
//! \quad\text{(milliseconds)}
//! \]
//!
//! where `t_state_started` is reset to `t_n` whenever the current
//! bitmask differs from the previous one.
//!
//! Flags are independent and may co-occur (e.g. `BRAKING | CORNERING`
//! on a trailing-brake corner entry). `STEADY` is set only when none of
//! the transient flags fire.
//!
//! See `docs/math.md#steady-state-classifier` for the full predicate
//! table and rationale.

use crate::decoder::RawPacket;

pub const FLAG_COASTING: u32 = 1 << 0;
pub const FLAG_ACCELERATING: u32 = 1 << 1;
pub const FLAG_BRAKING: u32 = 1 << 2;
pub const FLAG_CORNERING: u32 = 1 << 3;
pub const FLAG_STEADY: u32 = 1 << 4;

pub const PEDAL_ON_THRESHOLD_NORM: f32 = 0.05;
pub const ACCEL_TRANSIENT_THRESHOLD: f32 = 1.0; // m/s^2
pub const YAW_RATE_TRANSIENT_THRESHOLD: f32 = 0.05; // rad/s
pub const ACCEL_STEADY_THRESHOLD: f32 = 0.5;
pub const YAW_RATE_STEADY_THRESHOLD: f32 = 0.03;

#[derive(Debug, Clone, Copy)]
pub struct SteadyState {
    pub flags: u32,
    pub time_in_state_ms: u32,
}

#[derive(Debug, Default)]
pub struct Classifier {
    last_flags: Option<u32>,
    state_started_ns: Option<u64>,
}

impl Classifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute flags for the given packet. `recv_time_ns` is used to
    /// measure how long the current bitmask has held.
    pub fn classify(&mut self, recv_time_ns: u64, packet: &RawPacket) -> SteadyState {
        let mut flags = 0u32;

        let accel_norm = packet.accel.map(|v| v as f32 / 255.0).unwrap_or(0.0);
        let brake_norm = packet.brake.map(|v| v as f32 / 255.0).unwrap_or(0.0);

        let off_pedal =
            accel_norm < PEDAL_ON_THRESHOLD_NORM && brake_norm < PEDAL_ON_THRESHOLD_NORM;
        if off_pedal {
            flags |= FLAG_COASTING;
        }
        if accel_norm > PEDAL_ON_THRESHOLD_NORM {
            flags |= FLAG_ACCELERATING;
        }
        if brake_norm > PEDAL_ON_THRESHOLD_NORM {
            flags |= FLAG_BRAKING;
        }

        let a_lat = packet.acceleration_local[0].abs();
        let a_long = packet.acceleration_local[2].abs();
        let yaw_rate = packet.angular_velocity[1].abs();

        if a_lat > ACCEL_TRANSIENT_THRESHOLD || yaw_rate > YAW_RATE_TRANSIENT_THRESHOLD {
            flags |= FLAG_CORNERING;
        }

        if a_lat < ACCEL_STEADY_THRESHOLD
            && a_long < ACCEL_STEADY_THRESHOLD
            && yaw_rate < YAW_RATE_STEADY_THRESHOLD
        {
            flags |= FLAG_STEADY;
        }

        if self.last_flags != Some(flags) {
            self.state_started_ns = Some(recv_time_ns);
            self.last_flags = Some(flags);
        }
        let started = self.state_started_ns.unwrap_or(recv_time_ns);
        let dt_ns = recv_time_ns.saturating_sub(started);
        let time_in_state_ms = (dt_ns / 1_000_000) as u32;

        SteadyState {
            flags,
            time_in_state_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::decode;

    fn pkt_with(accel: u8, brake: u8, accel_local: [f32; 3], yaw_rate: f32, recv_ns: u64) -> RawPacket {
        // Build a synthetic packet via the test PacketSpec. We pull it in
        // through the public API to avoid duplicating the byte layout.
        // The test crate-private builder lives in tests/common, so for unit
        // tests inside the lib we hand-roll a minimal byte buffer using
        // public helpers.
        let mut buf = vec![0u8; 311];
        // Sled bytes we care about:
        buf[0..4].copy_from_slice(&1i32.to_le_bytes()); // is_race_on
        buf[20..24].copy_from_slice(&accel_local[0].to_le_bytes());
        buf[24..28].copy_from_slice(&accel_local[1].to_le_bytes());
        buf[28..32].copy_from_slice(&accel_local[2].to_le_bytes());
        buf[44..48].copy_from_slice(&0.0f32.to_le_bytes()); // ang_x
        buf[48..52].copy_from_slice(&yaw_rate.to_le_bytes()); // ang_y (yaw)
        buf[52..56].copy_from_slice(&0.0f32.to_le_bytes()); // ang_z
        // Dash byte slots for accel/brake.
        buf[232 + 71] = accel;
        buf[232 + 72] = brake;
        decode(&buf, recv_ns).unwrap_or_else(|e| panic!("decode {e:?}"))
    }

    #[test]
    fn coasting_when_off_pedals() {
        let mut c = Classifier::new();
        let p = pkt_with(0, 0, [0.0; 3], 0.0, 0);
        let st = c.classify(0, &p);
        assert!(st.flags & FLAG_COASTING != 0);
        assert!(st.flags & FLAG_STEADY != 0);
    }

    #[test]
    fn accelerating_and_cornering_can_co_occur() {
        let mut c = Classifier::new();
        let p = pkt_with(200, 0, [3.0, 0.0, 4.0], 0.4, 0);
        let st = c.classify(0, &p);
        assert!(st.flags & FLAG_ACCELERATING != 0);
        assert!(st.flags & FLAG_CORNERING != 0);
        assert_eq!(st.flags & FLAG_STEADY, 0);
    }

    #[test]
    fn time_in_state_grows_until_flags_change() {
        let mut c = Classifier::new();
        let p_steady = pkt_with(0, 0, [0.0; 3], 0.0, 0);

        let st0 = c.classify(0, &p_steady);
        let st1 = c.classify(100_000_000, &p_steady); // +100 ms
        let st2 = c.classify(500_000_000, &p_steady); // +500 ms total
        assert_eq!(st0.flags, st1.flags);
        assert_eq!(st0.flags, st2.flags);
        assert_eq!(st0.time_in_state_ms, 0);
        assert!((st1.time_in_state_ms as i32 - 100).abs() <= 1);
        assert!((st2.time_in_state_ms as i32 - 500).abs() <= 1);

        // Now hit the brakes; state changes -> timer resets.
        let p_braking = pkt_with(0, 200, [0.0, 0.0, -3.0], 0.0, 0);
        let st3 = c.classify(600_000_000, &p_braking);
        assert_ne!(st3.flags, st0.flags);
        assert_eq!(st3.time_in_state_ms, 0);
    }
}
