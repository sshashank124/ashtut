#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{
    spirv_std::{glam::Vec4, spirv},
    Vertex,
};

#[spirv(vertex)]
pub fn vert_main(
    vertex: Vertex,
    #[spirv(position, invariant)] position: &mut Vec4,
    color: &mut Vec4,
) {
    *position = vertex.position.extend(0.0).extend(1.0);
    *color = vertex.color.extend(1.0);
}

#[spirv(fragment)]
pub fn frag_main(color: Vec4, out_color: &mut Vec4) {
    *out_color = color;
}
