#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

const float PI = 3.1415926535897932384626433832795;

struct Transform {
  mat4 forward;
  mat4 inverse;
};

struct Camera {
  Transform view;
  Transform proj;
};

struct Uniforms {
  Camera camera;
};

struct Vertex {
  vec4 position;
  vec4 normal;
  vec4 tex_coords;
};

struct PrimitiveInfo {
  uint indices_offset;
  uint vertices_offset;
  uint material;
};

struct SceneDesc {
  uint64_t vertices_address;
  uint64_t indices_address;
  uint64_t primitives_address;
  uint64_t materials_address;
};

struct Material {
  vec4 color;
  vec4 emittance;
};

struct RasterizerConstants {
  mat4 model_transform;
  uint material_index;
};

struct PathtracerConstants {
  uint frame;
};

struct Rng {
  uint state;
};

vec3 barycentrics(vec2 uv) {
  return vec3(1 - uv.x - uv.y, uv);
}

uint rng_uint(inout Rng rng) {
  rng.state ^= rng.state << 13;
  rng.state ^= rng.state >> 17;
  rng.state ^= rng.state << 5;
  return rng.state;
}

float rng_float(inout Rng rng) {
  return uintBitsToFloat((rng_uint(rng) >> 9) | 0x3f800000) - 1;
}
