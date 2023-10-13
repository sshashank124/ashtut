pub mod offscreen;
pub mod tonemap;

pub use offscreen::Offscreen;
pub use tonemap::Tonemap;

use crate::gpu::{
    context::Context, descriptors::Descriptors, pipeline::Pipeline, render_pass::RenderPass,
    Destroy,
};

pub struct Pass {
    pub descriptors: Descriptors,
    pub render_pass: RenderPass,
    pub pipeline: Pipeline,
}

impl Destroy<Context> for Pass {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.pipeline.destroy_with(ctx);
        self.render_pass.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
