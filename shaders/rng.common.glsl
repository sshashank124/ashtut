#ifndef RNG_COMMON_GLSL_
#define RNG_COMMON_GLSL_

// pcg4d
struct Rng {
  uvec4 state;
};

Rng rng_init(uvec2 pixel, uint frame) {
  return Rng(uvec4(pixel, frame, 0));
}

uvec4 rng_uint4(inout Rng rng) {
  uvec4 v = rng.state * 1664525u + 1013904223u;

  v.x += v.y * v.w; 
  v.y += v.z * v.x; 
  v.z += v.x * v.y; 
  v.w += v.y * v.z;

  v ^= v >> 16u;

  v.x += v.y * v.w; 
  v.y += v.z * v.x; 
  v.z += v.x * v.y; 
  v.w += v.y * v.z;

  rng.state = v;
  return v;
}

float uintToFloat(uint x) {
  return uintBitsToFloat((x >> 9) | 0x3f800000) - 1.0f;
}

float rng_float(inout Rng rng) {
  return uintToFloat(rng_uint4(rng).x);
}

vec2 rng_vec2(inout Rng rng) {
  uvec4 v = rng_uint4(rng);
  return vec2(uintToFloat(v.x), uintToFloat(v.y));
}

#endif
