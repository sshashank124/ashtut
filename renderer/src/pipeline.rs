use std::{ops::Deref, slice};

use ash::vk;

use crate::{
    commands::Commands, context::queue::Queue, context::Context, sync_info::SyncInfo, Destroy,
};

pub struct Pipeline<const NUM_SETS: usize> {
    pub descriptor_sets: Vec<[vk::DescriptorSet; NUM_SETS]>,
    pub layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    commands: Vec<Commands>,
}

impl<const NUM_SETS: usize> Pipeline<{ NUM_SETS }> {
    pub fn new(
        ctx: &Context,
        name: impl AsRef<str>,
        descriptor_sets: impl IntoIterator<Item = [vk::DescriptorSet; NUM_SETS]>,
        layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        queue: &Queue,
        count: usize,
    ) -> Self {
        let descriptor_sets = descriptor_sets.into_iter().collect::<Vec<_>>();

        let name = String::from(name.as_ref()) + " - Pipeline";
        for (i, sets) in descriptor_sets.iter().enumerate() {
            for (j, set) in sets.iter().enumerate() {
                ctx.set_debug_name(*set, format!("{name} - Descriptor Set - #{i}:#{j}"));
            }
        }
        ctx.set_debug_name(layout, name.clone() + " - Layout");
        ctx.set_debug_name(pipeline, &name);

        let commands = (0..count)
            .map(|i| Commands::create_on_queue(ctx, format!("{name} - #{i}"), queue))
            .collect();

        Self {
            descriptor_sets,
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
        let mut submit_info = vk::SubmitInfo::default();
        if let Some(wait_on) = sync_info.wait_on.as_ref() {
            submit_info.wait_semaphore_count = 1;
            submit_info.p_wait_dst_stage_mask =
                [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT].as_ptr();
            submit_info.p_wait_semaphores = slice::from_ref(wait_on).as_ptr();
        }
        if let Some(signal_to) = sync_info.signal_to.as_ref() {
            submit_info.signal_semaphore_count = 1;
            submit_info.p_signal_semaphores = slice::from_ref(signal_to).as_ptr();
        }

        self.commands[idx].submit(ctx, &submit_info, sync_info.fence);
    }
}

impl<const NUM_SETS: usize> Destroy<Context> for Pipeline<{ NUM_SETS }> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.commands.destroy_with(ctx);
        ctx.destroy_pipeline(self.pipeline, None);
        ctx.destroy_pipeline_layout(self.layout, None);
    }
}

impl<const NUM_SETS: usize> Deref for Pipeline<{ NUM_SETS }> {
    type Target = vk::Pipeline;
    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
