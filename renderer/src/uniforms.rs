use ash::vk;

use shared::inputs;

use super::{buffer::Buffer, context::Context, Destroy};

pub struct Uniforms {
    data: inputs::Uniforms,
    buffer: Buffer,
    dirty: bool,
}

impl Uniforms {
    pub fn create(ctx: &Context, camera: inputs::Camera) -> Self {
        firestorm::profile_method!(create);

        let data = inputs::Uniforms { camera };

        let buffer_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .size(std::mem::size_of_val(&data) as u64);

        let buffer = Buffer::create_with_data(
            ctx,
            "Uniforms".to_owned(),
            buffer_info,
            bytemuck::bytes_of(&data),
        );

        Self {
            data,
            buffer,
            dirty: false,
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        firestorm::profile_method!(update);

        if self.dirty {
            self.buffer.fill_with(ctx, &self.data);
            self.dirty = false;
        }
    }

    pub fn update_camera(&mut self, camera: inputs::Camera) {
        self.data.camera = camera;
        self.dirty = true;
    }

    pub fn buffer_info(&self) -> vk::DescriptorBufferInfo {
        vk::DescriptorBufferInfo::default()
            .buffer(*self.buffer)
            .range(vk::WHOLE_SIZE)
    }
}

impl Destroy<Context> for Uniforms {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.buffer.destroy_with(ctx);
    }
}
