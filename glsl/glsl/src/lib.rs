pub use glsl_derive::GlslStruct;

pub trait Glsl {
    const NAME: &'static str;
}

pub struct GlslField {
    pub name: &'static str,
    pub ty: &'static str,
}

pub trait GlslStruct: Glsl {
    const FIELDS: &'static [GlslField];

    fn glsl_struct_definition() -> String {
        let mut def = String::from("struct ");
        def.push_str(Self::NAME);
        def.push_str(" {\n");
        for field in Self::FIELDS {
            def.push_str("  ");
            def.push_str(field.ty);
            def.push(' ');
            def.push_str(field.name);
            def.push_str(";\n");
        }
        def.push_str("};\n");
        def
    }
}

macro_rules! impl_glsl {
    ($type:ty => $name:expr) => {
        impl Glsl for $type {
            const NAME: &'static str = $name;
        }
    };
}

impl_glsl!(f32 => "float");
impl_glsl!(f64 => "double");
impl_glsl!(i32 => "int");
impl_glsl!(u32 => "uint");
impl_glsl!(i64 => "int64_t");
impl_glsl!(u64 => "uint64_t");

impl_glsl!(glam::Vec2 => "vec2");
impl_glsl!(glam::Vec3 => "vec3");
impl_glsl!(glam::Vec4 => "vec4");

impl_glsl!(glam::Mat4 => "mat4");
