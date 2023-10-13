pub mod offscreen;
pub mod tonemap;

pub use offscreen::Offscreen;
pub use tonemap::Tonemap;

use crate::gpu::{
    commands::Commands, context::Context, descriptors::Descriptors, pipeline::Pipeline,
    render_pass::RenderPass, Destroy,
};

pub struct Pass<T> {
    pub descriptors: Descriptors,
    pub render_pass: RenderPass,
    pub pipeline: Pipeline,
    pub commands: Vec<Commands>,

    pub data: T,
}

impl<Data> Pass<Data> {
    pub fn create(ctx: &mut Context, setup_scope: &mut Scope) -> Self {}
}

impl<Data> Destroy<Context> for Pass<Data> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.commands.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        self.render_pass.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
