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

struct RasterizerConstants {
  mat4 model_transform;
  uint material_index;
};

struct PathtracerConstants {
  uint frame;
};
