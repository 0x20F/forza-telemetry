//! `EnrichedFrame` - the flat row written to each CSV line.
//!
//! Phase 2 populates raw decoded fields and cheap normalizations. Temporal
//! columns (slip angle, smoothed accels, learned baselines, etc.) are
//! reserved here so the schema is stable; later aggregator phases fill them
//! in.

use serde::Serialize;

use crate::decoder::{RawPacket, Wheel};

/// Single CSV row. Field names correspond to column headers verbatim.
///
/// Optional columns serialize as empty cells when `None`, which is exactly
/// what consumers should see for "format does not carry this" or
/// "aggregator hasn't learned this yet".
#[derive(Debug, Clone, Serialize)]
pub struct EnrichedFrame {
    pub recv_time_ns: u64,
    pub source: &'static str,

    pub is_race_on: bool,
    pub timestamp_ms: u32,

    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,

    pub acceleration_local_x: f32,
    pub acceleration_local_y: f32,
    pub acceleration_local_z: f32,

    pub velocity_local_x: f32,
    pub velocity_local_y: f32,
    pub velocity_local_z: f32,

    pub angular_velocity_pitch: f32,
    pub angular_velocity_yaw: f32,
    pub angular_velocity_roll: f32,

    pub yaw_world: f32,
    pub pitch_world: f32,
    pub roll_world: f32,

    pub normalized_suspension_travel_fl: f32,
    pub normalized_suspension_travel_fr: f32,
    pub normalized_suspension_travel_rl: f32,
    pub normalized_suspension_travel_rr: f32,

    pub suspension_travel_meters_fl: f32,
    pub suspension_travel_meters_fr: f32,
    pub suspension_travel_meters_rl: f32,
    pub suspension_travel_meters_rr: f32,

    pub tire_slip_ratio_fl: f32,
    pub tire_slip_ratio_fr: f32,
    pub tire_slip_ratio_rl: f32,
    pub tire_slip_ratio_rr: f32,

    pub tire_slip_angle_fl: f32,
    pub tire_slip_angle_fr: f32,
    pub tire_slip_angle_rl: f32,
    pub tire_slip_angle_rr: f32,

    pub tire_combined_slip_fl: f32,
    pub tire_combined_slip_fr: f32,
    pub tire_combined_slip_rl: f32,
    pub tire_combined_slip_rr: f32,

    pub wheel_rotation_speed_fl: f32,
    pub wheel_rotation_speed_fr: f32,
    pub wheel_rotation_speed_rl: f32,
    pub wheel_rotation_speed_rr: f32,

    pub wheel_on_rumble_fl: bool,
    pub wheel_on_rumble_fr: bool,
    pub wheel_on_rumble_rl: bool,
    pub wheel_on_rumble_rr: bool,

    pub wheel_in_puddle_depth_fl: f32,
    pub wheel_in_puddle_depth_fr: f32,
    pub wheel_in_puddle_depth_rl: f32,
    pub wheel_in_puddle_depth_rr: f32,

    pub surface_rumble_fl: f32,
    pub surface_rumble_fr: f32,
    pub surface_rumble_rl: f32,
    pub surface_rumble_rr: f32,

    pub tire_temp_c_fl: f32,
    pub tire_temp_c_fr: f32,
    pub tire_temp_c_rl: f32,
    pub tire_temp_c_rr: f32,

    pub car_ordinal: i32,
    pub car_class: i32,
    pub performance_index: i32,
    pub drivetrain_type: i32,
    pub num_cylinders: i32,

    pub position_x: Option<f32>,
    pub position_y: Option<f32>,
    pub position_z: Option<f32>,

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
    pub steer: Option<i8>,
    pub normalized_driving_line: Option<i8>,
    pub normalized_ai_brake_difference: Option<i8>,

    pub tire_wear_fl: Option<f32>,
    pub tire_wear_fr: Option<f32>,
    pub tire_wear_rl: Option<f32>,
    pub tire_wear_rr: Option<f32>,
    pub track_id: Option<i32>,

    // -- Cheap per-frame normalizations --
    pub steer_normalized: Option<f32>,
    pub accel_normalized: Option<f32>,
    pub brake_normalized: Option<f32>,
    pub clutch_normalized: Option<f32>,
    pub handbrake_normalized: Option<f32>,

    // -- Temporal columns (reserved; populated in later phases) --
    pub body_slip_angle_rad: Option<f32>,
    pub acceleration_long_lp: Option<f32>,
    pub acceleration_lat_lp: Option<f32>,
    pub yaw_rate_lp: Option<f32>,

