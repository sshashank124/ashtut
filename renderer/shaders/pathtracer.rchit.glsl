#version 460
#extension GL_EXT_ray_tracing : require

#include "raycommon.glsl"

layout(std430, set=0, binding=1) buffer MaterialBuffer { Material materials[]; };
layout(std430, set=1, binding=2) buffer IndicesBuffer { uint indices[]; };
layout(std430, set=1, binding=3) buffer VerticesBuffer { Vertex vertices[]; };
layout(std430, set=1, binding=4) buffer PrimitivesBuffer { PrimitiveInfo primitives[]; };

layout(location=0) rayPayloadInEXT Payload payload;
hitAttributeEXT vec2 hit_uv;

void main() {
  const vec3 bary = barycentrics(hit_uv);
  const PrimitiveInfo primitive = primitives[gl_InstanceCustomIndexEXT];
  const uint ind_offset = primitive.indices_offset + (3 * gl_PrimitiveID);
  const Vertex v0 = vertices[indices[ind_offset] + primitive.vertices_offset],
               v1 = vertices[indices[ind_offset + 1] + primitive.vertices_offset],
               v2 = vertices[indices[ind_offset + 2] + primitive.vertices_offset];

  const vec3 position = v0.position.xyz * bary.x + v1.position.xyz * bary.y + v2.position.xyz * bary.z;
  const vec3 world_position = vec3(gl_ObjectToWorldEXT * vec4(position, 1));

  const vec3 normal = normalize(v0.normal.xyz * bary.x + v1.normal.xyz * bary.y + v2.normal.xyz * bary.z);
  const vec3 world_normal = normalize(transpose(mat3(gl_WorldToObjectEXT)) * normal);

  const vec3 tangent = (abs(world_normal.x) > abs(world_normal.y))
                     ? (vec3(world_normal.z, 0, -world_normal.x) / length(world_normal.xz))
                     : (vec3(0, -world_normal.z, world_normal.y) / length(world_normal.yz));
  const vec3 bitangent = cross(world_normal, tangent);
  const mat3 frame = mat3(tangent, bitangent, world_normal);

  const float r1 = rng_float(payload.rng);
  const float r2 = 2 * PI * rng_float(payload.rng);
  const float r1_sq = sqrt(r1);
  const vec3 dir = vec3(cos(r2) * r1_sq, sin(r2) * r1_sq, sqrt(1 - r1));
  const vec3 out_dir = frame * dir;

  const Material material = materials[primitive.material];

  payload.ray.origin = vec4(world_position, 0);
  payload.ray.direction = vec4(frame * out_dir, 0);
  payload.hit_value = vec3(material.emittance);
  payload.weight = vec3(material.color);
}
