#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{
    glam::{Vec2, Vec4},
    spirv_std::{
        self,
        image::{Image, SampledImage},
        spirv,
    },
    UniformObjects, Vertex,
};

pub type TextureFormat = Image!(2D, format = rgba8, sampled);

#[spirv(vertex)]
pub fn vert_main(
    vertex: Vertex,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    #[spirv(position, invariant)] position: &mut Vec4,
    tex_coord: &mut Vec2,
) {
    let transforms = uniforms.transforms;
    *position = transforms.proj * transforms.view * transforms.model * vertex.position.extend(1.0);
    *tex_coord = vertex.tex_coord;
}

#[spirv(fragment)]
pub fn frag_main(
    tex_coord: Vec2,
    #[spirv(descriptor_set = 0, binding = 1)] texture: &SampledImage<TextureFormat>,
    out_color: &mut Vec4,
) {
    *out_color = texture.sample(tex_coord);
}
