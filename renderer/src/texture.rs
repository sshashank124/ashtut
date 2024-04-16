use ash::vk;

use super::{context::Context, image, sampler::Sampler, Destroy};

pub struct Texture<const FORMAT: image::Format> {
    pub view: vk::ImageView,
    pub sampler: Sampler,
}

impl<const FORMAT: image::Format> Texture<FORMAT> {
    pub fn for_image(
        ctx: &Context,
        name: impl AsRef<str>,
        image: &image::Image<{ FORMAT }>,
    ) -> Self {
        let sampler = Sampler::create(ctx, name);
        Self {
            view: image.view,
            sampler,
        }
    }
}

impl<const FORMAT: image::Format> Destroy<Context> for Texture<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        self.sampler.destroy_with(ctx);
    }
}
