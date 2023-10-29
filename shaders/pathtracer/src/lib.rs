#![cfg_attr(target_arch = "spirv", no_std)]

mod ray;

use shared::{
    glam::{vec3, UVec3, Vec2, Vec3Swizzles, Vec4, Vec4Swizzles},
    spirv_std::{
        self,
        ray_tracing::{AccelerationStructure, RayFlags},
        spirv, Image,
    },
    Material, PrimitiveInfo, UniformObjects, Vertex,
};

use ray::Payload;

pub type OutputImage = Image!(2D, format = rgba32f, sampled = false);

#[allow(clippy::too_many_arguments)]
#[spirv(closest_hit)]
pub fn closest_hit(
    #[spirv(primitive_id)] primitive_id: usize,
    #[spirv(instance_custom_index)] primitive_index: usize,
    #[spirv(hit_attribute)] hit_uv: &Vec2,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] materials: &[Material],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 2)] indices: &[u32],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 3)] vertices: &[Vertex],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 4)] primitives: &[PrimitiveInfo],
    #[spirv(incoming_ray_payload)] out: &mut Payload,
) {
    let bary = vec3(1.0 - hit_uv.x - hit_uv.y, hit_uv.x, hit_uv.y);

    let primitive = primitives[primitive_index];
    let indices_offset = primitive.indices_offset as usize + (3 * primitive_id);
    let vertices_offset = primitive.vertices_offset as usize;
    let indices = [
        indices[indices_offset] as usize + vertices_offset,
        indices[indices_offset + 1] as usize + vertices_offset,
        indices[indices_offset + 2] as usize + vertices_offset,
    ];
    let vertices = [
        vertices[indices[0]],
        vertices[indices[1]],
        vertices[indices[2]],
    ];
    let tex_coords = [
        vertices[0].tex_coord,
        vertices[1].tex_coord,
        vertices[2].tex_coord,
    ];

    let _tex_coord = bary.x * tex_coords[0] + bary.y * tex_coords[1] + bary.z * tex_coords[2];

    out.hit_value = materials[primitive.material as usize].color.xyz();
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
    out.hit_value = vec3(0.01, 0.01, 0.2);
}
