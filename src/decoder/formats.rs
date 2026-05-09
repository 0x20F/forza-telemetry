//! Per-source offset tables and length-dispatched decoding.
//!
//! The Forza Data Out wire format has a fixed sled prefix (232 bytes), an
//! optional dash-extras block (79 bytes), and a few flavor-specific tails.
//! We dispatch on packet length to pick the right offset table.
//!
//! Offset table provenance: Forza Motorsport Data Out documentation, also
//! cross-checked against community references. All multi-byte fields are
//! little-endian.

use super::bytes;
use super::packet::{RawPacket, Wheel};
use super::units::f_to_c;
use super::DecodeError;

/// The wire format flavor a packet is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// 232 bytes - sled-only telemetry (no dash extras).
    Sled,
    /// 311 bytes - sled + dash (FM7 / FM 2023 base).
    DashFm,
    /// 324 bytes - sled + 12-byte gap + dash (FH4 / FH5).
    DashHorizon,
    /// 331 bytes - FM 2023 dash + tire wear + track ordinal.
    Fm2023Extras,
}

impl Source {
    pub fn from_len(len: usize) -> Result<Self, DecodeError> {
        match len {
            232 => Ok(Source::Sled),
            311 => Ok(Source::DashFm),
            324 => Ok(Source::DashHorizon),
            331 => Ok(Source::Fm2023Extras),
            other => Err(DecodeError::UnsupportedLength(other)),
        }
    }

    /// Stable string used in CSV / sidecar / logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Sled => "sled",
            Source::DashFm => "dash_fm",
            Source::DashHorizon => "dash_horizon",
            Source::Fm2023Extras => "fm2023_extras",
        }
    }
}

// Sled offsets (shared by every source).
mod sled {
    pub const IS_RACE_ON: usize = 0;
    pub const TIMESTAMP_MS: usize = 4;
    pub const ENGINE_MAX_RPM: usize = 8;
    pub const ENGINE_IDLE_RPM: usize = 12;
    pub const CURRENT_ENGINE_RPM: usize = 16;
    pub const ACCELERATION: usize = 20; // 3 floats
    pub const VELOCITY: usize = 32; // 3 floats
    pub const ANGULAR_VELOCITY: usize = 44; // 3 floats
    pub const YAW: usize = 56;
    pub const PITCH: usize = 60;
    pub const ROLL: usize = 64;
    pub const NORM_SUSP_TRAVEL: usize = 68; // wheel<f32>
    pub const TIRE_SLIP_RATIO: usize = 84;
    pub const WHEEL_ROTATION_SPEED: usize = 100;
    pub const WHEEL_ON_RUMBLE: usize = 116; // wheel<i32-as-bool>
    pub const WHEEL_IN_PUDDLE_DEPTH: usize = 132;
    pub const SURFACE_RUMBLE: usize = 148;
    pub const TIRE_SLIP_ANGLE: usize = 164;
    pub const TIRE_COMBINED_SLIP: usize = 180;
    pub const SUSP_TRAVEL_METERS: usize = 196;
    pub const CAR_ORDINAL: usize = 212;
    pub const CAR_CLASS: usize = 216;
    pub const CAR_PERFORMANCE_INDEX: usize = 220;
    pub const DRIVETRAIN_TYPE: usize = 224;
    pub const NUM_CYLINDERS: usize = 228;
}

// Dash extras layout, expressed relative to the start of the dash block.
// In FM packets the dash block starts at byte 232; in Horizon packets it
// starts at byte 244 (sled + 12-byte gap).
mod dash {
    pub const POSITION: usize = 0; // 3 floats
    pub const SPEED: usize = 12;
    pub const POWER: usize = 16;
    pub const TORQUE: usize = 20;
    pub const TIRE_TEMP: usize = 24; // wheel<f32>, Fahrenheit on the wire
    pub const BOOST: usize = 40;
    pub const FUEL: usize = 44;
    pub const DISTANCE_TRAVELED: usize = 48;
    pub const BEST_LAP: usize = 52;
    pub const LAST_LAP: usize = 56;
    pub const CURRENT_LAP: usize = 60;
    pub const CURRENT_RACE_TIME: usize = 64;
    pub const LAP_NUMBER: usize = 68; // u16
    pub const RACE_POSITION: usize = 70; // u8
    pub const ACCEL: usize = 71;
    pub const BRAKE: usize = 72;
    pub const CLUTCH: usize = 73;
    pub const HANDBRAKE: usize = 74;
    pub const GEAR: usize = 75;
    pub const STEER: usize = 76; // i8
    pub const NORMALIZED_DRIVING_LINE: usize = 77; // i8
    pub const NORMALIZED_AI_BRAKE_DIFFERENCE: usize = 78; // i8
}

