#version 460
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_nonuniform_qualifier : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "inputs.h.glsl"
#include "rasterizer.common.glsl"
#include "scene.h.glsl"

layout(push_constant) uniform _PushConstants { RasterizerConstants constants; };

layout(set=0, binding=1) uniform _SceneDesc { SceneDesc scene_desc; };
layout(set=0, binding=2) uniform sampler2D[] textures;

layout(buffer_reference, scalar) buffer Materials { Material m[]; };

layout(location=0) in _Interface { Interface in_data; };

layout(location=0) out vec4 color;

void main() {
  Materials materials = Materials(scene_desc.materials_address);
  Material material = materials.m[constants.material_index];
  vec3 diffuse = material.color;
  if (material.color_texture > -1) {
    diffuse *= texture(textures[material.color_texture], in_data.tex_coords.xy).xyz;
  }
  vec3 emittance = material.emittance;
  if (material.emittance_texture > -1) {
    emittance *= texture(textures[material.emittance_texture], in_data.tex_coords.xy).xyz;
  }
  color = vec4(diffuse + emittance, 1);
}
