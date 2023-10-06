#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{glam::Vec4, spirv_std::spirv, UniformObjects, Vertex};

#[spirv(vertex)]
pub fn vert_main(
    vertex: Vertex,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    #[spirv(position, invariant)] position: &mut Vec4,
    color: &mut Vec4,
) {
    let transforms = uniforms.transforms;
    *position = transforms.proj
        * transforms.view
        * transforms.model
        * vertex.position.extend(0.0).extend(1.0);
    *color = vertex.color.extend(1.0);
}

#[spirv(fragment)]
pub fn frag_main(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}
