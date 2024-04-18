use ash::vk;

use shared::inputs;

use crate::memory;

use super::{buffer::Buffer, context::Context, Destroy};

pub struct Uniforms {
    pub buffer: Buffer,
}

impl Uniforms {
    pub fn create(ctx: &Context) -> Self {
        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(std::mem::size_of::<inputs::Uniforms>() as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = Buffer::create(
            ctx,
            "Uniforms".to_owned(),
            buffer_info,
            &memory::purpose::staging(),
        );

        Self { buffer }
    }

    pub fn update(&self, ctx: &Context, uniforms: &inputs::Uniforms) {
        self.buffer.fill_with(ctx, uniforms);
    }
}

impl Destroy<Context> for Uniforms {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        self.buffer.destroy_with(ctx);
    }
}
