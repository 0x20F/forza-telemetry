#[derive(Debug, Default)]
pub struct Dash {
    // meters
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,

    pub speed: f32,     // meters/second
    pub power: f32,     // watts
    pub torque: f32,    // newton meter

    pub boost: f32,
    pub fuel: f32,
    pub distance_traveled: f32,

    // laps
    pub lap: Lap,

    pub acceleration: u8,
    pub brake: u8,
    pub clutch: u8,
    pub handbrake: u8,
    pub gear: u8,
    pub steer: i8,

    pub normalized_driving_line: u8,
    pub normalized_ai_brake_difference: u8
}

#[derive(Debug, Default)]
pub struct Lap {
    pub number: u16,
    pub best: f32,
    pub last: f32,
    pub current: f32,
    pub current_race_time: f32,
    pub race_position: u8
}