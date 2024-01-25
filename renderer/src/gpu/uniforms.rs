use ash::vk;

use super::{buffer::Buffer, context::Context, Destroy};
use crate::data::shader;

pub struct Uniforms {
    pub buffer: Buffer,
}

impl Uniforms {
    pub fn create(ctx: &mut Context) -> Self {
        let buffer_info = vk::BufferCreateInfo::builder()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(std::mem::size_of::<shader::Uniforms>() as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = Buffer::create(
            ctx,
            "UniformBuffer",
            *buffer_info,
            gpu_allocator::MemoryLocation::CpuToGpu,
        );

        Self { buffer }
    }

    pub fn update(&mut self, uniforms: &shader::Uniforms) {
        self.buffer.fill_with(uniforms);
    }
}

impl Destroy<Context> for Uniforms {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.buffer.destroy_with(ctx);
    }
}
