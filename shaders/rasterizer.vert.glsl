#version 460

#include "inputs.h.glsl"
#include "rasterizer.common.glsl"

layout(push_constant) uniform _PushConstants { RasterizerConstants constants; };

layout(binding=0) uniform _Uniforms { Uniforms uniforms; };

layout(location=0) in vec4 position;
layout(location=1) in vec2 tex_coord;

layout(location=0) out _Interface { Interface out_data; };

void main() {
  gl_Position = uniforms.camera.proj.forward
              * uniforms.camera.view.forward
              * constants.model_transform
              * position;
  out_data.tex_coord = tex_coord;
}
