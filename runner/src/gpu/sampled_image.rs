use super::{context::Context, image::ColorImage, sampler::Sampler, Destroy};

pub struct SampledImage {
    pub image: ColorImage,
    pub sampler: Sampler,
}

impl SampledImage {
    pub fn from_image(ctx: &Context, image: ColorImage) -> Self {
        let sampler = Sampler::create(ctx);
        Self { image, sampler }
    }
}

impl Destroy<Context> for SampledImage {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.sampler.destroy_with(ctx);
        self.image.destroy_with(ctx);
    }
}
