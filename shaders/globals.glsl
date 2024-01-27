const float PI = 3.1415926535897932384626433832795;

vec3 barycentrics(vec2 uv) {
  return vec3(1 - uv.x - uv.y, uv);
}
