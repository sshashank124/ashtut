#ifndef BSDF_COMMON_GLSL_
#define BSDF_COMMON_GLSL_

#include "globals.common.glsl"
#include "scene.h.glsl"

const float MIN_DIELECTRICS_F0 = 0.04;

struct MaterialHit {
  vec3 base_color;
  float metallic;
  vec3 emittance;
  float roughness;
};

float clamp_unit_nonzero(float value) {
  return clamp(value, 0.00001, 1);
}

float clamp_unit(float value) {
  return clamp(value, 0, 1);
}

float clamp_pos(float value) {
  return max(0, value);
}

float luminance(vec3 color) {
  return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

vec3 base_color_to_specular_f0(vec3 base_color, float metallic) {
  return mix(vec3(MIN_DIELECTRICS_F0), base_color, metallic);
}

vec3 base_color_to_diffuse_reflectance(vec3 base_color, float metallic) {
  return base_color * (1 - metallic);
}

float shadowed_f90(vec3 f0) {
  return min(1, luminance(f0) / MIN_DIELECTRICS_F0);
}

// Schlick's approximation
vec3 eval_fresnel(vec3 f0, float n_dot_s) {
  return f0 + (shadowed_f90(f0) - f0) * pow(1 - n_dot_s, 5);
}

// GGX
float smith_g1(float alpha_sq, float n_dot_s_sq) {
  return 2 / (sqrt((alpha_sq * (1 - n_dot_s_sq) + n_dot_s_sq) / n_dot_s_sq) + 1);
}

// GGX VNDF height correlated
float specular_sample_weight(float alpha_sq, float n_dot_l_sq, float n_dot_wo_sq) {
  const float g1wo = smith_g1(alpha_sq, n_dot_wo_sq);
  const float g1l = smith_g1(alpha_sq, n_dot_l_sq);
  return g1l / (g1wo + g1l - g1wo * g1l);
}

// GGX VNDF
vec3 sample_specular_half_vector(vec3 wo, float alpha, vec2 uv) {
  const vec3 vh = normalize(vec3(alpha * wo.xy, wo.z));

  const float len_sq = dot(wo.xy, wo.xy);
  const vec3 tv1 = len_sq > 0 ? vec3(-wo.y, wo.x, 0) / sqrt(len_sq) : vec3(1, 0, 0);
  const vec3 tv2 = cross(wo, tv1);

  const float r = sqrt(uv.x);
  const float phi = 2 * PI * uv.y;
  const float t1 = r * cos(phi);
  float t2 = r * sin(phi);
  const float s = 0.5 * (1 + wo.z);
  t2 = mix(sqrt(1 - t1 * t1), t2, s);

  const vec3 nh = t1 * tv1 + t2 * tv2 + sqrt(max(0, 1 - t1 * t1 - t2 * t2)) * vh;

  return normalize(vec3(alpha * nh.xy, max(0, nh.z)));
}

vec3 sample_specular_microfacet(vec3 wo, float alpha, vec3 specular_f0, vec2 r, out vec3 weight) {
  vec3 h;
  if (alpha == 0) h = vec3(0, 0, 1);
  else h = sample_specular_half_vector(wo, alpha, r);

  const vec3 l = reflect(-wo, h);

  const vec3 n = vec3(0, 0, 1);
  const float h_dot_l = clamp_unit_nonzero(dot(h, l));
  const float n_dot_l = clamp_unit_nonzero(dot(n, l));
  const float n_dot_wo = clamp_unit_nonzero(dot(n, wo));

  const vec3 F = eval_fresnel(specular_f0, h_dot_l);

  weight = F * specular_sample_weight(alpha * alpha, n_dot_l * n_dot_l, n_dot_wo * n_dot_wo);

  return l;
}

float specular_probability(MaterialHit material, vec3 wo, vec3 n) {
  float specular_f0 = luminance(base_color_to_specular_f0(material.base_color, material.metallic));
  float diffuse_reflectance = luminance(base_color_to_diffuse_reflectance(material.base_color, material.metallic));

  float specular = clamp_unit(luminance(eval_fresnel(vec3(specular_f0), clamp_pos(dot(wo, n)))));
  float diffuse = diffuse_reflectance * (1 - specular);

  float p = specular / max(0.0001, specular + diffuse);
  return clamp(p, 0.1, 0.9);
}

bool bsdf_sample(MaterialHit material, bool is_specular, vec3 wo, vec3 n, vec2 r,
                 out vec3 wi, out vec3 weight) {
  if (dot(n, wo) <= 0) false;

  const vec4 frame = quat_frame(n);
  wo = quat_rotate(frame, wo);

  const float alpha = material.roughness * material.roughness;
  const vec3 specular_f0 = base_color_to_specular_f0(material.base_color, material.metallic);

  if (is_specular) {
    wi = sample_specular_microfacet(wo, alpha, specular_f0, r, weight);
  } else {
    wi = sample_hemisphere(r);

    const vec3 h = sample_specular_half_vector(wo, alpha, r);
    const float wo_dot_h = clamp_unit_nonzero(dot(wo, h));
    weight = base_color_to_diffuse_reflectance(material.base_color, material.metallic)
              * (vec3(1) - eval_fresnel(specular_f0, wo_dot_h));
  }

  if (luminance(weight) == 0) return false;

  wi = normalize(quat_rotate(quat_invert_rotation(frame), wi));

  return dot(n, wi) > 0;
}

#endif
