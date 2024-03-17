use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use glsl::GlslStruct;

#[repr(C)]
#[derive(Copy, Clone, Default, GlslStruct, Pod, Zeroable)]
pub struct SceneDesc {
    pub vertices_address: u64,
    pub indices_address: u64,
    pub materials_address: u64,
    pub primitives_address: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Deserialize, Serialize, GlslStruct, Pod, Zeroable)]
pub struct Vertex {
    pub position: glam::Vec4,
    pub normal: glam::Vec4,
    pub tex_coords: glam::Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Deserialize, Serialize, GlslStruct, Pod, Zeroable)]
pub struct Material {
    pub color: glam::Vec3,
    pub color_texture: i32,
    pub emittance: glam::Vec3,
    pub emittance_texture: i32,
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Deserialize, Serialize, GlslStruct, Pod, Zeroable)]
pub struct PrimitiveInfo {
    pub indices_offset: u32,
    pub vertices_offset: u32,
    pub material: u32,
}

impl Vertex {
    pub fn new(position: &[f32], normal: &[f32], tex_coord0: &[f32], tex_coord1: &[f32]) -> Self {
        Self {
            position: glam::Vec3::from_slice(position).extend(1.0),
            normal: glam::Vec3::from_slice(normal).extend(1.0),
            tex_coords: glam::Vec4::new(tex_coord0[0], tex_coord0[1], tex_coord1[0], tex_coord1[1]),
        }
    }
}

// (((position, normal), tex_coord0), tex_coord1)
type RawData = ((([f32; 3], [f32; 3]), [f32; 2]), [f32; 2]);
impl From<RawData> for Vertex {
    fn from((((position, normal), tex_coord0), tex_coord1): RawData) -> Self {
        Self::new(&position, &normal, &tex_coord0, &tex_coord1)
    }
}