const FM_DASH_BASE: usize = 232;
const HORIZON_DASH_BASE: usize = 244; // sled + 12-byte gap

// FM 2023 extras tail, relative to the end of the FM dash block (byte 311).
mod fm2023 {
    pub const TIRE_WEAR: usize = 311; // wheel<f32>
    pub const TRACK_ORDINAL: usize = 327; // i32
}

pub(super) fn decode_with_source(
    buf: &[u8],
    source: Source,
    recv_time_ns: u64,
) -> Result<RawPacket, DecodeError> {
    let mut packet = decode_sled(buf, source, recv_time_ns)?;

    let dash_base = match source {
        Source::Sled => None,
        Source::DashFm | Source::Fm2023Extras => Some(FM_DASH_BASE),
        Source::DashHorizon => Some(HORIZON_DASH_BASE),
    };

    if let Some(base) = dash_base {
        decode_dash(buf, base, &mut packet)?;
    }

    if matches!(source, Source::Fm2023Extras) {
        decode_fm2023_extras(buf, &mut packet)?;
    }

    Ok(packet)
}

fn decode_sled(buf: &[u8], source: Source, recv_time_ns: u64) -> Result<RawPacket, DecodeError> {
    use sled::*;

    let acceleration_local = [
        bytes::read_f32(buf, ACCELERATION)?,
        bytes::read_f32(buf, ACCELERATION + 4)?,
        bytes::read_f32(buf, ACCELERATION + 8)?,
    ];
    let velocity_local = [
        bytes::read_f32(buf, VELOCITY)?,
        bytes::read_f32(buf, VELOCITY + 4)?,
        bytes::read_f32(buf, VELOCITY + 8)?,
    ];
    let angular_velocity = [
        bytes::read_f32(buf, ANGULAR_VELOCITY)?,
        bytes::read_f32(buf, ANGULAR_VELOCITY + 4)?,
        bytes::read_f32(buf, ANGULAR_VELOCITY + 8)?,
    ];

    Ok(RawPacket {
        source,
        recv_time_ns,
        is_race_on: bytes::read_i32(buf, IS_RACE_ON)? != 0,
        timestamp_ms: bytes::read_u32(buf, TIMESTAMP_MS)?,
        engine_max_rpm: bytes::read_f32(buf, ENGINE_MAX_RPM)?,
        engine_idle_rpm: bytes::read_f32(buf, ENGINE_IDLE_RPM)?,
        current_engine_rpm: bytes::read_f32(buf, CURRENT_ENGINE_RPM)?,
        acceleration_local,
        velocity_local,
        angular_velocity,
        yaw_world: bytes::read_f32(buf, YAW)?,
        pitch_world: bytes::read_f32(buf, PITCH)?,
        roll_world: bytes::read_f32(buf, ROLL)?,
        normalized_suspension_travel: bytes::read_wheel_f32(buf, NORM_SUSP_TRAVEL)?,
        suspension_travel_meters: bytes::read_wheel_f32(buf, SUSP_TRAVEL_METERS)?,
        tire_slip_ratio: bytes::read_wheel_f32(buf, TIRE_SLIP_RATIO)?,
        tire_slip_angle: bytes::read_wheel_f32(buf, TIRE_SLIP_ANGLE)?,
        tire_combined_slip: bytes::read_wheel_f32(buf, TIRE_COMBINED_SLIP)?,
        wheel_rotation_speed: bytes::read_wheel_f32(buf, WHEEL_ROTATION_SPEED)?,
        wheel_on_rumble: bytes::read_wheel_bool_i32(buf, WHEEL_ON_RUMBLE)?,
        wheel_in_puddle_depth: bytes::read_wheel_f32(buf, WHEEL_IN_PUDDLE_DEPTH)?,
        surface_rumble: bytes::read_wheel_f32(buf, SURFACE_RUMBLE)?,
        // Sled does not carry tire temps; default to zero, dash will overwrite.
        tire_temp_c: Wheel::new(0.0, 0.0, 0.0, 0.0),
        car_ordinal: bytes::read_i32(buf, CAR_ORDINAL)?,
        car_class: bytes::read_i32(buf, CAR_CLASS)?,
        performance_index: bytes::read_i32(buf, CAR_PERFORMANCE_INDEX)?,
        drivetrain_type: bytes::read_i32(buf, DRIVETRAIN_TYPE)?,
        num_cylinders: bytes::read_i32(buf, NUM_CYLINDERS)?,

        position: None,
        speed: None,
        power_w: None,
        torque_nm: None,
        boost_psi: None,
        fuel: None,
        distance_traveled_m: None,
        best_lap_s: None,
        last_lap_s: None,
        current_lap_s: None,
        current_race_time_s: None,
        lap_number: None,
        race_position: None,
        accel: None,
        brake: None,
        clutch: None,
        handbrake: None,
        gear: None,
        steer: None,
        normalized_driving_line: None,
        normalized_ai_brake_difference: None,

        tire_wear: None,
        track_id: None,
    })
}

