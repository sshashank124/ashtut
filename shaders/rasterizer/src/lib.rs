#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{glam::Vec4, spirv_std::spirv, Material, PushConstants, UniformObjects, Vertex};

#[spirv(vertex)]
pub fn vert_main(
    #[spirv(push_constant)] constants: &PushConstants,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    vertex: Vertex,
    #[spirv(position, invariant)] position: &mut Vec4,
) {
    *position =
        uniforms.proj * (uniforms.view * constants.model_transform * vertex.position.extend(1.0));
}

#[spirv(fragment)]
pub fn frag_main(
    #[spirv(push_constant)] constants: &PushConstants,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] materials: &[Material],
    out_color: &mut Vec4,
) {
    *out_color = materials[constants.material_index as usize].color;
}
