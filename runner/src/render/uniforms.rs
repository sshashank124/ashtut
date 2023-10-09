use ash::vk;

use shared::UniformObjects;

use crate::{context::Context, util::Destroy, wrapper::buffer::Buffer};

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

impl<'a> Destroy<&'a mut Context> for Uniforms {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        for buffer in &mut self.buffers {
            buffer.destroy_with(ctx);
        }
    }
}
