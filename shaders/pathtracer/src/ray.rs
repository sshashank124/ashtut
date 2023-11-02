use shared::glam::*;

#[derive(Default)]
pub struct Payload {
    pub origin: Vec3,
    pub direction: Vec3,
    pub hit_value: Vec3,
    pub weight: Vec3,
    pub depth: u32,
}
