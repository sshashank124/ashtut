pub mod graphics;
pub mod raytrace;

pub use graphics::Graphics;
pub use raytrace::RayTrace;

use ash::vk;

use crate::gpu::{commands::Commands, context::Context, descriptors::Descriptors, Destroy};

use super::sync_info::SyncInfo;

pub trait Specialization: Destroy<Context> {
    const BIND_POINT: vk::PipelineBindPoint;
    type Output;
    fn start_pass(
        &self,
        _ctx: &Context,
        _commands: &Commands,
        _idx: usize,
        _output_to: &Self::Output,
    ) {
    }
    fn end_pass(&self, _ctx: &Context, _commands: &Commands) {}
}

pub trait Contents<S: Specialization>: Destroy<Context> {
    fn num_command_sets(ctx: &Context) -> u32;
    fn render_area(ctx: &Context) -> vk::Rect2D;

    fn create_descriptors(ctx: &Context) -> Descriptors;
    fn create_specialization(ctx: &Context) -> S;
    fn create_pipeline(
        ctx: &Context,
        spec: &S,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline);

    fn bind_descriptors(&self, ctx: &Context, descriptors: &Descriptors);

    fn pre_pass(&self, _ctx: &Context, _commands: &Commands) {}
    fn record_commands(&self, ctx: &Context, commands: &Commands);
    fn post_pass(&self, _ctx: &Context, _commands: &Commands) {}
}

pub struct Pipeline<S, C> {
    descriptors: Descriptors,
    pub spec: S,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    commands: Vec<Commands>,
    pub contents: C,
}

impl<S: Specialization, C: Contents<S>> Pipeline<S, C> {
    pub fn create(ctx: &mut Context, contents: C) -> Self {
        let descriptors = C::create_descriptors(ctx);
        let spec = C::create_specialization(ctx);
        let (layout, pipeline) = C::create_pipeline(ctx, &spec, descriptors.layout);
        let commands = (0..C::num_command_sets(ctx))
            .map(|_| Commands::create_on_queue(ctx, ctx.queues.graphics()))
            .collect();
        contents.bind_descriptors(ctx, &descriptors);

        Self {
            descriptors,
            spec,
            layout,
            pipeline,
            commands,

            contents,
        }
    }

    pub fn run(&self, ctx: &Context, idx: usize, output_to: &S::Output, sync_info: &SyncInfo) {
        let commands = &self.commands[idx];

        commands.reset(ctx);
        commands.begin_recording(ctx);
        self.contents.pre_pass(ctx, commands);
        self.spec.start_pass(ctx, commands, idx, output_to);
        unsafe {
            // ctx.cmd_begin_render_pass(commands.buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            ctx.cmd_bind_descriptor_sets(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &self.descriptors.sets[crate::util::solo_range(idx)],
                &[],
            );
        }

        self.contents.record_commands(ctx, commands);

        unsafe {
            ctx.cmd_end_render_pass(commands.buffer);
        }
        self.contents.post_pass(ctx, commands);

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(sync_info.wait_on)
            .signal_semaphores(sync_info.signal_to);
        commands.submit(ctx, &submit_info, sync_info.fence);
    }
}

impl<S, C> Destroy<Context> for Pipeline<S, C>
where
    S: Destroy<Context>,
    C: Destroy<Context>,
{
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.contents.destroy_with(ctx);
        self.commands.destroy_with(ctx);
        ctx.destroy_pipeline(self.pipeline, None);
        ctx.destroy_pipeline_layout(self.layout, None);
        self.spec.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
