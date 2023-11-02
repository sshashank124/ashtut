use core::ops::{Add, Mul};

use shared::{glam::*, PrimitiveInfo, Vertex};

pub fn triangle(
    indices: &[u32],
    vertices: &[Vertex],
    primitive: &PrimitiveInfo,
    primitive_id: usize,
) -> [Vertex; 3] {
    let indices_offset = primitive.indices_offset as usize + (3 * primitive_id);
    let vertices_offset = primitive.vertices_offset as usize;
    [
        vertices[indices[indices_offset] as usize + vertices_offset],
        vertices[indices[indices_offset + 1] as usize + vertices_offset],
        vertices[indices[indices_offset + 2] as usize + vertices_offset],
    ]
}

pub fn barycentrics(uv: &Vec2) -> Vec3 {
    vec3(1.0 - uv.x - uv.y, uv.x, uv.y)
}

pub fn dotv<T>(s: Vec3, v: [T; 3]) -> T
where
    f32: Mul<T, Output = T>,
    T: Add<Output = T> + Copy,
{
    s.x * v[0] + s.y * v[1] + s.z * v[2]
}
