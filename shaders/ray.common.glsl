#ifndef RAY_COMMON_GLSL_
#define RAY_COMMON_GLSL_

#include "rng.common.glsl"
#include "globals.common.glsl"

const uint MIN_BOUNCES = 3;
const uint MAX_BOUNCES = 8;
const uint RAY_FLAGS = gl_RayFlagsOpaqueEXT;
const float T_MIN = 1e-4;
const float T_MAX = FLOAT_MAX;

struct Ray {
  vec4 origin;
  vec4 direction;
};

struct HitInfo {
  vec4 position;
  vec4 normal;
  vec2 uv;
  uint material;
  bool hit;
};

#endif
