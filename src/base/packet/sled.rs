#[derive(Debug, Default)]
pub struct General {
    pub drive_train: u8,        // Between 100 (slowest car) and 999 (fastest car) inclusive
    pub cylinders: u8,          // Number of cylinders in the engine
    pub performance_index: u8,  // Between 100 (slowest car) and 999 (fastest car) inclusive
    pub class: u8,              // Between 0 (D -- worst cars) and 7 (X class -- best cars) inclusive
    pub ordinal: u8             // Unique ID of the car make/model
}

#[derive(Debug, Default)]
pub struct Engine {
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,
}

#[derive(Debug, Default)]
pub struct Wheels {
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

    pub tires: Tires
}

#[derive(Debug, Default)]
pub struct Tires {
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
}

#[derive(Debug, Default)]
pub struct Suspension {
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
}