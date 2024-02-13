#include "rng.common.glsl"

const uint MAX_RECURSE_DEPTH = 5;
const uint RAY_FLAGS = gl_RayFlagsOpaqueEXT;
const float T_MIN = 1e-3;
const float T_MAX = 1e+5;
const vec3 ENV_COLOR = vec3(1.0f);

struct Ray {
  vec4 origin;
  vec4 direction;
};

struct Payload {
  Ray ray;
  vec3 hit_value;
  Rng rng;
  vec3 weight;
  uint depth;
};
