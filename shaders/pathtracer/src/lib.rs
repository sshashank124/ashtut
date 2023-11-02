#![cfg_attr(target_arch = "spirv", no_std)]

mod math;
mod ray;

use math::*;
use ray::Payload;

#[allow(unused_imports)]
use shared::spirv_std::num_traits::Float;
use shared::{
    glam::*,
    spirv_std::{self, *},
    Material, PrimitiveInfo, UniformObjects, Vertex,
};

pub type OutputImage = Image!(2D, format = rgba32f, sampled = false);

#[spirv(matrix)]
pub struct Affine3 {
    x: Vec3,
    y: Vec3,
    z: Vec3,
    w: Vec3,
}

#[allow(clippy::too_many_arguments)]
#[spirv(closest_hit)]
pub fn closest_hit(
    #[spirv(primitive_id)] primitive_id: usize,
    #[spirv(instance_custom_index)] primitive_index: usize,
    #[spirv(object_to_world)] otw: Affine3,
    #[spirv(world_to_object)] wto: Affine3,
    #[spirv(hit_attribute)] hit_uv: &Vec2,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] materials: &[Material],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 2)] indices: &[u32],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 3)] vertices: &[Vertex],
    #[spirv(storage_buffer, descriptor_set = 1, binding = 4)] primitives: &[PrimitiveInfo],
    #[spirv(incoming_ray_payload)] out: &mut Payload,
) {
    let bary = barycentrics(hit_uv);
    let object_to_world = Mat4::from_cols(
        otw.x.extend(0.),
        otw.y.extend(0.),
        otw.z.extend(0.),
        otw.w.extend(1.),
    );
    let world_to_object = Mat3::from_cols(wto.x, wto.y, wto.z);

    let primitive = &primitives[primitive_index];
    let verts = triangle(indices, vertices, primitive, primitive_id);

    let position = Vec3::from(dotv(
        bary,
        [verts[0].position, verts[1].position, verts[2].position],
    ));
    let world_position = (object_to_world * position.extend(1.)).truncate();

    let normal = Vec3::from(dotv(
        bary,
        [verts[0].normal, verts[1].normal, verts[2].normal],
    ))
    .normalize();
    let world_normal = (world_to_object.transpose() * normal).normalize();

    let tangent = if world_normal.x.abs() > world_normal.y.abs() {
        vec3(world_normal.z, 0., -world_normal.x) * world_normal.xz().length_recip()
    } else {
        vec3(0., -world_normal.z, world_normal.y) * world_normal.yz().length_recip()
    };
    let _bitangent = world_normal.cross(tangent);

    let ray_origin = world_position;
    let ray_direction = world_normal;

    let material = materials[primitive.material as usize];

    let albedo = material.color.truncate();
    let emittance = material.emittance.truncate();

    out.origin = ray_origin;
    out.direction = ray_direction;
    out.hit_value = emittance;
    out.weight = albedo;
}

const MAX_RECURSE_DEPTH: u32 = 2;
const RAY_FLAGS: ray_tracing::RayFlags = ray_tracing::RayFlags::OPAQUE;
const T_MIN: f32 = 1e-3;
const T_MAX: f32 = 1e+5;

#[spirv(ray_generation)]
pub fn ray_generation(
    #[spirv(launch_id)] launch_id: UVec3,
    #[spirv(launch_size)] launch_size: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &UniformObjects,
    #[spirv(descriptor_set = 1, binding = 0)] tlas: &ray_tracing::AccelerationStructure,
    #[spirv(descriptor_set = 1, binding = 1)] output_image: &OutputImage,
    #[spirv(ray_payload)] payload: &mut ray::Payload,
) {
    let uv = (launch_id.as_vec3().xy() + 0.5) / launch_size.as_vec3().xy();

    let origin = (uniforms.view / Vec4::W).truncate();
    let target = (uniforms.proj / (2. * uv - 1.).extend(1.).extend(1.)).truncate();
    let direction = (uniforms.view / target.normalize().extend(0.)).truncate();

    *payload = Payload {
        origin,
        direction,
        ..Default::default()
    };

    let mut total = Vec3::ZERO;
    let mut weight = Vec3::ONE;
    while payload.depth < MAX_RECURSE_DEPTH {
        unsafe {
            tlas.trace_ray(
                RAY_FLAGS,
                0xff,
                0,
                0,
                0,
                payload.origin,
                T_MIN,
                payload.direction,
                T_MAX,
                payload,
            );
        };
        total += weight * payload.hit_value;
        weight *= payload.weight;
        payload.depth += 1;
    }

    unsafe { output_image.write(launch_id.xy(), total.extend(1.0)) };
}

#[spirv(miss)]
pub fn miss(#[spirv(incoming_ray_payload)] out: &mut Payload) {
    const ENV_COLOR: Vec3 = Vec3::new(0.05, 0.01, 0.01);
    // env light just to see some results (technically not part of the rendering equation)
    out.hit_value = ENV_COLOR;
    out.depth = MAX_RECURSE_DEPTH;
}
