use bytemuck::{Pod, Zeroable};

use core::ops::{Div, Mul};

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct RasterizerConstants {
    pub model_transform: glam::Mat4,
    pub material_index: u32,
    pub _pad: glam::Vec3,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct PathtracerConstants {
    pub frame: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct Uniforms {
    pub camera: Camera,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct Camera {
    pub view: Transform,
    pub proj: Transform,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct Transform {
    pub forward: glam::Mat4,
    pub inverse: glam::Mat4,
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
