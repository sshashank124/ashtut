#![cfg_attr(target_arch = "spirv", no_std)]

pub use spirv_std;

use spirv_std::glam::{Vec2, Vec3};

#[repr(C)]
pub struct Vertex {
    pub position: Vec2,
    pub color: Vec3,
}

impl Vertex {
    pub const fn new(position: [f32; 2], color: [f32; 3]) -> Self {
        Self {
            position: Vec2::from_array(position),
            color: Vec3::from_array(color),
        }
    }
}
