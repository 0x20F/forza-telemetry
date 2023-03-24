mod sled;
mod space;
mod dash;

use self::{sled::General, dash::{Dash, Lap}};

use super::bytes;
use sled::{ Engine, Wheels, Tires, Suspension };
use space::Spatial;

#[derive(Default, Debug)]
pub struct Packet {
    // ----------------------- sled
    // --------------------------------
    pub is_race_on: bool,         // = 1 when race is on, = 0 when in menus/race stopped
    pub timestamp_ms: u32,       // can overflow to 0 eventually

    pub general: General,

    // All the sled/car information
    // engine information
    pub engine: Engine,

    // wheel and tire information
    pub wheels: Wheels,

    // suspension information
    pub suspension: Suspension,

    // spatial information
    pub spatial: Spatial,

    // non-dimensional surface rumble values passed to controller force feedback
    pub surface_rumble_front_left: f32,
    pub surface_rumble_front_right: f32,
    pub surface_rumble_rear_left: f32,
    pub surface_rumble_rear_right: f32,

    // ----------------------- dash
    // --------------------------------
    pub dash: Dash,
}

impl Packet {
    pub fn new(data: &[u8]) -> Packet {
        Packet {
            is_race_on:         bytes::read_bool(data, 0),
            timestamp_ms:       bytes::read_u32(data, 4),

            general: General {
                ordinal:            bytes::read_u8(data, 212),
                class:              bytes::read_u8(data, 216),
                performance_index:  bytes::read_u8(data, 220),
                drive_train:        bytes::read_u8(data, 224),
                cylinders:          bytes::read_u8(data, 228)
            },

            engine: Engine {
                engine_max_rpm:     bytes::read_f32(data, 8),
                engine_idle_rpm:    bytes::read_f32(data, 12),
                current_engine_rpm: bytes::read_f32(data, 16),
            },

            spatial: Spatial {
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
                roll:               bytes::read_f32(data, 64)
            },

            wheels: Wheels {
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

                tires: Tires {
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
                    tire_temp_rear_right:           bytes::read_f32(data, 268)
                },
            },

            suspension: Suspension {
                normalized_suspension_travel_front_left:    bytes::read_f32(data, 68),
                normalized_suspension_travel_front_right:   bytes::read_f32(data, 72),
                normalized_suspension_travel_rear_left:     bytes::read_f32(data, 76),
                normalized_suspension_travel_rear_right:    bytes::read_f32(data, 80),

                suspension_travel_meters_front_left:    bytes::read_f32(data, 196),
                suspension_travel_meters_front_right:   bytes::read_f32(data, 200),
                suspension_travel_meters_rear_left:     bytes::read_f32(data, 204),
                suspension_travel_meters_rear_right:    bytes::read_f32(data, 208)
            },

            surface_rumble_front_left:  bytes::read_f32(data, 148),
            surface_rumble_front_right: bytes::read_f32(data, 152),
            surface_rumble_rear_left:   bytes::read_f32(data, 156),
            surface_rumble_rear_right:  bytes::read_f32(data, 160),

            dash: Dash {
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

                lap: Lap {
                    number:             bytes::read_u16(data, 300),
                    best:               bytes::read_f32(data, 284),
                    last:               bytes::read_f32(data, 288),
                    current:            bytes::read_f32(data, 292),
                    current_race_time:  bytes::read_f32(data, 296),
                    race_position:      bytes::read_u8(data, 302)
                }
            }
        }
    }
}