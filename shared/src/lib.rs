#![cfg_attr(target_arch = "spirv", no_std)]

pub use bytemuck;
pub use spirv_std;
pub use spirv_std::glam;

use glam::{Mat4, Vec2, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformObjects {
    pub transforms: ModelViewProjection,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Vertex {
    pub position: Vec3,
    pub tex_coord: Vec2,
}
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ModelViewProjection {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}
unsafe impl bytemuck::Zeroable for ModelViewProjection {}
unsafe impl bytemuck::Pod for ModelViewProjection {}

impl Vertex {
    pub const fn new(position: &[f32], tex_coord: &[f32]) -> Self {
        Self {
            position: Vec3::from_slice(position),
            tex_coord: Vec2::from_slice(tex_coord),
        }
    }
}

impl ModelViewProjection {
    pub fn new(model: Mat4, view: Mat4, mut proj: Mat4) -> Self {
        proj.y_axis.y *= -1.0;
        Self { model, view, proj }
    }
}
