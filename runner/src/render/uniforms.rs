use ash::vk;

use shared::UniformObjects;

use crate::gpu::{buffer::Buffer, context::Context, Destroy};

pub struct Uniforms {
    pub buffers: Vec<Buffer>,
}

impl Uniforms {
    pub fn create(ctx: &mut Context) -> Self {
        let buffer_info = vk::BufferCreateInfo::builder()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(std::mem::size_of::<UniformObjects>() as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffers = (0..ctx.surface.config.image_count)
            .map(|idx| {
                Buffer::create(
                    ctx,
                    &format!("UniformBuffer#{idx}"),
                    *buffer_info,
                    gpu_allocator::MemoryLocation::CpuToGpu,
                )
            })
            .collect();

        Self { buffers }
    }

    pub fn update(&mut self, current_frame: usize, uniforms: &UniformObjects) {
        self.buffers[current_frame].fill_with(uniforms);
    }
}

impl Destroy<Context> for Uniforms {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.buffers.destroy_with(ctx);
    }
}
