use super::{
    context::Context,
    image::{Color, HdrColor, Image},
    sampler::Sampler,
    Destroy,
};

pub type SampledHdrImage = SampledImage<HdrColor>;
pub type SampledColorImage = SampledImage<Color>;

pub struct SampledImage<T> {
    pub image: Image<T>,
    pub sampler: Sampler,
}

impl<T> SampledImage<T> {
    pub fn from_image(ctx: &Context, image: Image<T>) -> Self {
        let sampler = Sampler::create(ctx);
        Self { image, sampler }
    }
}

impl<T> Destroy<Context> for SampledImage<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.sampler.destroy_with(ctx);
        self.image.destroy_with(ctx);
    }
}
