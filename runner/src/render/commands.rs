use ash::vk;

use crate::{
    context::Context,
    util::{self, Destroy},
};

pub struct Commands {
    pool: vk::CommandPool,
    pub buffers: Vec<vk::CommandBuffer>,
}

impl Commands {
    pub fn create(ctx: &Context) -> Self {
        let pool = ctx.device.create_command_pool();

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool)
            .command_buffer_count(ctx.surface.config.image_count);

        let buffers = unsafe {
            ctx.device
                .allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffers")
        };

        Self { pool, buffers }
    }

    pub fn record(
        &self,
        ctx: &Context,
        render_pass: vk::RenderPass,
        pipeline: vk::Pipeline,
        vertex_buffer: vk::Buffer,
        framebuffers: &[vk::Framebuffer],
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_info_template = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .render_area(
                vk::Rect2D::builder()
                    .extent(ctx.surface.config.extent)
                    .build(),
            )
            .clear_values(&clear_values)
            .build();

        let vertex_buffers = [vertex_buffer];

        for (&command_buffer, &framebuffer) in self.buffers.iter().zip(framebuffers) {
            let command_buffer_info = vk::CommandBufferBeginInfo::builder();

            unsafe {
                ctx.device
                    .begin_command_buffer(command_buffer, &command_buffer_info)
                    .expect("Failed to begin recording command buffer");
            }

            let mut render_pass_info = render_pass_info_template;
            render_pass_info.framebuffer = framebuffer;

            unsafe {
                ctx.device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_info,
                    vk::SubpassContents::INLINE,
                );

                ctx.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );

                let viewports = [vk::Viewport::builder()
                    .width(ctx.surface.config.extent.width as f32)
                    .height(ctx.surface.config.extent.height as f32)
                    .max_depth(1.0)
                    .build()];
                ctx.device
                    .cmd_set_viewport_with_count(command_buffer, &viewports);

                let scissors = [vk::Rect2D::builder()
                    .extent(ctx.surface.config.extent)
                    .build()];
                ctx.device
                    .cmd_set_scissor_with_count(command_buffer, &scissors);

                ctx.device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &[0]);
                ctx.device.cmd_draw(
                    command_buffer,
                    super::conf::VERTICES_DATA.len() as u32,
                    1,
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
                .queue_submit(ctx.device.queue.graphics, &submit_infos, fence)
                .expect("Failed to submit commands through the `graphics` queue");
        }
    }

    pub fn reset(&mut self, ctx: &mut Context) {
        unsafe {
            ctx.device
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .expect("Failed to reset command pool");
        }
    }
}

impl<'a> Destroy<&'a mut Context> for Commands {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        ctx.device.destroy_command_pool(self.pool, None);
    }
}
