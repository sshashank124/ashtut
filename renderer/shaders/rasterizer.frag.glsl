#version 460

#include "common.glsl"

layout(push_constant) uniform Constants {
  RasterizerConstants constants;
};

layout(std430, binding=1) buffer MaterialBuffer {
  Material materials[];
};

layout(location=0) out vec4 color;

void main() {
  color = materials[constants.material_index].color;
}
