//! Shared test helpers for building synthetic Forza Data Out byte buffers.
//!
//! The decoder is the source of truth for offsets; this builder writes bytes
//! at the same offsets so we can round-trip a known set of values through
//! `decode()` and assert what comes back.

#![allow(dead_code)]

use forza_telemetry::Source;

/// A logical frame whose values we can paint into bytes for any source.
#[derive(Debug, Clone)]
pub struct PacketSpec {
    pub is_race_on: bool,
    pub timestamp_ms: u32,
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,
    pub acceleration_local: [f32; 3],
    pub velocity_local: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub yaw_world: f32,
    pub pitch_world: f32,
    pub roll_world: f32,
    pub norm_susp_travel: [f32; 4],
    pub tire_slip_ratio: [f32; 4],
    pub wheel_rotation_speed: [f32; 4],
    pub wheel_on_rumble: [bool; 4],
    pub wheel_in_puddle_depth: [f32; 4],
    pub surface_rumble: [f32; 4],
    pub tire_slip_angle: [f32; 4],
    pub tire_combined_slip: [f32; 4],
    pub susp_travel_meters: [f32; 4],
    pub car_ordinal: i32,
    pub car_class: i32,
    pub performance_index: i32,
    pub drivetrain_type: i32,
    pub num_cylinders: i32,
    // Dash-only
    pub position: [f32; 3],
    pub speed: f32,
    pub power_w: f32,
    pub torque_nm: f32,
    pub tire_temp_f: [f32; 4],
    pub boost_psi: f32,
    pub fuel: f32,
    pub distance_traveled_m: f32,
    pub best_lap_s: f32,
    pub last_lap_s: f32,
    pub current_lap_s: f32,
    pub current_race_time_s: f32,
    pub lap_number: u16,
    pub race_position: u8,
    pub accel: u8,
    pub brake: u8,
    pub clutch: u8,
    pub handbrake: u8,
    pub gear: u8,
    pub steer: i8,
    pub normalized_driving_line: i8,
    pub normalized_ai_brake_difference: i8,
    // FM 2023 extras
    pub tire_wear: [f32; 4],
    pub track_id: i32,
}

impl PacketSpec {
    /// Distinct, easy-to-eyeball values for every field. Wheel lanes get
    /// distinct values so we can verify FL/FR/RL/RR ordering.
    pub fn distinct() -> Self {
        Self {
            is_race_on: true,
            timestamp_ms: 1_234_567,
            engine_max_rpm: 9000.0,
            engine_idle_rpm: 800.0,
            current_engine_rpm: 4500.0,
            acceleration_local: [0.5, -1.0, 9.81],
            velocity_local: [0.1, 0.0, 25.0],
            angular_velocity: [0.01, 0.2, -0.05],
            yaw_world: 0.7,
            pitch_world: -0.05,
            roll_world: 0.02,
            norm_susp_travel: [0.10, 0.11, 0.12, 0.13],
            tire_slip_ratio: [0.20, 0.21, 0.22, 0.23],
            wheel_rotation_speed: [70.0, 71.0, 72.0, 73.0],
            wheel_on_rumble: [false, true, false, true],
            wheel_in_puddle_depth: [0.0, 0.01, 0.02, 0.03],
            surface_rumble: [0.30, 0.31, 0.32, 0.33],
            tire_slip_angle: [0.40, 0.41, 0.42, 0.43],
            tire_combined_slip: [0.50, 0.51, 0.52, 0.53],
            susp_travel_meters: [0.060, 0.061, 0.062, 0.063],
            car_ordinal: 2735,
            car_class: 5,
            performance_index: 800,
            drivetrain_type: 2,
            num_cylinders: 8,
            position: [-100.0, 5.0, 250.0],
            speed: 60.0,
            power_w: 250_000.0,
            torque_nm: 700.0,
            tire_temp_f: [180.0, 190.0, 200.0, 210.0],
            boost_psi: 12.5,
            fuel: 0.75,
            distance_traveled_m: 4321.0,
            best_lap_s: 95.0,
            last_lap_s: 96.5,
            current_lap_s: 30.0,
            current_race_time_s: 320.0,
            lap_number: 3,
            race_position: 2,
            accel: 200,
            brake: 0,
            clutch: 0,
            handbrake: 0,
            gear: 4,
            steer: 50,
            normalized_driving_line: -10,
            normalized_ai_brake_difference: 5,
            tire_wear: [0.001, 0.002, 0.003, 0.004],
            track_id: 99,
        }
    }

