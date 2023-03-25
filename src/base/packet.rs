use super::bytes;
use serde::Serialize;


#[derive(Serialize, Default, Debug)]
pub struct Packet {
    // ----------------------- SLED
    // --------------------------------
    pub is_race_on: bool,           // = 1 when race is on, = 0 when in menus/race stopped
    pub timestamp_ms: u32,          // can overflow to 0 eventually

    /**
     * 
     * 
     * ~ General Information
     * 
     * 
     */
    pub drive_train: u8,        // Between 100 (slowest car) and 999 (fastest car) inclusive
    pub cylinders: u8,          // Number of cylinders in the engine
    pub performance_index: u8,  // Between 100 (slowest car) and 999 (fastest car) inclusive
    pub class: u8,              // Between 0 (D -- worst cars) and 7 (X class -- best cars) inclusive
    pub ordinal: u8,             // Unique ID of the car make/model

    /**
     * 
     * 
     * ~ Engine Information
     * 
     * 
     */
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,

    /**
     * 
     * 
     * ~ Wheel information
     * 
     * 
     */
    // wheel rotation speed in radians/sec
    pub wheel_rotation_speed_front_left: f32,
    pub wheel_rotation_speed_front_right: f32,
    pub wheel_rotation_speed_rear_left: f32,
    pub wheel_rotation_speed_rear_right: f32,

    // = 1 when wheel on rumble strip, = 0 when off
    pub wheel_on_rumble_strip_front_left: bool,
    pub wheel_on_rumble_strip_front_right: bool,
    pub wheel_on_rumble_strip_rear_left: bool,
    pub wheel_on_rumble_strip_rear_right: bool,

    // = from 0 to 1, where 1 is the deepest puddle
    pub wheel_in_puddle_depth_front_left: f32,
    pub wheel_in_puddle_depth_front_right: f32,
    pub wheel_in_puddle_depth_rear_left: f32,
    pub wheel_in_puddle_depth_rear_right: f32,

    /**
     * 
     * 
     * ~ Tire information
     * 
     * 
     */
    // slip ratio; = 0 means 100% grip and |ratio| > 1.0 means loss of grip.
    pub tire_slip_rotation_front_left: f32,
    pub tire_slip_rotation_front_right: f32,
    pub tire_slip_rotation_rear_left: f32,
    pub tire_slip_rotation_rear_right: f32,

    // tire normalized slip angle, = 0 means 100% grip and |angle| > 1.0 means loss of grip.
    pub tire_slip_angle_front_left: f32,
    pub tire_slip_angle_front_right: f32,
    pub tire_slip_angle_rear_left: f32,
    pub tire_slip_angle_rear_right: f32,

    // tire normalized combined slip, = 0 means 100% grip and |slip| > 1.0 means loss of grip
    pub tire_combined_slip_front_left: f32,
    pub tire_combined_slip_front_right: f32,
    pub tire_combined_slip_rear_left: f32,
    pub tire_combined_slip_rear_right: f32,

    // These are from the V2/Dash version
    // Also in Fahrenheit so use (temp - 32) * 5/9 to get celsius
    pub tire_temp_front_left: f32,
    pub tire_temp_front_right: f32,
    pub tire_temp_rear_left: f32,
    pub tire_temp_rear_right: f32,

    /**
     * 
     * 
     * ~ Suspension Information
     * 
     * 
     */
    // suspension - travel normalized; 0.0f = max stretch; 1.0 = max compression
    pub normalized_suspension_travel_front_left: f32,
    pub normalized_suspension_travel_front_right: f32,
    pub normalized_suspension_travel_rear_left: f32,
    pub normalized_suspension_travel_rear_right: f32,

    // actual suspension travel in meters
    pub suspension_travel_meters_front_left: f32,
    pub suspension_travel_meters_front_right: f32,
    pub suspension_travel_meters_rear_left: f32,
    pub suspension_travel_meters_rear_right: f32,

    /**
     * 
     * 
     * ~ Spatial information
     * 
     * 
     */
    // meters - These 3 are also part of the V2/Dash version
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,

    // movement - in the car's local space; x = right, y = up, z = forward.
    pub acceleration_x: f32,
    pub acceleration_y: f32,
    pub acceleration_z: f32,

    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_z: f32,

    // x = pitch, y = yaw, z = roll
    pub angular_velocity_x: f32,
    pub angular_velocity_y: f32,
    pub angular_velocity_z: f32,

    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    /**
     * 
     * 
     * ~ Force feedback information
     * 
     * 
     */
    // non-dimensional surface rumble values passed to controller force feedback
    pub surface_rumble_front_left: f32,
    pub surface_rumble_front_right: f32,
    pub surface_rumble_rear_left: f32,
    pub surface_rumble_rear_right: f32,

    // ----------------------- DASH
    // --------------------------------
    /**
     * 
     * 
     * ~ Literal dashboard information
     * 
     * 
     */
    pub speed: f32,     // meters/second
    pub power: f32,     // watts
    pub torque: f32,    // newton meter

    pub boost: f32,
    pub fuel: f32,
    pub distance_traveled: f32,

    pub acceleration: u8,
    pub brake: u8,
    pub clutch: u8,
    pub handbrake: u8,
    pub gear: u8,
    pub steer: i8,

    /**
     * 
     * 
     * ~ Lap information
     * 
     * 
     */
    pub number: u16,
    pub best: f32,
    pub last: f32,
    pub current: f32,
    pub current_race_time: f32,
    pub race_position: u8,

    /**
     * 
     * 
     * ~ Game data
     * 
     * 
     */
    pub normalized_driving_line: u8,
    pub normalized_ai_brake_difference: u8,
}

