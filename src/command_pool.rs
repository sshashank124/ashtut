use ash::vk;

use crate::{
    device::Device, graphics_pipeline::GraphicsPipeline, physical_device::PhysicalDevice,
    util::Destroy,
};

pub struct CommandPool {
    pool: vk::CommandPool,
    pub buffers: Vec<vk::CommandBuffer>,
}

impl CommandPool {
    pub fn create(
        physical_device: &PhysicalDevice,
        device: &Device,
        graphics_pipeline: &GraphicsPipeline,
    ) -> Self {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(physical_device.indices.graphics())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let pool = unsafe {
            device
                .create_command_pool(&create_info, None)
                .expect("Failed to create command pool")
        };

        let buffers = Self::create_commandbuffers(device, graphics_pipeline, pool);

        Self { pool, buffers }
    }

    fn create_commandbuffers(
        device: &Device,
        graphics_pipeline: &GraphicsPipeline,
        command_pool: vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(graphics_pipeline.framebuffers.len() as u32);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffers")
        };

        for (&framebuffer, &command_buffer) in
            graphics_pipeline.framebuffers.iter().zip(&command_buffers)
        {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

            unsafe {
                device
                    .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                    .expect("Failed to begin recording command buffer");
            }

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(*graphics_pipeline.render_pass)
                .framebuffer(framebuffer)
                .render_area(
                    vk::Rect2D::builder()
                        .extent(graphics_pipeline.swapchain.extent)
                        .build(),
                )
                .clear_values(&clear_values);

            unsafe {
                device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    graphics_pipeline.pipeline,
                );
                device.cmd_draw(command_buffer, 3, 1, 0, 0);
                device.cmd_end_render_pass(command_buffer);
                device
                    .end_command_buffer(command_buffer)
                    .expect("Failed to end recording command buffer");
            }
        }

        command_buffers
    }
}

impl<'a> Destroy<&'a Device> for CommandPool {
    fn destroy_with(&self, device: &'a Device) {
        unsafe {
            device.destroy_command_pool(self.pool, None);
        }
    }
}
