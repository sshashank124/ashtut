#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{
    glam::{Vec2, Vec4},
    spirv_std::spirv,
    PushConstants, UniformObjects, Vertex,
};

#[spirv(vertex)]
pub fn vert_main(
    #[spirv(push_constant)] constants: &PushConstants,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    vertex: Vertex,
    #[spirv(position, invariant)] position: &mut Vec4,
    tex_coord: &mut Vec2,
) {
    *position =
        uniforms.proj * (uniforms.view * constants.model_transform * vertex.position.extend(1.0));
    *tex_coord = vertex.tex_coord;
}

#[spirv(fragment)]
pub fn frag_main(tex_coord: Vec2, out_color: &mut Vec4) {
    *out_color = tex_coord.extend(1.).extend(1.);
}
