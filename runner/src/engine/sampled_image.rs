use crate::{context::Context, util::Destroy};

use super::{image::Image, sampler::Sampler};

pub struct SampledImage {
    pub image: Image,
    pub sampler: Sampler,
}

impl SampledImage {
    pub fn create_with_sampler(ctx: &Context, image: Image) -> Self {
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