    pub fn to_bytes(&self, source: Source) -> Vec<u8> {
        let len = match source {
            Source::Sled => 232,
            Source::DashFm => 311,
            Source::DashHorizon => 324,
            Source::Fm2023Extras => 331,
        };
        let mut buf = vec![0u8; len];

        // -- Sled (shared by every source) --
        write_i32(&mut buf, 0, if self.is_race_on { 1 } else { 0 });
        write_u32(&mut buf, 4, self.timestamp_ms);
        write_f32(&mut buf, 8, self.engine_max_rpm);
        write_f32(&mut buf, 12, self.engine_idle_rpm);
        write_f32(&mut buf, 16, self.current_engine_rpm);
        write_vec3(&mut buf, 20, self.acceleration_local);
        write_vec3(&mut buf, 32, self.velocity_local);
        write_vec3(&mut buf, 44, self.angular_velocity);
        write_f32(&mut buf, 56, self.yaw_world);
        write_f32(&mut buf, 60, self.pitch_world);
        write_f32(&mut buf, 64, self.roll_world);
        write_wheel_f32(&mut buf, 68, self.norm_susp_travel);
        write_wheel_f32(&mut buf, 84, self.tire_slip_ratio);
        write_wheel_f32(&mut buf, 100, self.wheel_rotation_speed);
        write_wheel_bool_i32(&mut buf, 116, self.wheel_on_rumble);
        write_wheel_f32(&mut buf, 132, self.wheel_in_puddle_depth);
        write_wheel_f32(&mut buf, 148, self.surface_rumble);
        write_wheel_f32(&mut buf, 164, self.tire_slip_angle);
        write_wheel_f32(&mut buf, 180, self.tire_combined_slip);
        write_wheel_f32(&mut buf, 196, self.susp_travel_meters);
        write_i32(&mut buf, 212, self.car_ordinal);
        write_i32(&mut buf, 216, self.car_class);
        write_i32(&mut buf, 220, self.performance_index);
        write_i32(&mut buf, 224, self.drivetrain_type);
        write_i32(&mut buf, 228, self.num_cylinders);

        let dash_base = match source {
            Source::Sled => None,
            Source::DashFm | Source::Fm2023Extras => Some(232),
            Source::DashHorizon => Some(244),
        };

        if let Some(base) = dash_base {
            write_vec3(&mut buf, base, self.position);
            write_f32(&mut buf, base + 12, self.speed);
            write_f32(&mut buf, base + 16, self.power_w);
            write_f32(&mut buf, base + 20, self.torque_nm);
            write_wheel_f32(&mut buf, base + 24, self.tire_temp_f);
            write_f32(&mut buf, base + 40, self.boost_psi);
            write_f32(&mut buf, base + 44, self.fuel);
            write_f32(&mut buf, base + 48, self.distance_traveled_m);
            write_f32(&mut buf, base + 52, self.best_lap_s);
            write_f32(&mut buf, base + 56, self.last_lap_s);
            write_f32(&mut buf, base + 60, self.current_lap_s);
            write_f32(&mut buf, base + 64, self.current_race_time_s);
            write_u16(&mut buf, base + 68, self.lap_number);
            buf[base + 70] = self.race_position;
            buf[base + 71] = self.accel;
            buf[base + 72] = self.brake;
            buf[base + 73] = self.clutch;
            buf[base + 74] = self.handbrake;
            buf[base + 75] = self.gear;
            buf[base + 76] = self.steer as u8;
            buf[base + 77] = self.normalized_driving_line as u8;
            buf[base + 78] = self.normalized_ai_brake_difference as u8;
        }

        if matches!(source, Source::Fm2023Extras) {
            write_wheel_f32(&mut buf, 311, self.tire_wear);
            write_i32(&mut buf, 327, self.track_id);
        }

        buf
    }
}

fn write_f32(buf: &mut [u8], offset: usize, v: f32) {
    buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn write_u32(buf: &mut [u8], offset: usize, v: u32) {
    buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn write_i32(buf: &mut [u8], offset: usize, v: i32) {
    buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

fn write_u16(buf: &mut [u8], offset: usize, v: u16) {
    buf[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
}

fn write_vec3(buf: &mut [u8], offset: usize, v: [f32; 3]) {
    write_f32(buf, offset, v[0]);
    write_f32(buf, offset + 4, v[1]);
    write_f32(buf, offset + 8, v[2]);
}

fn write_wheel_f32(buf: &mut [u8], offset: usize, v: [f32; 4]) {
    write_f32(buf, offset, v[0]);
    write_f32(buf, offset + 4, v[1]);
    write_f32(buf, offset + 8, v[2]);
    write_f32(buf, offset + 12, v[3]);
}

fn write_wheel_bool_i32(buf: &mut [u8], offset: usize, v: [bool; 4]) {
    for (i, b) in v.iter().enumerate() {
        write_i32(buf, offset + i * 4, if *b { 1 } else { 0 });
    }
}
