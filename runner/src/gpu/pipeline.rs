use std::ops::Deref;

use ash::vk;

use crate::gpu::{commands::Commands, context::Context, descriptors::Descriptors, Destroy};

use super::{context::queue::Queue, sync_info::SyncInfo};

pub struct Pipeline {
    descriptors: Descriptors,
    pub layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    commands: Vec<Commands>,
}

impl Pipeline {
    pub fn new(
        ctx: &Context,
        descriptors: Descriptors,
        layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        queue: &Queue,
        count: usize,
    ) -> Self {
        let commands = (0..count)
            .map(|_| Commands::create_on_queue(ctx, queue))
            .collect();

        Self {
            descriptors,
            layout,
            pipeline,
            commands,
        }
    }

    pub fn begin_pipeline(&self, ctx: &Context, idx: usize) -> &Commands {
        self.commands[idx].reset(ctx);
        self.commands[idx].begin_recording(ctx);
        &self.commands[idx]
    }

    pub fn submit_pipeline(&self, ctx: &Context, idx: usize, sync_info: &SyncInfo) {
        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(sync_info.wait_on)
            .signal_semaphores(sync_info.signal_to);

        self.commands[idx].submit(ctx, &submit_info, sync_info.fence);
    }

    pub fn descriptor_set(&self, idx: usize) -> &[vk::DescriptorSet] {
        &self.descriptors.sets[crate::util::solo_range(idx)]
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.commands.destroy_with(ctx);
        ctx.destroy_pipeline(self.pipeline, None);
        ctx.destroy_pipeline_layout(self.layout, None);
        self.descriptors.destroy_with(ctx);
    }
}

impl Deref for Pipeline {
    type Target = vk::Pipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
