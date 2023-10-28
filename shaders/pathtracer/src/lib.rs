#![cfg_attr(target_arch = "spirv", no_std)]

mod ray;

use shared::{
    glam::{vec3, UVec3, Vec2, Vec3Swizzles, Vec4, Vec4Swizzles},
    spirv_std::{
        self,
        ray_tracing::{AccelerationStructure, RayFlags},
        spirv, Image,
    },
    UniformObjects,
};

use ray::Payload;

pub type OutputImage = Image!(2D, format = rgba32f, sampled = false);

#[spirv(closest_hit)]
pub fn closest_hit(
    #[spirv(hit_attribute)] hit_uv: &Vec2,
    #[spirv(incoming_ray_payload)] out: &mut Payload,
) {
    out.hit_value = hit_uv.extend(1.);
}

#[spirv(ray_generation)]
pub fn ray_generation(
    #[spirv(launch_id)] launch_id: UVec3,
    #[spirv(launch_size)] launch_size: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    #[spirv(descriptor_set = 1, binding = 0)] tlas: &AccelerationStructure,
    #[spirv(descriptor_set = 1, binding = 1)] output_image: &OutputImage,
    #[spirv(ray_payload)] payload: &mut ray::Payload,
) {
    let uv = (launch_id.as_vec3().xy() + 0.5) / launch_size.as_vec3().xy();

    let origin = uniforms.view / Vec4::W;
    let target = uniforms.proj / (uv * 2.0 - 1.0).extend(1.0).extend(1.0);
    let direction = uniforms.view / target.xyz().normalize().extend(0.0);

    let t_min = 0.001;
    let t_max = 1000.0;

    unsafe {
        tlas.trace_ray(
            RayFlags::OPAQUE,
            0xff,
            0,
            0,
            0,
            origin.xyz(),
            t_min,
            direction.xyz(),
            t_max,
            payload,
        );

        output_image.write(launch_id.xy(), payload.hit_value.extend(1.0));
    };
}

#[spirv(miss)]
pub fn miss(#[spirv(incoming_ray_payload)] out: &mut Payload) {
    out.hit_value = vec3(0.1, 0.1, 0.4);
}
