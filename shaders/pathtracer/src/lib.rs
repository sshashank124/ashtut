#![no_std]

mod math;
mod rand;
mod ray;

use glam::{swizzles::Vec4Swizzles, *};

use math::*;
use rand::Rng;

use shared::{
    scene::{Material, PrimitiveInfo},
    PathtracerConstants, Uniforms, Vertex,
};
#[allow(unused_imports)]
use spirv_std::{num_traits::Float, ray_tracing, spirv, Image};

const MAX_RECURSE_DEPTH: u32 = 5;
const RAY_FLAGS: ray_tracing::RayFlags = ray_tracing::RayFlags::OPAQUE;
const T_MIN: f32 = 1e-3;
const T_MAX: f32 = 1e+5;
// env light just to see some results (technically not part of the rendering equation)
const ENV_COLOR: Vec3 = Vec3::new(0.01, 0.01, 0.01);

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
    #[spirv(incoming_ray_payload)] payload: &mut ray::Payload,
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

    let position = dotv(
        bary,
        [verts[0].position, verts[1].position, verts[2].position],
    );
    let world_position = (object_to_world * position).truncate();

    let world_normal = {
        let normal = dotv(bary, [verts[0].normal, verts[1].normal, verts[2].normal])
            .truncate()
            .normalize();
        (world_to_object.transpose() * normal).normalize()
    };

    let frame = {
        let tangent = if world_normal.x.abs() > world_normal.y.abs() {
            vec3(world_normal.z, 0., -world_normal.x) * world_normal.xz().length_recip()
        } else {
            vec3(0., -world_normal.z, world_normal.y) * world_normal.yz().length_recip()
        };
        let bitangent = world_normal.cross(tangent);
        Mat3::from_cols(tangent, bitangent, world_normal)
    };

    let out_dir = {
        let r1 = payload.rng.next_float();
        let r2 = 2. * core::f32::consts::PI * payload.rng.next_float();
        let sq = r1.sqrt();
        let dir = vec3(r2.cos() * sq, r2.sin() * sq, (1. - r1).sqrt());
        frame * dir
    };

    let material = materials[primitive.material as usize];

    let albedo = material.color.truncate();
    let emittance = material.emittance.truncate();

    payload.ray = ray::Ray {
        origin: world_position,
        direction: frame * out_dir,
    };
    payload.hit_value = emittance;
    payload.weight = albedo;
}

#[spirv(ray_generation)]
pub fn ray_generation(
    #[spirv(launch_id)] launch_id: UVec3,
    #[spirv(launch_size)] launch_size: UVec3,
    #[spirv(push_constant)] constants: &PathtracerConstants,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &Uniforms,
    #[spirv(descriptor_set = 1, binding = 0)] tlas: &ray_tracing::AccelerationStructure,
    #[spirv(descriptor_set = 1, binding = 1)] output_image: &OutputImage,
    #[spirv(ray_payload)] payload: &mut ray::Payload,
) {
    let ray = {
        let uv = (launch_id.as_vec3().xy() + 0.5) / launch_size.as_vec3().xy();
        let target = (uniforms.camera.proj / (2. * uv - 1.).extend(1.).extend(1.)).truncate();
        ray::Ray {
            origin: (uniforms.camera.view / Vec4::W).truncate(),
            direction: (uniforms.camera.view / target.normalize().extend(0.)).truncate(),
        }
    };

    let rng = Rng::from_seed(
        launch_size.x * (launch_size.y * constants.frame + launch_id.y) + launch_id.x,
    );

    *payload = ray::Payload {
        ray,
        rng,
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
                payload.ray.origin,
                T_MIN,
                payload.ray.direction,
                T_MAX,
                payload,
            );
        };
        total += weight * payload.hit_value;
        weight *= payload.weight;
        payload.depth += 1;
    }

    let new_color = if constants.frame > 0 {
        let w = (constants.frame as f32 + 1.).recip();
        let old_color = output_image.read(launch_id.xy()).xyz();
        old_color.lerp(total, w)
    } else {
        total
    };

    unsafe { output_image.write(launch_id.xy(), new_color.extend(1.0)) };
}

#[spirv(miss)]
pub fn miss(#[spirv(incoming_ray_payload)] out: &mut ray::Payload) {
    out.hit_value = ENV_COLOR;
    out.depth = MAX_RECURSE_DEPTH;
}
