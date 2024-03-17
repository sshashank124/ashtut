#version 460
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "ray.common.glsl"
#include "scene.h.glsl"

layout(set=0, binding=1) uniform _SceneDesc { SceneDesc scene_desc; };

layout(buffer_reference, scalar) buffer Vertices { Vertex v[]; };
layout(buffer_reference, scalar) buffer Indices { uvec3 i[]; };
layout(buffer_reference, scalar) buffer Primitives { PrimitiveInfo p[]; };

layout(location=0) rayPayloadInEXT HitInfo payload;
hitAttributeEXT vec2 hit_uv;


void main() {
  Vertices vertices = Vertices(scene_desc.vertices_address);
  Indices indices = Indices(scene_desc.indices_address);
  Primitives primitives = Primitives(scene_desc.primitives_address);

  const vec3 bary = barycentrics(hit_uv);

  const PrimitiveInfo primitive = primitives.p[gl_InstanceCustomIndexEXT];
  const uvec3 idx = indices.i[primitive.indices_offset / 3 + gl_PrimitiveID] + primitive.vertices_offset;
  const Vertex v0 = vertices.v[idx.x], v1 = vertices.v[idx.y], v2 = vertices.v[idx.z];

  const vec3 position = v0.position.xyz * bary.x + v1.position.xyz * bary.y + v2.position.xyz * bary.z;
  payload.position = vec4(gl_ObjectToWorldEXT * vec4(position, 1), 0);
  const vec3 normal = normalize(v0.normal.xyz * bary.x + v1.normal.xyz * bary.y + v2.normal.xyz * bary.z);
  payload.normal = vec4(normalize(gl_ObjectToWorldEXT * vec4(normal, 0)), 0);
  payload.uv = v0.tex_coords.xy * bary.x + v1.tex_coords.xy * bary.y + v2.tex_coords.xy * bary.z;
  payload.material = primitive.material;
  payload.hit = true;
}
