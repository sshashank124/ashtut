#[cfg(not(target_arch = "spirv"))]
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Clone, Copy, Default)]
#[cfg_attr(
    not(target_arch = "spirv"),
    derive(bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize)
)]
pub struct Material {
    pub color: glam::Vec4,
    pub emittance: glam::Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
#[cfg_attr(
    not(target_arch = "spirv"),
    derive(bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize)
)]
pub struct PrimitiveInfo {
    pub indices_offset: u32,
    pub vertices_offset: u32,
    pub material: u32,
}

#[cfg_attr(not(target_arch = "spirv"), derive(Deserialize, Serialize))]
pub struct PrimitiveSize {
    pub indices_size: u32,
    pub vertices_size: u32,
}

#[cfg_attr(not(target_arch = "spirv"), derive(Deserialize, Serialize))]
pub struct Instance {
    pub primitive_index: usize,
    pub transform: glam::Mat4,
}

impl PrimitiveSize {
    pub const fn count(&self) -> u32 {
        self.indices_size / 3
    }
}
