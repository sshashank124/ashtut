pub mod gltf;
pub mod io;

use serde::{Deserialize, Serialize};

pub use shared::scene::*;

#[derive(Default, Deserialize, Serialize)]
pub struct Scene {
    pub data: Data,
    pub info: Info,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Data {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
    pub materials: Vec<Material>,
    pub images: Vec<Image>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Info {
    pub primitive_infos: Vec<PrimitiveInfo>,
    pub primitive_sizes: Vec<PrimitiveSize>,
    pub instances: Vec<Instance>,
    pub textures: Vec<TextureInfo>,
    pub bounding_box: BoundingBox,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Image {
    pub source: std::path::PathBuf,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PrimitiveSize {
    pub indices_size: u32,
    pub vertices_size: u32,
}

#[derive(Deserialize, Serialize)]
pub struct Instance {
    pub primitive_index: usize,
    pub transform: glam::Mat4,
}

#[derive(Default, Deserialize, Serialize)]
pub struct TextureInfo {
    pub image_index: u32,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct BoundingBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
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

    #[must_use]
    pub fn transform(self, transform: glam::Mat4) -> Self {
        let a = (transform * self.min.extend(1.0)).truncate();
        let b = (transform * self.max.extend(1.0)).truncate();
        Self::new(a.min(b), a.max(b))
    }

    #[must_use]
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
