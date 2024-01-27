#version 460

const float INV_GAMMA = 1.0f / 2.2f;

layout(binding=0) uniform sampler2D tex;

layout(location=0) in vec2 uv;

layout(location=0) out vec4 color;

void main() {
  color = pow(texture(tex, uv), vec4(INV_GAMMA));
}
