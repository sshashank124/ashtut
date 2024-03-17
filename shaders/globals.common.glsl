#ifndef GLOBALS_COMMON_GLSL_
#define GLOBALS_COMMON_GLSL_

const float PI = 3.1415926535897932384626433832795;
const float FLOAT_MAX = 3.402823466e+38f;

vec3 barycentrics(vec2 uv) {
  return vec3(1 - uv.x - uv.y, uv);
}

vec4 quat_frame(vec3 v) {
  if (v.z < -0.99999) return vec4(1, 0, 0, 0);
  return normalize(vec4(v.y, -v.x, 0, 1 + v.z));
}

vec4 quat_invert_rotation(vec4 q) {
  return vec4(-q.xyz, q.w);
}

vec3 quat_rotate(vec4 q, vec3 v) {
  return 2 * dot(q.xyz, v) * q.xyz
    + (q.w * q.w - dot(q.xyz, q.xyz)) * v
    + 2 * q.w * cross(q.xyz, v);
}

vec3 sample_hemisphere(vec2 r) {
  r.y *= 2 * PI;
  const vec2 uv = vec2(cos(r.y), sin(r.y));
  return vec3(uv * sqrt(r.x), sqrt(1 - r.x));
}

#endif
