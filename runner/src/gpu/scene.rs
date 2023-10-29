use ash::vk;

use shared::{self, bytemuck};

use super::{buffer::Buffer, context::Context, scope::OneshotScope, Destroy};
use crate::data::gltf_scene;

pub struct Scene {
    pub indices: Buffer,
    pub vertices: Buffer,
    pub primitives: Buffer,
    pub host_info: gltf_scene::Info,
    pub device_info: shared::SceneInfo,
}

impl Scene {
    pub fn create(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: gltf_scene::GltfScene,
    ) -> Self {
        let (vertices, indices) = Self::init_vertex_index_buffer(ctx, scope, &scene.data);

        let host_info = scene.info;
        let primitives = Self::init_primitives_buffer(ctx, scope, &host_info);

        let device_info = shared::SceneInfo {
            indices_address: indices.get_device_address(ctx),
            vertices_address: vertices.get_device_address(ctx),
            primitives_address: primitives.get_device_address(ctx),
        };

        Self {
            indices,
            vertices,
            primitives,

            host_info,
            device_info,
        }
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &gltf_scene::Data,
    ) -> (Buffer, Buffer) {
        let vertices = {
            let create_info = vk::BufferCreateInfo::builder().usage(
                vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            );

            Buffer::create_with_staged_data(
                ctx,
                scope,
                "Vertex Buffer",
                *create_info,
                bytemuck::cast_slice(&scene.vertices),
            )
        };

        let indices = {
            let create_info = vk::BufferCreateInfo::builder().usage(
                vk::BufferUsageFlags::INDEX_BUFFER
                    | vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            );

            Buffer::create_with_staged_data(
                ctx,
                scope,
                "Index Buffer",
                *create_info,
                bytemuck::cast_slice(&scene.indices),
            )
        };

        (vertices, indices)
    }

    fn init_primitives_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &gltf_scene::Info,
    ) -> Buffer {
        let create_info = vk::BufferCreateInfo::builder().usage(
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        Buffer::create_with_staged_data(
            ctx,
            scope,
            "Primitives Buffer",
            *create_info,
            bytemuck::cast_slice(&scene.primitive_infos),
        )
    }
}

impl Destroy<Context> for Scene {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.primitives.destroy_with(ctx);
        self.vertices.destroy_with(ctx);
        self.indices.destroy_with(ctx);
    }
}
