#![cfg_attr(target_arch = "spirv", no_std)]

use core::ops::{Div, Mul};

pub use bytemuck;
pub use spirv_std;
pub use spirv_std::glam;

use glam::{Mat4, Vec2, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformObjects {
    pub view: Transform,
    pub proj: Transform,
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
pub struct Transform {
    pub forward: Mat4,
    pub inverse: Mat4,
}
unsafe impl bytemuck::Zeroable for Transform {}
unsafe impl bytemuck::Pod for Transform {}

impl Vertex {
    pub const fn new(position: &[f32], tex_coord: &[f32]) -> Self {
        Self {
            position: Vec3::from_slice(position),
            tex_coord: Vec2::from_slice(tex_coord),
        }
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
        mat.y_axis.y *= -1.0;
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
