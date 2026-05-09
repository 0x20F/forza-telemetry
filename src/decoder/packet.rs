//! `RawPacket` - the canonical typed snapshot of a single Forza Data Out
//! frame, plus the per-corner [`Wheel`] container used throughout the crate.

use super::Source;

/// Per-corner value, ordered front-left, front-right, rear-left, rear-right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel<T: Copy> {
    pub fl: T,
    pub fr: T,
    pub rl: T,
    pub rr: T,
}

impl<T: Copy> Wheel<T> {
    pub fn new(fl: T, fr: T, rl: T, rr: T) -> Self {
        Self { fl, fr, rl, rr }
    }

    pub fn map<U: Copy>(self, mut f: impl FnMut(T) -> U) -> Wheel<U> {
        Wheel {
            fl: f(self.fl),
            fr: f(self.fr),
            rl: f(self.rl),
            rr: f(self.rr),
        }
    }

    pub fn into_array(self) -> [T; 4] {
        [self.fl, self.fr, self.rl, self.rr]
    }

    pub fn from_array(arr: [T; 4]) -> Self {
        Self {
            fl: arr[0],
            fr: arr[1],
            rl: arr[2],
            rr: arr[3],
        }
    }
}

/// A decoded Forza Data Out frame.
///
/// Optional fields are present only on packet sources that carry them; on a
/// 232-byte sled packet, every dash-only and FM2023-only field is `None`.
#[derive(Debug, Clone)]
pub struct RawPacket {
    pub source: Source,
    /// Monotonic receive timestamp injected by the listener (nanoseconds).
    pub recv_time_ns: u64,

    pub is_race_on: bool,
    pub timestamp_ms: u32,
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,

    /// Car-local acceleration: X right, Y up, Z forward (m/s^2).
    pub acceleration_local: [f32; 3],
    /// Car-local velocity (m/s).
    pub velocity_local: [f32; 3],
    /// Car-local angular velocity: X pitch, Y yaw, Z roll (rad/s).
    pub angular_velocity: [f32; 3],

    /// World-space yaw, pitch, roll (radians).
    pub yaw_world: f32,
    pub pitch_world: f32,
    pub roll_world: f32,

    /// 0..1, 0=stretch, 1=compression.
    pub normalized_suspension_travel: Wheel<f32>,
    /// Absolute meters; needs baseline learning to map to signed-relative.
    pub suspension_travel_meters: Wheel<f32>,

    pub tire_slip_ratio: Wheel<f32>,
    pub tire_slip_angle: Wheel<f32>,
    pub tire_combined_slip: Wheel<f32>,
    pub wheel_rotation_speed: Wheel<f32>,
    pub wheel_on_rumble: Wheel<bool>,
    pub wheel_in_puddle_depth: Wheel<f32>,
    pub surface_rumble: Wheel<f32>,

    /// Tire surface temperature, converted to Celsius at decode time.
    pub tire_temp_c: Wheel<f32>,

    pub car_ordinal: i32,
    pub car_class: i32,
    pub performance_index: i32,
    pub drivetrain_type: i32,
    pub num_cylinders: i32,

    // -- Dash-only fields (None on Sled) --
    pub position: Option<[f32; 3]>,
    pub speed: Option<f32>,
    pub power_w: Option<f32>,
    pub torque_nm: Option<f32>,
    pub boost_psi: Option<f32>,
    pub fuel: Option<f32>,
    pub distance_traveled_m: Option<f32>,
    pub best_lap_s: Option<f32>,
    pub last_lap_s: Option<f32>,
    pub current_lap_s: Option<f32>,
    pub current_race_time_s: Option<f32>,
    pub lap_number: Option<u16>,
    pub race_position: Option<u8>,
    pub accel: Option<u8>,
    pub brake: Option<u8>,
    pub clutch: Option<u8>,
    pub handbrake: Option<u8>,
    pub gear: Option<u8>,
    /// Steering input as `i8` in `[-127, +127]`. Positive = right.
    pub steer: Option<i8>,
    pub normalized_driving_line: Option<i8>,
    pub normalized_ai_brake_difference: Option<i8>,

    // -- FM2023-only fields --
    pub tire_wear: Option<Wheel<f32>>,
    pub track_id: Option<i32>,
}
