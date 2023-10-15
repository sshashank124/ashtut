use ash::vk;
use shared::bytemuck;

use crate::mesh::Mesh;

use super::{
    buffer::Buffer,
    context::Context,
    image::{format, Image},
    scope::OneshotScope,
    texture::Texture,
    Destroy,
};

pub struct Model {
    pub mesh: Mesh,
    pub texture_image: image::RgbaImage,
    pub vertex_index_buffer: Buffer,
    pub texture: Texture<{ format::COLOR }>,
}

impl Model {
    fn from_data(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        mesh: Mesh,
        texture_image: image::RgbaImage,
    ) -> Self {
        let vertex_index_buffer = Self::init_vertex_index_buffer(ctx, scope, &mesh);
        let texture = Self::init_texture(ctx, scope, &texture_image);

        Self {
            mesh,
            texture_image,
            vertex_index_buffer,
            texture,
        }
    }

    pub fn from_files(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        mesh_file: &str,
        texture_file: &str,
    ) -> Self {
        let mesh = Mesh::from_file(mesh_file);
        let texture_image = crate::util::load_image_from_file(texture_file);
        Self::from_data(ctx, scope, mesh, texture_image)
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        mesh: &Mesh,
    ) -> Buffer {
        let data_sources = &[
            bytemuck::cast_slice(&mesh.vertices),
            bytemuck::cast_slice(&mesh.indices),
        ];
        let create_info = vk::BufferCreateInfo::builder().usage(
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
        );

        Buffer::create_with_staged_data(
            ctx,
            scope,
            "Vertex+Index Buffer",
            *create_info,
            data_sources,
        )
    }

    fn init_texture(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        texture_image: &image::RgbaImage,
    ) -> Texture<{ format::COLOR }> {
        let image = Image::create_from_image(ctx, scope, "Texture", texture_image);
        Texture::from_image(ctx, image)
    }

    pub fn buffer_device_addresses(&self, ctx: &Context) -> (vk::DeviceAddress, vk::DeviceAddress) {
        let vertex_data_address = self.vertex_index_buffer.get_device_address(ctx);
        (
            vertex_data_address,
            vertex_data_address | self.mesh.indices_offset() as vk::DeviceAddress,
        )
    }

    #[allow(dead_code)]
    pub fn demo_viking_room(ctx: &mut Context, init_scope: &mut OneshotScope) -> Self {
        Self::from_files(
            ctx,
            init_scope,
            "assets/models/viking_room.obj",
            "assets/textures/viking_room.png",
        )
    }

    #[allow(dead_code)]
    pub fn demo_2_planes(ctx: &mut Context, init_scope: &mut OneshotScope) -> Self {
        Self::from_data(
            ctx,
            init_scope,
            Mesh::demo_2_planes(),
            crate::util::load_image_from_file("assets/textures/statue.jpg"),
        )
    }
}

impl Destroy<Context> for Model {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.texture.destroy_with(ctx);
        self.vertex_index_buffer.destroy_with(ctx);
    }
}
