pub mod offscreen;
pub mod tonemap;

use ash::vk;
pub use offscreen::Offscreen;
pub use tonemap::Tonemap;

use crate::gpu::{
    commands::Commands, context::Context, descriptors::Descriptors, framebuffers,
    pipeline::Pipeline, render_pass::RenderPass, Destroy,
};

use super::sync_state::SyncRequirements;

pub trait Contents: Destroy<Context> {
    fn num_command_sets(ctx: &Context) -> u32;
    fn render_area(ctx: &Context) -> vk::Rect2D;

    fn create_descriptors(ctx: &Context) -> Descriptors;
    fn create_render_pass(ctx: &Context) -> RenderPass;
    fn create_pipeline(
        ctx: &Context,
        render_pass: &RenderPass,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Pipeline;

    fn bind_descriptors(&self, ctx: &Context, descriptors: &Descriptors);

    fn begin_pass(&self, _: &Context, _: &Commands) {}
    fn record_commands(&self, ctx: &Context, commands: &Commands);
    fn end_pass(&self, _: &Context, _: &Commands) {}
}

pub struct Pass<T> {
    descriptors: Descriptors,
    pub render_pass: RenderPass,
    pipeline: Pipeline,
    commands: Vec<Commands>,
    pub contents: T,
}

impl<T: Contents> Pass<T> {
    pub fn create(ctx: &mut Context, contents: T) -> Self {
        let descriptors = T::create_descriptors(ctx);
        let render_pass = T::create_render_pass(ctx);
        let pipeline = T::create_pipeline(ctx, &render_pass, descriptors.layout);
        let commands = (0..T::num_command_sets(ctx))
            .map(|_| Commands::create_on_queue(ctx, ctx.queues.graphics()))
            .collect();
        contents.bind_descriptors(ctx, &descriptors);

        Self {
            descriptors,
            render_pass,
            pipeline,
            commands,

            contents,
        }
    }

    pub fn draw<const TARGET_FORMAT: vk::Format>(
        &self,
        ctx: &Context,
        command_set: usize,
        render_target: &framebuffers::Framebuffers<{ TARGET_FORMAT }>,
        sync_requirements: &SyncRequirements,
    ) {
        let commands = &self.commands[command_set];

        commands.reset(ctx);

        let pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(*self.render_pass)
            .render_area(T::render_area(ctx))
            .framebuffer(render_target.framebuffers[command_set])
            .clear_values(framebuffers::CLEAR_VALUES)
            .build();

        commands.begin_recording(ctx);
        self.contents.begin_pass(ctx, commands);
        unsafe {
            ctx.cmd_begin_render_pass(commands.buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pipeline,
            );

            ctx.cmd_bind_descriptor_sets(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &self.descriptors.sets[crate::util::solo_range(command_set)],
                &[],
            );
        }

        self.contents.record_commands(ctx, commands);

        unsafe {
            ctx.cmd_end_render_pass(commands.buffer);
        }
        self.contents.end_pass(ctx, commands);

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(sync_requirements.wait_on)
            .signal_semaphores(sync_requirements.signal_to);
        commands.submit(ctx, &submit_info, sync_requirements.fence);
    }
}

impl<T: Contents> Destroy<Context> for Pass<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.contents.destroy_with(ctx);
        self.commands.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        self.render_pass.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