fn decode_dash(buf: &[u8], base: usize, packet: &mut RawPacket) -> Result<(), DecodeError> {
    use dash::*;

    let position = [
        bytes::read_f32(buf, base + POSITION)?,
        bytes::read_f32(buf, base + POSITION + 4)?,
        bytes::read_f32(buf, base + POSITION + 8)?,
    ];
    let tire_temp_f = bytes::read_wheel_f32(buf, base + TIRE_TEMP)?;

    packet.position = Some(position);
    packet.speed = Some(bytes::read_f32(buf, base + SPEED)?);
    packet.power_w = Some(bytes::read_f32(buf, base + POWER)?);
    packet.torque_nm = Some(bytes::read_f32(buf, base + TORQUE)?);
    packet.tire_temp_c = tire_temp_f.map(f_to_c);
    packet.boost_psi = Some(bytes::read_f32(buf, base + BOOST)?);
    packet.fuel = Some(bytes::read_f32(buf, base + FUEL)?);
    packet.distance_traveled_m = Some(bytes::read_f32(buf, base + DISTANCE_TRAVELED)?);
    packet.best_lap_s = Some(bytes::read_f32(buf, base + BEST_LAP)?);
    packet.last_lap_s = Some(bytes::read_f32(buf, base + LAST_LAP)?);
    packet.current_lap_s = Some(bytes::read_f32(buf, base + CURRENT_LAP)?);
    packet.current_race_time_s = Some(bytes::read_f32(buf, base + CURRENT_RACE_TIME)?);
    packet.lap_number = Some(bytes::read_u16(buf, base + LAP_NUMBER)?);
    packet.race_position = Some(bytes::read_u8(buf, base + RACE_POSITION)?);
    packet.accel = Some(bytes::read_u8(buf, base + ACCEL)?);
    packet.brake = Some(bytes::read_u8(buf, base + BRAKE)?);
    packet.clutch = Some(bytes::read_u8(buf, base + CLUTCH)?);
    packet.handbrake = Some(bytes::read_u8(buf, base + HANDBRAKE)?);
    packet.gear = Some(bytes::read_u8(buf, base + GEAR)?);
    packet.steer = Some(bytes::read_i8(buf, base + STEER)?);
    packet.normalized_driving_line =
        Some(bytes::read_i8(buf, base + NORMALIZED_DRIVING_LINE)?);
    packet.normalized_ai_brake_difference =
        Some(bytes::read_i8(buf, base + NORMALIZED_AI_BRAKE_DIFFERENCE)?);

    Ok(())
}

fn decode_fm2023_extras(buf: &[u8], packet: &mut RawPacket) -> Result<(), DecodeError> {
    use fm2023::*;
    packet.tire_wear = Some(bytes::read_wheel_f32(buf, TIRE_WEAR)?);
    packet.track_id = Some(bytes::read_i32(buf, TRACK_ORDINAL)?);
    Ok(())
}
