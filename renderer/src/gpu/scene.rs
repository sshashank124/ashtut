use ash::vk;

use super::{
    buffer::Buffer,
    context::Context,
    image::{Format, Image},
    scope::OneshotScope,
    texture::Texture,
    Destroy,
};

pub struct Scene {
    pub indices: Buffer,
    pub vertices: Buffer,
    pub primitives: Buffer,
    pub materials: Buffer,
    pub scene_desc: Buffer,
    pub images: Vec<Image<{ Format::Color }>>,
    pub textures: Vec<Texture<{ Format::Color }>>,
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

        let scene::Scene { info, data } = scene;
        let (images, textures) = Self::init_textures(ctx, scope, &info, data);

        Self {
            indices,
            vertices,
            primitives,
            materials,
            scene_desc,
            images,
            textures,
            host_info: info,
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
                "Vertices",
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
                "Indices",
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
            "Primitives",
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
            "Materials",
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
            "Scene Desc",
            *create_info,
            bytemuck::bytes_of(scene_desc),
        )
    }

    fn init_textures(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene_info: &scene::Info,
        scene_data: scene::Data,
    ) -> (
        Vec<Image<{ Format::Color }>>,
        Vec<Texture<{ Format::Color }>>,
    ) {
        let images = if scene_data.images.is_empty() {
            vec![Image::create_from_image(
                ctx,
                scope,
                "Placeholder Texture Pixel",
                &image::RgbaImage::new(1, 1),
            )]
        } else {
            scene_data
                .images
                .into_iter()
                .map(|scene::Image { source }| {
                    let image = image::open(&source)
                        .expect("Unable to load image")
                        .into_rgba8();
                    Image::create_from_image(
                        ctx,
                        scope,
                        source.to_str().unwrap_or_default(),
                        &image,
                    )
                })
                .collect::<Vec<_>>()
        };

        let scene_textures = if scene_info.textures.is_empty() {
            std::slice::from_ref(&scene::TextureInfo { image_index: 0 })
        } else {
            scene_info.textures.as_slice()
        };
        let textures = scene_textures
            .iter()
            .enumerate()
            .map(|(idx, tex)| {
                Texture::for_image(
                    ctx,
                    format!("Texture - #{idx}"),
                    &images[tex.image_index as usize],
                )
            })
            .collect();

        (images, textures)
    }
}

impl Destroy<Context> for Scene {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.textures.destroy_with(ctx);
        self.images.destroy_with(ctx);
        self.scene_desc.destroy_with(ctx);
        self.primitives.destroy_with(ctx);
        self.materials.destroy_with(ctx);
        self.vertices.destroy_with(ctx);
        self.indices.destroy_with(ctx);
    }
}
