#version 460
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "inputs.h.glsl"
#include "ray.common.glsl"
#include "bsdf.common.glsl"

const vec3 ENV_COLOR = vec3(1);

layout(push_constant) uniform _PushConstants { PathtracerConstants constants; };

layout(set=0, binding=0) uniform _Uniforms { Uniforms uniforms; };
layout(set=0, binding=1) uniform _SceneDesc { SceneDesc scene_desc; };
layout(set=0, binding=2) uniform accelerationStructureEXT tlas;
layout(set=0, binding=3, rgba32f) uniform image2D output_image;
layout(set=0, binding=4) uniform sampler2D[] textures;

layout(buffer_reference, scalar) buffer Materials { Material m[]; };

layout(location=0) rayPayloadEXT HitInfo payload;


MaterialHit material_info_at_hit(Material material, vec2 coords) {
  MaterialHit info;
  info.base_color = material.color;
  if (material.color_texture > -1) {
    info.base_color *= texture(textures[material.color_texture], coords).xyz;
  }
  info.emittance = material.emittance;
  if (material.emittance_texture > -1) {
    info.emittance *= texture(textures[material.emittance_texture], coords).xyz;
  }
  info.metallic = material.metallic;
  info.roughness = material.roughness;
  if (material.metallic_roughness_texture > -1) {
    vec2 metallic_roughness = texture(textures[material.metallic_roughness_texture], coords).yz;
    info.metallic *= metallic_roughness.y;
    info.roughness *= metallic_roughness.x;
  }
  return info;
}


void main() {
  Materials materials = Materials(scene_desc.materials_address);

  const uvec2 launch_index = gl_LaunchIDEXT.xy;
  const uvec2 launch_dims = gl_LaunchSizeEXT.xy;
  const uint frame_num = constants.frame;

  Rng rng = rng_init(launch_index, frame_num);

  // anti-aliased pixel
  const vec2 pixel = vec2(launch_index) + rng_vec2(rng);
  const vec2 resolution = vec2(launch_dims);
  const vec2 coords = 2 * (pixel / resolution) - 1;

  const vec4 origin = uniforms.camera.view.inverse * vec4(0, 0, 0, 1);
  const vec4 target = uniforms.camera.proj.inverse * vec4(coords, 1, 1);
  const vec4 direction = uniforms.camera.view.inverse * vec4(normalize(target.xyz), 0);

  Ray ray = Ray(origin, direction);

  vec3 radiance = vec3(0);
  vec3 throughput = vec3(1);
  for (int depth = 0; depth < MAX_BOUNCES; ++depth) {
    traceRayEXT(tlas, RAY_FLAGS, 0xff, 0, 0, 0, ray.origin.xyz, T_MIN, ray.direction.xyz, T_MAX, 0);

    if (!payload.hit) {
      radiance += throughput * ENV_COLOR;
      break;
    }

    const vec3 wo = -ray.direction.xyz;
    vec3 n = payload.normal.xyz;
    if (dot(n, wo) < 0) n = -n;

    const MaterialHit material = material_info_at_hit(materials.m[payload.material], payload.uv);

    radiance += throughput * material.emittance;

    // Don't need to sample BSDF on last bounce
    if (depth == MAX_BOUNCES - 1) break;

    // Russian Roulette
    if (depth > MIN_BOUNCES) {
      float p_rr = min(0.95, luminance(throughput));
      if (p_rr < rng_float(rng)) break;
      else throughput /= p_rr;
    }

    // BSDF evaluation
    bool is_specular = material.metallic == 1 && material.roughness == 0;
    if (!is_specular) {
      float p_spec = specular_probability(material, wo, n);

      if (rng_float(rng) < p_spec) {
        is_specular = true;
        throughput /= p_spec;
      } else {
        throughput /= 1 - p_spec;
      }
    }

    // Importance sample the BSDF
    vec3 wi, weight;
    if (!bsdf_sample(material, is_specular, wo, n, rng_vec2(rng), wi, weight)) break;

    throughput *= weight;

    ray.origin = payload.position;
    ray.direction = vec4(wi, 0);
  }

  vec3 new_color = radiance;
  const ivec2 out_pixel = ivec2(launch_index);
  if (frame_num > 0) {
    const float w = 1 / float(frame_num + 1);
    const vec3 old_color = imageLoad(output_image, out_pixel).xyz;
    new_color = mix(old_color, radiance, w);
  }
  imageStore(output_image, out_pixel, vec4(new_color, 1));
}