impl Packet {
    pub fn new(data: &[u8]) -> Packet {
        Packet {
            is_race_on:         bytes::read_bool(data, 0),
            timestamp_ms:       bytes::read_u32(data, 4),

            ordinal:            bytes::read_u8(data, 212),
            class:              bytes::read_u8(data, 216),
            performance_index:  bytes::read_u8(data, 220),
            drive_train:        bytes::read_u8(data, 224),
            cylinders:          bytes::read_u8(data, 228),

            engine_max_rpm:     bytes::read_f32(data, 8),
            engine_idle_rpm:    bytes::read_f32(data, 12),
            current_engine_rpm: bytes::read_f32(data, 16),

            acceleration_x:     bytes::read_f32(data, 20),
            acceleration_y:     bytes::read_f32(data, 24),
            acceleration_z:     bytes::read_f32(data, 28),

            velocity_x:         bytes::read_f32(data, 32),
            velocity_y:         bytes::read_f32(data, 36),
            velocity_z:         bytes::read_f32(data, 40),

            angular_velocity_x: bytes::read_f32(data, 44),
            angular_velocity_y: bytes::read_f32(data, 48),
            angular_velocity_z: bytes::read_f32(data, 52),

            yaw:                bytes::read_f32(data, 56),
            pitch:              bytes::read_f32(data, 60),
            roll:               bytes::read_f32(data, 64),

            wheel_in_puddle_depth_front_left:   bytes::read_f32(data, 132),
            wheel_in_puddle_depth_front_right:  bytes::read_f32(data, 136),
            wheel_in_puddle_depth_rear_left:    bytes::read_f32(data, 140),
            wheel_in_puddle_depth_rear_right:   bytes::read_f32(data, 144),

            wheel_on_rumble_strip_front_left:   bytes::read_bool(data, 116),
            wheel_on_rumble_strip_front_right:  bytes::read_bool(data, 120),
            wheel_on_rumble_strip_rear_left:    bytes::read_bool(data, 124),
            wheel_on_rumble_strip_rear_right:   bytes::read_bool(data, 128),

            wheel_rotation_speed_front_left:    bytes::read_f32(data, 100),
            wheel_rotation_speed_front_right:   bytes::read_f32(data, 104),
            wheel_rotation_speed_rear_left:     bytes::read_f32(data, 108),
            wheel_rotation_speed_rear_right:    bytes::read_f32(data, 112),

            tire_slip_rotation_front_left:  bytes::read_f32(data, 84),
            tire_slip_rotation_front_right: bytes::read_f32(data, 88),
            tire_slip_rotation_rear_left:   bytes::read_f32(data, 92),
            tire_slip_rotation_rear_right:  bytes::read_f32(data, 96),

            tire_slip_angle_front_left:     bytes::read_f32(data, 164),
            tire_slip_angle_front_right:    bytes::read_f32(data, 168),
            tire_slip_angle_rear_left:      bytes::read_f32(data, 172),
            tire_slip_angle_rear_right:     bytes::read_f32(data, 176),

            tire_combined_slip_front_left:  bytes::read_f32(data, 180),
            tire_combined_slip_front_right: bytes::read_f32(data, 184),
            tire_combined_slip_rear_left:   bytes::read_f32(data, 188),
            tire_combined_slip_rear_right:  bytes::read_f32(data, 192),

            tire_temp_front_left:           bytes::read_f32(data, 256),
            tire_temp_front_right:          bytes::read_f32(data, 260),
            tire_temp_rear_left:            bytes::read_f32(data, 264),
            tire_temp_rear_right:           bytes::read_f32(data, 268),

            normalized_suspension_travel_front_left:    bytes::read_f32(data, 68),
            normalized_suspension_travel_front_right:   bytes::read_f32(data, 72),
            normalized_suspension_travel_rear_left:     bytes::read_f32(data, 76),
            normalized_suspension_travel_rear_right:    bytes::read_f32(data, 80),

            suspension_travel_meters_front_left:    bytes::read_f32(data, 196),
            suspension_travel_meters_front_right:   bytes::read_f32(data, 200),
            suspension_travel_meters_rear_left:     bytes::read_f32(data, 204),
            suspension_travel_meters_rear_right:    bytes::read_f32(data, 208),

            surface_rumble_front_left:  bytes::read_f32(data, 148),
            surface_rumble_front_right: bytes::read_f32(data, 152),
            surface_rumble_rear_left:   bytes::read_f32(data, 156),
            surface_rumble_rear_right:  bytes::read_f32(data, 160),

            position_x:         bytes::read_f32(data, 232),
            position_y:         bytes::read_f32(data, 236),
            position_z:         bytes::read_f32(data, 240),

            speed:              bytes::read_f32(data, 244),
            power:              bytes::read_f32(data, 248),
            torque:             bytes::read_f32(data, 252),

            boost:              bytes::read_f32(data, 272),
            fuel:               bytes::read_f32(data, 276),
            distance_traveled:  bytes::read_f32(data, 280),

            acceleration:       bytes::read_u8(data, 303),
            brake:              bytes::read_u8(data, 304),
            clutch:             bytes::read_u8(data, 305),
            handbrake:          bytes::read_u8(data, 306),
            gear:               bytes::read_u8(data, 307),
            steer:              bytes::read_i8(data, 308),
            normalized_driving_line:        bytes::read_u8(data, 309),
            normalized_ai_brake_difference: bytes::read_u8(data, 310),

            number:             bytes::read_u16(data, 300),
            best:               bytes::read_f32(data, 284),
            last:               bytes::read_f32(data, 288),
            current:            bytes::read_f32(data, 292),
            current_race_time:  bytes::read_f32(data, 296),
            race_position:      bytes::read_u8(data, 302)
        }
    }
}