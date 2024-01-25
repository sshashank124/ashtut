pub mod loader;

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Scene {
    pub data: Data,
    pub info: Info,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Data {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
    pub materials: Vec<Material>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Info {
    pub primitive_infos: Vec<PrimitiveInfo>,
    pub primitive_sizes: Vec<PrimitiveSize>,
    pub instances: Vec<Instance>,
    pub bounding_box: BoundingBox,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct SceneDesc {
    pub vertices_address: u64,
    pub indices_address: u64,
    pub primitives_address: u64,
    pub materials_address: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Deserialize, Serialize, Pod, Zeroable)]
pub struct Vertex {
    pub position: glam::Vec4,
    pub normal: glam::Vec4,
    pub tex_coords: glam::Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Deserialize, Serialize, Pod, Zeroable)]
pub struct Material {
    pub color: glam::Vec4,
    pub emittance: glam::Vec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Deserialize, Serialize, Pod, Zeroable)]
pub struct PrimitiveInfo {
    pub indices_offset: u32,
    pub vertices_offset: u32,
    pub material: u32,
}

#[derive(Deserialize, Serialize)]
pub struct PrimitiveSize {
    pub indices_size: u32,
    pub vertices_size: u32,
}

#[derive(Deserialize, Serialize)]
pub struct Instance {
    pub primitive_index: usize,
    pub transform: glam::Mat4,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct BoundingBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl Vertex {
    pub fn new(position: &[f32], normal: &[f32], tex_coord: &[f32]) -> Self {
        Self {
            position: glam::Vec3::from_slice(position).extend(1.0),
            normal: glam::Vec3::from_slice(normal).extend(1.0),
            tex_coords: glam::Vec2::from_slice(tex_coord).extend(0.0).extend(0.0),
        }
    }
}

type RawData = (([f32; 3], [f32; 3]), [f32; 2]); // ((position, normal), tex_coord)
impl From<RawData> for Vertex {
    fn from(((position, normal), tex_coord): RawData) -> Self {
        Self::new(&position, &normal, &tex_coord)
    }
}

impl PrimitiveSize {
    pub const fn count(&self) -> u32 {
        self.indices_size / 3
    }
}

impl BoundingBox {
    pub fn new<T: Into<glam::Vec3>>(min: T, max: T) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }

    pub fn transform(self, transform: glam::Mat4) -> Self {
        let a = (transform * self.min.extend(1.0)).truncate();
        let b = (transform * self.max.extend(1.0)).truncate();
        Self::new(a.min(b), a.max(b))
    }

    pub fn union(self, other: Self) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    pub fn center(&self) -> glam::Vec3 {
        (self.min + self.max) / 2.
    }

    pub fn size(&self) -> glam::Vec3 {
        self.max - self.min
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::new(glam::Vec3::INFINITY, glam::Vec3::NEG_INFINITY)
    }
}
