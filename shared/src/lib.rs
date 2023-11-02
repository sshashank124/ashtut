#![cfg_attr(target_arch = "spirv", no_std)]

use core::ops::{Div, Mul};

pub use bytemuck;
pub use spirv_std;
pub use spirv_std::glam;

use glam::{Mat4, Vec2, Vec3A, Vec4};

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct PushConstants {
    pub model_transform: Mat4,
    pub material_index: u32,
}
unsafe impl bytemuck::Zeroable for PushConstants {}
unsafe impl bytemuck::Pod for PushConstants {}

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformObjects {
    pub view: Transform,
    pub proj: Transform,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Vertex {
    pub position: Vec3A,
    pub normal: Vec3A,
    pub tex_coord: Vec2,
}
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Transform {
    pub forward: Mat4,
    pub inverse: Mat4,
}
unsafe impl bytemuck::Zeroable for Transform {}
unsafe impl bytemuck::Pod for Transform {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PrimitiveInfo {
    pub indices_offset: u32,
    pub vertices_offset: u32,
    pub material: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Material {
    pub color: Vec4,
    pub emittance: Vec4,
}
unsafe impl bytemuck::Zeroable for Material {}
unsafe impl bytemuck::Pod for Material {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneInfo {
    pub indices_address: u64,
    pub vertices_address: u64,
}

impl Vertex {
    pub fn new(position: &[f32], normal: &[f32], tex_coord: &[f32]) -> Self {
        Self {
            position: Vec3A::from_slice(position),
            normal: Vec3A::from_slice(normal),
            tex_coord: Vec2::from_slice(tex_coord),
        }
    }
}

type RawData = (([f32; 3], [f32; 3]), [f32; 2]); // ((position, normal), tex_coord)
impl From<RawData> for Vertex {
    fn from(((position, normal), tex_coord): RawData) -> Self {
        Self::new(&position, &normal, &tex_coord)
    }
}

impl Transform {
    pub fn new(mat: Mat4) -> Self {
        Self {
            forward: mat,
            inverse: mat.inverse(),
        }
    }

    pub fn proj(mut mat: Mat4) -> Self {
        mat.y_axis.y *= -1.;
        Self::new(mat)
    }
}

impl<T> Mul<T> for Transform
where
    Mat4: Mul<T>,
{
    type Output = <Mat4 as Mul<T>>::Output;
    fn mul(self, rhs: T) -> Self::Output {
        self.forward * rhs
    }
}

impl<T> Div<T> for Transform
where
    Mat4: Mul<T>,
{
    type Output = <Mat4 as Mul<T>>::Output;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: T) -> Self::Output {
        self.inverse * rhs
    }
}