    pub suspension_travel_relative_fl: Option<f32>,
    pub suspension_travel_relative_fr: Option<f32>,
    pub suspension_travel_relative_rl: Option<f32>,
    pub suspension_travel_relative_rr: Option<f32>,

    pub ride_height_baseline_fl: Option<f32>,
    pub ride_height_baseline_fr: Option<f32>,
    pub ride_height_baseline_rl: Option<f32>,
    pub ride_height_baseline_rr: Option<f32>,

    pub normal_load_fl: Option<f32>,
    pub normal_load_fr: Option<f32>,
    pub normal_load_rl: Option<f32>,
    pub normal_load_rr: Option<f32>,

    pub wheel_radius_learned_f: Option<f32>,
    pub wheel_radius_learned_r: Option<f32>,

    pub steady_state_flags: Option<u32>,
    pub time_in_state_ms: Option<u32>,
}

impl EnrichedFrame {
    /// Build a Phase-2 frame: all raw + cheap derivations populated, every
    /// temporal column left empty (`None`).
    pub fn from_raw(packet: &RawPacket) -> Self {
        Self {
            recv_time_ns: packet.recv_time_ns,
            source: packet.source.as_str(),

            is_race_on: packet.is_race_on,
            timestamp_ms: packet.timestamp_ms,

            engine_max_rpm: packet.engine_max_rpm,
            engine_idle_rpm: packet.engine_idle_rpm,
            current_engine_rpm: packet.current_engine_rpm,

            acceleration_local_x: packet.acceleration_local[0],
            acceleration_local_y: packet.acceleration_local[1],
            acceleration_local_z: packet.acceleration_local[2],

            velocity_local_x: packet.velocity_local[0],
            velocity_local_y: packet.velocity_local[1],
            velocity_local_z: packet.velocity_local[2],

            angular_velocity_pitch: packet.angular_velocity[0],
            angular_velocity_yaw: packet.angular_velocity[1],
            angular_velocity_roll: packet.angular_velocity[2],

            yaw_world: packet.yaw_world,
            pitch_world: packet.pitch_world,
            roll_world: packet.roll_world,

            normalized_suspension_travel_fl: packet.normalized_suspension_travel.fl,
            normalized_suspension_travel_fr: packet.normalized_suspension_travel.fr,
            normalized_suspension_travel_rl: packet.normalized_suspension_travel.rl,
            normalized_suspension_travel_rr: packet.normalized_suspension_travel.rr,

            suspension_travel_meters_fl: packet.suspension_travel_meters.fl,
            suspension_travel_meters_fr: packet.suspension_travel_meters.fr,
            suspension_travel_meters_rl: packet.suspension_travel_meters.rl,
            suspension_travel_meters_rr: packet.suspension_travel_meters.rr,

            tire_slip_ratio_fl: packet.tire_slip_ratio.fl,
            tire_slip_ratio_fr: packet.tire_slip_ratio.fr,
            tire_slip_ratio_rl: packet.tire_slip_ratio.rl,
            tire_slip_ratio_rr: packet.tire_slip_ratio.rr,

            tire_slip_angle_fl: packet.tire_slip_angle.fl,
            tire_slip_angle_fr: packet.tire_slip_angle.fr,
            tire_slip_angle_rl: packet.tire_slip_angle.rl,
            tire_slip_angle_rr: packet.tire_slip_angle.rr,

            tire_combined_slip_fl: packet.tire_combined_slip.fl,
            tire_combined_slip_fr: packet.tire_combined_slip.fr,
            tire_combined_slip_rl: packet.tire_combined_slip.rl,
            tire_combined_slip_rr: packet.tire_combined_slip.rr,

            wheel_rotation_speed_fl: packet.wheel_rotation_speed.fl,
            wheel_rotation_speed_fr: packet.wheel_rotation_speed.fr,
            wheel_rotation_speed_rl: packet.wheel_rotation_speed.rl,
            wheel_rotation_speed_rr: packet.wheel_rotation_speed.rr,

            wheel_on_rumble_fl: packet.wheel_on_rumble.fl,
            wheel_on_rumble_fr: packet.wheel_on_rumble.fr,
            wheel_on_rumble_rl: packet.wheel_on_rumble.rl,
            wheel_on_rumble_rr: packet.wheel_on_rumble.rr,

            wheel_in_puddle_depth_fl: packet.wheel_in_puddle_depth.fl,
            wheel_in_puddle_depth_fr: packet.wheel_in_puddle_depth.fr,
            wheel_in_puddle_depth_rl: packet.wheel_in_puddle_depth.rl,
            wheel_in_puddle_depth_rr: packet.wheel_in_puddle_depth.rr,

            surface_rumble_fl: packet.surface_rumble.fl,
            surface_rumble_fr: packet.surface_rumble.fr,
            surface_rumble_rl: packet.surface_rumble.rl,
            surface_rumble_rr: packet.surface_rumble.rr,

            tire_temp_c_fl: packet.tire_temp_c.fl,
            tire_temp_c_fr: packet.tire_temp_c.fr,
            tire_temp_c_rl: packet.tire_temp_c.rl,
            tire_temp_c_rr: packet.tire_temp_c.rr,

            car_ordinal: packet.car_ordinal,
            car_class: packet.car_class,
            performance_index: packet.performance_index,
            drivetrain_type: packet.drivetrain_type,
            num_cylinders: packet.num_cylinders,

            position_x: packet.position.map(|p| p[0]),
            position_y: packet.position.map(|p| p[1]),
            position_z: packet.position.map(|p| p[2]),

            speed: packet.speed,
            power_w: packet.power_w,
            torque_nm: packet.torque_nm,
            boost_psi: packet.boost_psi,
            fuel: packet.fuel,
            distance_traveled_m: packet.distance_traveled_m,

            best_lap_s: packet.best_lap_s,
            last_lap_s: packet.last_lap_s,
            current_lap_s: packet.current_lap_s,
            current_race_time_s: packet.current_race_time_s,

            lap_number: packet.lap_number,
            race_position: packet.race_position,

            accel: packet.accel,
            brake: packet.brake,
            clutch: packet.clutch,
            handbrake: packet.handbrake,
            gear: packet.gear,
            steer: packet.steer,
            normalized_driving_line: packet.normalized_driving_line,
            normalized_ai_brake_difference: packet.normalized_ai_brake_difference,

            tire_wear_fl: wheel_lane(packet.tire_wear, |w| w.fl),
            tire_wear_fr: wheel_lane(packet.tire_wear, |w| w.fr),
            tire_wear_rl: wheel_lane(packet.tire_wear, |w| w.rl),
            tire_wear_rr: wheel_lane(packet.tire_wear, |w| w.rr),
            track_id: packet.track_id,

            // Cheap per-frame normalizations of Forza's wire-format
            // bytes into the conventional analog ranges. See
            // `docs/math.md#cheap-per-frame-normalizations` for the
            // tilde-notation summary; in short:
            //
            //   ~s = s / 127  in [-1, +1]   (i8 steering)
            //   ~a = a / 255  in [ 0,  1]   (u8 throttle)
            //   ~b = b / 255  in [ 0,  1]   (u8 brake)
            //   ~c = c / 255  in [ 0,  1]   (u8 clutch)
            //   ~h = h / 255  in [ 0,  1]   (u8 handbrake)
            //
            // Steering uses 127 (not 128) so the output range is exactly
            // symmetric, at the cost of one unreachable code at -128.
            steer_normalized: packet.steer.map(|s| (s as f32) / 127.0),
            accel_normalized: packet.accel.map(|v| (v as f32) / 255.0),
            brake_normalized: packet.brake.map(|v| (v as f32) / 255.0),
            clutch_normalized: packet.clutch.map(|v| (v as f32) / 255.0),
            handbrake_normalized: packet.handbrake.map(|v| (v as f32) / 255.0),

            body_slip_angle_rad: None,
            acceleration_long_lp: None,
            acceleration_lat_lp: None,
            yaw_rate_lp: None,

            suspension_travel_relative_fl: None,
            suspension_travel_relative_fr: None,
            suspension_travel_relative_rl: None,
            suspension_travel_relative_rr: None,

            ride_height_baseline_fl: None,
            ride_height_baseline_fr: None,
            ride_height_baseline_rl: None,
            ride_height_baseline_rr: None,

            normal_load_fl: None,
            normal_load_fr: None,
            normal_load_rl: None,
            normal_load_rr: None,

            wheel_radius_learned_f: None,
            wheel_radius_learned_r: None,

            steady_state_flags: None,
            time_in_state_ms: None,
        }
    }
}

fn wheel_lane<F>(w: Option<Wheel<f32>>, get: F) -> Option<f32>
where
    F: Fn(Wheel<f32>) -> f32,
{
    w.map(get)
}
