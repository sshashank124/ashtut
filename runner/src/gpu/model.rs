use super::{
    context::Context,
    image::{Format, Image},
    scope::OneshotScope,
    texture::Texture,
    Destroy,
};

pub struct Model {
    pub texture_image: image::RgbaImage,
    pub texture: Texture<{ Format::Color }>,
}

impl Model {
    fn from_data(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        texture_image: image::RgbaImage,
    ) -> Self {
        let texture = Self::init_texture(ctx, scope, &texture_image);

        Self {
            texture_image,
            texture,
        }
    }

    pub fn from_files(ctx: &mut Context, scope: &mut OneshotScope, texture_file: &str) -> Self {
        let texture_image = crate::util::load_image_from_file(texture_file);
        Self::from_data(ctx, scope, texture_image)
    }

    fn init_texture(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        texture_image: &image::RgbaImage,
    ) -> Texture<{ Format::Color }> {
        let image = Image::create_from_image(ctx, scope, "Texture", texture_image);
        Texture::from_image(ctx, image)
    }

    #[allow(dead_code)]
    pub fn demo_viking_room(ctx: &mut Context, init_scope: &mut OneshotScope) -> Self {
        Self::from_files(ctx, init_scope, "assets/textures/viking_room.png")
    }

    #[allow(dead_code)]
    pub fn demo_2_planes(ctx: &mut Context, init_scope: &mut OneshotScope) -> Self {
        Self::from_data(
            ctx,
            init_scope,
            crate::util::load_image_from_file("assets/textures/statue.jpg"),
        )
    }
}

impl Destroy<Context> for Model {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.texture.destroy_with(ctx);
    }
}
