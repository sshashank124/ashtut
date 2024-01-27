#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

struct SceneDesc {
  uint64_t vertices_address;
  uint64_t indices_address;
  uint64_t materials_address;
  uint64_t primitives_address;
};

struct Vertex {
  vec4 position;
  vec4 normal;
  vec4 tex_coords;
};

struct Material {
  vec4 color;
  vec4 emittance;
};

struct PrimitiveInfo {
  uint indices_offset;
  uint vertices_offset;
  uint material;
};
