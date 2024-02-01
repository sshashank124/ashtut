#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require


#include "inputs.h.glsl"
#include "scene.h.glsl"
#include "ray.common.glsl"

layout(push_constant) uniform _PushConstants { PathtracerConstants constants; };

layout(set=0, binding=0) uniform _Uniforms { Uniforms uniforms; };
layout(set=1, binding=0) uniform accelerationStructureEXT tlas;
layout(set=1, binding=1, rgba32f) uniform image2D output_image;

layout(location=0) rayPayloadEXT Payload payload;

void main() {
  const vec2 uv = (vec2(gl_LaunchIDEXT.xy) + vec2(0.5)) / vec2(gl_LaunchSizeEXT.xy);
  const vec4 origin = uniforms.camera.view.inverse * vec4(0, 0, 0, 1);
  const vec4 target = uniforms.camera.proj.inverse * vec4(2 * uv - 1, 1, 1);
  const vec4 direction = uniforms.camera.view.inverse * vec4(normalize(target.xyz), 0);

  const uint rng_seed = gl_LaunchSizeEXT.x * (gl_LaunchSizeEXT.y * constants.frame + gl_LaunchIDEXT.y)
                        + gl_LaunchIDEXT.x;

  payload = Payload(Ray(origin, direction), vec3(0), Rng(rng_seed), vec3(0), 0);

  vec3 total = vec3(0);
  vec3 weight = vec3(1);

  while (payload.depth < MAX_RECURSE_DEPTH) {
    traceRayEXT(tlas, RAY_FLAGS, 0xff, 0, 0, 0,
                payload.ray.origin.xyz, T_MIN, payload.ray.direction.xyz, T_MAX, 0);
    total += weight * payload.hit_value;
    weight *= payload.weight;
    ++payload.depth;
  }

  vec3 new_color = total;
  if (constants.frame > 0) {
    const float w = 1 / float(constants.frame + 1);
    const vec3 old_color = imageLoad(output_image, ivec2(gl_LaunchIDEXT.xy)).xyz;
    new_color = mix(old_color, total, w);
  }

  imageStore(output_image, ivec2(gl_LaunchIDEXT.xy), vec4(new_color, 1));

}
