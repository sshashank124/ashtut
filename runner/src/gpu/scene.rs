use ash::vk;
use shared::bytemuck;

use super::{buffer::Buffer, context::Context, scope::OneshotScope, Destroy};
use crate::data::{gltf_scene, Instance, Primitive};

pub struct Scene {
    pub vertices: Buffer,
    pub indices: Buffer,
    pub primitives: Vec<Primitive>,
    pub instances: Vec<Instance>,
}

impl Scene {
    pub fn load_gltf(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &gltf_scene::GltfScene,
    ) -> Self {
        let (vertices, indices) = Self::init_vertex_index_buffer(ctx, scope, &scene.data);

        Self {
            vertices,
            indices,
            primitives: scene.primitives.clone(),
            instances: scene.instances.clone(),
        }
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene_data: &gltf_scene::Data,
    ) -> (Buffer, Buffer) {
        let vertices = {
            let create_info = vk::BufferCreateInfo::builder().usage(
                vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            );

            Buffer::create_with_staged_data(
                ctx,
                scope,
                "Vertex Buffer",
                *create_info,
                bytemuck::cast_slice(&scene_data.vertices),
            )
        };

        let indices = {
            let create_info = vk::BufferCreateInfo::builder().usage(
                vk::BufferUsageFlags::INDEX_BUFFER
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            );

            Buffer::create_with_staged_data(
                ctx,
                scope,
                "Index Buffer",
                *create_info,
                bytemuck::cast_slice(&scene_data.indices),
            )
        };

        (vertices, indices)
    }
}

impl Destroy<Context> for Scene {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.indices.destroy_with(ctx);
        self.vertices.destroy_with(ctx);
    }
}
