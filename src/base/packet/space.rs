#[derive(Debug, Default)]
pub struct Spatial {
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
}