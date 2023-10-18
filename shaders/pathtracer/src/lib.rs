#![cfg_attr(target_arch = "spirv", no_std)]

use shared::{
    glam::{vec3, vec4, UVec3, Vec3, Vec3Swizzles},
    spirv_std::{self, ray_tracing::AccelerationStructure, spirv, Image},
};

pub type TargetFormat = Image!(2D, format = rgba32f, sampled = false);

#[spirv(ray_generation)]
pub fn ray_generation(
    #[spirv(launch_id)] launch_id: UVec3,
    #[spirv(launch_size)] _launch_size: UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] _tlas: &AccelerationStructure,
    #[spirv(descriptor_set = 0, binding = 1)] target: &TargetFormat,
) {
    unsafe { target.write(launch_id.xy(), vec4(0.5, 0.5, 0.5, 1.0)) };
}

#[spirv(miss)]
pub fn miss(#[spirv(incoming_ray_payload)] out: &mut Vec3) {
    *out = vec3(0.0, 0.1, 0.3);
}

#[spirv(closest_hit)]
pub fn closest_hit(
    #[spirv(incoming_ray_payload)] out: &mut Vec3,
    #[spirv(hit_attribute)] _attribs: &Vec3,
) {
    *out = vec3(0.2, 0.5, 0.5);
}
