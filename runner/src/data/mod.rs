pub mod bounding_box;
pub mod gltf_scene;

use shared::glam::Mat4;

#[derive(Clone, Debug)]
pub struct Primitive {
    pub indices: Range,
    pub vertices: Range,
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub primitive_index: usize,
    pub transform: Mat4,
}

#[derive(Clone, Debug)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Primitive {
    pub const fn count(&self) -> usize {
        self.indices.count() / 3
    }
}

impl Range {
    pub const fn count(&self) -> usize {
        self.end - self.start
    }
}
