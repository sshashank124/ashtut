#version 460
#extension GL_EXT_ray_tracing : require

#include "ray.common.glsl"

layout(location=0) rayPayloadInEXT HitInfo payload;

void main() {
  payload.hit = false;
}
