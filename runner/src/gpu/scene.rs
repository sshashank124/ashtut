use ash::vk;
use shared::bytemuck;

use super::{buffer::Buffer, context::Context, scope::OneshotScope, Destroy};
use crate::data::{gltf_scene, Instance, Primitive};

pub struct Scene {
    pub geometry: VertexIndexBuffer,
    pub primitives: Vec<Primitive>,
    pub instances: Vec<Instance>,
}

pub struct VertexIndexBuffer {
    pub buffer: Buffer,
    pub indices_offset: u64,
    pub indices_count: u32,
}

impl Scene {
    pub fn load_gltf(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &gltf_scene::GltfScene,
    ) -> Self {
        let geometry = VertexIndexBuffer::create(ctx, scope, &scene.data);

        Self {
            geometry,
            primitives: scene.primitives.clone(),
            instances: scene.instances.clone(),
        }
    }
}

impl VertexIndexBuffer {
    fn create(ctx: &mut Context, scope: &mut OneshotScope, scene_data: &gltf_scene::Data) -> Self {
        let indices_offset = std::mem::size_of_val(scene_data.vertices.as_slice()) as _;
        let indices_count = scene_data.indices.len() as _;

        let data_sources = &[
            bytemuck::cast_slice(&scene_data.vertices),
            bytemuck::cast_slice(&scene_data.indices),
        ];
        let create_info = vk::BufferCreateInfo::builder().usage(
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        );

        let buffer = Buffer::create_with_staged_data(
            ctx,
            scope,
            "Vertex + Index Buffer",
            *create_info,
            data_sources,
        );

        Self {
            buffer,
            indices_offset,
            indices_count,
        }
    }

    pub fn device_addresses(&self, ctx: &Context) -> (vk::DeviceAddress, vk::DeviceAddress) {
        let vertex_data_address = self.buffer.get_device_address(ctx);
        (
            vertex_data_address,
            vertex_data_address + self.indices_offset,
        )
    }
}

impl Destroy<Context> for Scene {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.geometry.destroy_with(ctx);
    }
}

impl Destroy<Context> for VertexIndexBuffer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.buffer.destroy_with(ctx);
    }
}
