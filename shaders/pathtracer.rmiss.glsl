#version 460
#extension GL_EXT_ray_tracing : require

#include "ray.common.glsl"

layout(location=0) rayPayloadInEXT Payload payload;

void main() {
  payload.hit_value = ENV_COLOR;
  payload.depth = MAX_RECURSE_DEPTH;
}
