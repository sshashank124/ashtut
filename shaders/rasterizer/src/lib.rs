#![no_std]

use glam::Vec4;

use shared::{scene::Material, RasterizerConstants, Uniforms, Vertex};
use spirv_std::spirv;

#[spirv(vertex)]
pub fn vert_main(
    #[spirv(push_constant)] constants: &RasterizerConstants,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &Uniforms,
    vertex: Vertex,
    #[spirv(position, invariant)] position: &mut Vec4,
) {
    *position =
        uniforms.camera.proj * (uniforms.camera.view * constants.model_transform * vertex.position);
}

#[spirv(fragment)]
pub fn frag_main(
    #[spirv(push_constant)] constants: &RasterizerConstants,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] materials: &[Material],
    out_color: &mut Vec4,
) {
    *out_color = materials[constants.material_index as usize].color;
}
