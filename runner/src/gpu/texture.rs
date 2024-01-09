use super::{context::Context, image, sampler::Sampler, Destroy};

pub struct Texture<const FORMAT: image::Format> {
    pub image: image::Image<FORMAT>,
    pub sampler: Sampler,
}

impl<const FORMAT: image::Format> Texture<FORMAT> {
    pub fn from_image(ctx: &Context, image: image::Image<FORMAT>) -> Self {
        let sampler = Sampler::create(ctx);
        Self { image, sampler }
    }
}

impl<const FORMAT: image::Format> Destroy<Context> for Texture<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.sampler.destroy_with(ctx);
        self.image.destroy_with(ctx);
    }
}
