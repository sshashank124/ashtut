use ash::vk;

use crate::gpu::{
    commands::Commands,
    context::Context,
    framebuffers::{self, Framebuffers},
    Destroy,
};

use super::{Pipeline, Specialization};

pub struct GraphicsPipeline {
    pipeline: Pipeline,
    pub render_pass: vk::RenderPass,
}

impl<const FMT: vk::Format> Graphics<{ FMT }> {
    pub fn create(ctx: &Context, render_pass_info: &vk::RenderPassCreateInfo) -> Self {
        let render_pass = unsafe {
            ctx.create_render_pass(render_pass_info, None)
                .expect("Failed to create render pass")
        };

        Self { render_pass }
    }
}

impl<const FMT: vk::Format> Specialization for Graphics<{ FMT }> {
    const BIND_POINT: vk::PipelineBindPoint = vk::PipelineBindPoint::GRAPHICS;

    type Output = Framebuffers<{ FMT }>;

    fn start_pass(&self, ctx: &Context, commands: &Commands, idx: usize, output_to: &Self::Output) {
        let pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .render_area(C::render_area(ctx))
            .framebuffer(output_to.framebuffers[idx])
            .clear_values(framebuffers::CLEAR_VALUES)
            .build();
    }
}

impl<const FMT: vk::Format> Destroy<Context> for Graphics<{ FMT }> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_render_pass(self.render_pass, None);
    }
}
