// AUTO-GENERATED: do not edit

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
  vec3 color;
  int color_texture;
  vec3 emittance;
  int emittance_texture;
  float metallic;
  float roughness;
  int metallic_roughness_texture;
};

struct PrimitiveInfo {
  uint indices_offset;
  uint vertices_offset;
  uint material;
};
