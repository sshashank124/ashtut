use ash::vk;

use crate::{buffer::Buffer, context::Context, util};

use super::{pass::Pass, pipeline::Pipeline};

pub struct DrawCommands {
    pub buffers: Vec<vk::CommandBuffer>,
}

impl DrawCommands {
    pub fn create(ctx: &Context) -> Self {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(ctx.device.queues.graphics_pool())
            .command_buffer_count(ctx.surface.config.image_count);

        let buffers = unsafe {
            ctx.device
                .allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffers")
        };

        Self { buffers }
    }

    pub fn record(
        &self,
        ctx: &Context,
        pass: &Pass,
        pipeline: &Pipeline,
        vertex_index_buffer: &Buffer,
        descriptor_sets: &[vk::DescriptorSet],
        framebuffers: &[vk::Framebuffer],
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let pass_info_template = vk::RenderPassBeginInfo::builder()
            .render_pass(**pass)
            .render_area(
                vk::Rect2D::builder()
                    .extent(ctx.surface.config.extent)
                    .build(),
            )
            .clear_values(&clear_values)
            .build();

        for (&command_buffer, &descriptor_set, &framebuffer) in
            itertools::izip!(&self.buffers, descriptor_sets, framebuffers)
        {
            let command_buffer_info = vk::CommandBufferBeginInfo::builder();

            unsafe {
                ctx.device
                    .begin_command_buffer(command_buffer, &command_buffer_info)
                    .expect("Failed to begin recording command buffer");
            }

            let mut pass_info = pass_info_template;
            pass_info.framebuffer = framebuffer;

            let viewports = [vk::Viewport::builder()
                .width(ctx.surface.config.extent.width as f32)
                .height(ctx.surface.config.extent.height as f32)
                .max_depth(1.0)
                .build()];

            let scissors = [vk::Rect2D::builder()
                .extent(ctx.surface.config.extent)
                .build()];

            unsafe {
                ctx.device.cmd_begin_render_pass(
                    command_buffer,
                    &pass_info,
                    vk::SubpassContents::INLINE,
                );

                ctx.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    **pipeline,
                );

                ctx.device
                    .cmd_set_viewport_with_count(command_buffer, &viewports);

                ctx.device
                    .cmd_set_scissor_with_count(command_buffer, &scissors);

                let vertex_buffers = [**vertex_index_buffer];
                ctx.device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &[0]);

                ctx.device.cmd_bind_index_buffer(
                    command_buffer,
                    **vertex_index_buffer,
                    super::data::indices_offset(),
                    vk::IndexType::UINT16,
                );

                let descriptor_sets = [descriptor_set];
                ctx.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.layout,
                    0,
                    &descriptor_sets,
                    &[],
                );

                ctx.device.cmd_draw_indexed(
                    command_buffer,
                    super::data::INDICES_DATA.len() as u32,
                    1,
                    0,
                    0,
                    0,
                );

                ctx.device.cmd_end_render_pass(command_buffer);

                ctx.device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to end recording command buffer");
            }
        }
    }

    pub fn run(
        &self,
        ctx: &Context,
        image_index: u32,
        wait_on: &[vk::Semaphore],
        signal_to: &[vk::Semaphore],
        fence: vk::Fence,
    ) {
        let submit_infos = [vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(wait_on)
            .command_buffers(&self.buffers[util::solo_range(image_index as usize)])
            .signal_semaphores(signal_to)
            .build()];

        unsafe {
            ctx.device
                .queue_submit(*ctx.device.queues.graphics, &submit_infos, fence)
                .expect("Failed to submit commands through the `graphics` queue");
        }
    }
}
