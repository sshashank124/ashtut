#![no_std]

use spirv_std::{
    glam::{vec2, Vec2, Vec4},
    image::{Image2d, SampledImage},
    spirv,
};

const GAMMA: f32 = 1. / 2.2;

#[spirv(vertex)]
pub fn vert_main(
    #[spirv(vertex_index)] vertex_index: u32,
    #[spirv(position, invariant)] position: &mut Vec4,
    uv: &mut Vec2,
) {
    *uv = vec2(((vertex_index << 1) & 2) as f32, (vertex_index & 2) as f32);
    *position = (2. * *uv - 1.).extend(0.).extend(1.);
}

#[spirv(fragment)]
pub fn frag_main(
    #[spirv(descriptor_set = 0, binding = 0)] texture: &SampledImage<Image2d>,
    uv: Vec2,
    color: &mut Vec4,
) {
    let raw = texture.sample(uv);
    *color = raw.powf(GAMMA);
}
