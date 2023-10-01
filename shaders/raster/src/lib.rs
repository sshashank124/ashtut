#![no_std]

use spirv_std::glam::{vec2, vec3, Vec2, Vec3, Vec4};
use spirv_std::spirv;

const VERTICES: [Vec2; 3] = [vec2(0.0, -0.5), vec2(-0.5, 0.5), vec2(0.5, 0.5)];

const COLORS: [Vec3; 3] = [
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, 1.0, 0.0),
];

#[spirv(vertex)]
pub fn vert_main(
    #[spirv(vertex_index)] vertex_index: i32,
    #[spirv(position, invariant)] position: &mut Vec4,
    color: &mut Vec4,
) {
    *position = Vec4::from((VERTICES[vertex_index as usize], 0.0, 1.0));
    *color = Vec4::from((COLORS[vertex_index as usize], 1.0));
}

#[spirv(fragment)]
pub fn frag_main(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}
