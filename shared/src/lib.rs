#![cfg_attr(target_arch = "spirv", no_std)]

pub mod bounding_box;
pub mod scene;

use core::ops::{Div, Mul};

#[cfg(not(target_arch = "spirv"))]
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(not(target_arch = "spirv"), derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct RasterizerConstants {
    pub model_transform: glam::Mat4,
    pub material_index: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(not(target_arch = "spirv"), derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct PathtracerConstants {
    pub frame: u32,
}
#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(not(target_arch = "spirv"), derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Uniforms {
    pub camera: Camera,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(not(target_arch = "spirv"), derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Camera {
    pub view: Transform,
    pub proj: Transform,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(
    not(target_arch = "spirv"),
    derive(bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize)
)]
pub struct Vertex {
    pub position: glam::Vec4,
    pub normal: glam::Vec4,
    pub tex_coord: glam::Vec2,
    pub _pad0: u32,
    pub _pad1: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
#[cfg_attr(not(target_arch = "spirv"), derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Transform {
    pub forward: glam::Mat4,
    pub inverse: glam::Mat4,
}

impl Vertex {
    pub fn new(position: &[f32], normal: &[f32], tex_coord: &[f32]) -> Self {
        Self {
            position: glam::Vec3::from_slice(position).extend(1.0),
            normal: glam::Vec3::from_slice(normal).extend(1.0),
            tex_coord: glam::Vec2::from_slice(tex_coord),
            ..Default::default()
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
    pub fn new(mat: glam::Mat4) -> Self {
        Self {
            forward: mat,
            inverse: mat.inverse(),
        }
    }

    pub fn proj(mut mat: glam::Mat4) -> Self {
        mat.y_axis.y *= -1.;
        Self::new(mat)
    }
}

impl<T> Mul<T> for Transform
where
    glam::Mat4: Mul<T>,
{
    type Output = <glam::Mat4 as Mul<T>>::Output;
    fn mul(self, rhs: T) -> Self::Output {
        self.forward * rhs
    }
}

impl<T> Div<T> for Transform
where
    glam::Mat4: Mul<T>,
{
    type Output = <glam::Mat4 as Mul<T>>::Output;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: T) -> Self::Output {
        self.inverse * rhs
    }
}
