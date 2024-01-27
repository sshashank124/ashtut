use ash::vk;

use super::{buffer::Buffer, context::Context, scope::OneshotScope, Destroy};

pub struct Scene {
    pub indices: Buffer,
    pub vertices: Buffer,
    pub primitives: Buffer,
    pub materials: Buffer,
    pub scene_desc: Buffer,
    pub host_info: scene::Info,
    pub device_info: scene::SceneDesc,
}

impl Scene {
    pub fn create(ctx: &mut Context, scope: &mut OneshotScope, scene: scene::Scene) -> Self {
        let (vertices, indices) = Self::init_vertex_index_buffer(ctx, scope, &scene.data);
        let primitives = Self::init_primitives_buffer(ctx, scope, &scene.info);
        let materials = Self::init_materials_buffer(ctx, scope, &scene.data);

        let device_info = scene::SceneDesc {
            vertices_address: vertices.get_device_address(ctx),
            indices_address: indices.get_device_address(ctx),
            materials_address: materials.get_device_address(ctx),
            primitives_address: primitives.get_device_address(ctx),
        };
        let scene_desc = Self::init_scene_desc_buffer(ctx, scope, &device_info);
        let host_info = scene.info;

        Self {
            indices,
            vertices,
            primitives,
            materials,
            scene_desc,
            host_info,
            device_info,
        }
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &scene::Data,
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
        scene: &scene::Info,
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

    fn init_materials_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene: &scene::Data,
    ) -> Buffer {
        let create_info = vk::BufferCreateInfo::builder().usage(
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        Buffer::create_with_staged_data(
            ctx,
            scope,
            "Materials Buffer",
            *create_info,
            bytemuck::cast_slice(&scene.materials),
        )
    }

    fn init_scene_desc_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene_desc: &scene::SceneDesc,
    ) -> Buffer {
        let create_info =
            vk::BufferCreateInfo::builder().usage(vk::BufferUsageFlags::UNIFORM_BUFFER);

        Buffer::create_with_staged_data(
            ctx,
            scope,
            "Scene Desc Buffer",
            *create_info,
            bytemuck::bytes_of(scene_desc),
        )
    }
}

impl Destroy<Context> for Scene {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.scene_desc.destroy_with(ctx);
        self.primitives.destroy_with(ctx);
        self.materials.destroy_with(ctx);
        self.vertices.destroy_with(ctx);
        self.indices.destroy_with(ctx);
    }
}
