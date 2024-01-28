#version 460

#include "inputs.h.glsl"

layout(push_constant) uniform _PushConstants { RasterizerConstants constants; };

layout(binding=0) uniform _Uniforms { Uniforms uniforms; };

layout(location=0) in vec4 position;

void main() {
  gl_Position = uniforms.camera.proj.forward
              * uniforms.camera.view.forward
              * constants.model_transform
              * position;
}
