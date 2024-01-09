use glam::*;

use super::rand::Rng;

#[derive(Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Default)]
pub struct Payload {
    pub ray: Ray,
    pub hit_value: Vec3,
    pub rng: Rng,
    pub weight: Vec3,
    pub depth: u32,
}
