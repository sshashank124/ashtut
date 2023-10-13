use ash::vk;

use super::{context::Context, image::Image, sampler::Sampler, Destroy};

pub struct Texture<const FORMAT: vk::Format> {
    pub image: Image<FORMAT>,
    pub sampler: Sampler,
}

impl<const FORMAT: vk::Format> Texture<FORMAT> {
    pub fn from_image(ctx: &Context, image: Image<FORMAT>) -> Self {
        let sampler = Sampler::create(ctx);
        Self { image, sampler }
    }
}

impl<const FORMAT: vk::Format> Destroy<Context> for Texture<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.sampler.destroy_with(ctx);
        self.image.destroy_with(ctx);
    }
}
