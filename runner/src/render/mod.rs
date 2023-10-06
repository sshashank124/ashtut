mod descriptors;
mod draw_commands;
mod pass;
mod pipeline;
mod swapchain;
mod sync_state;
mod uniforms;

use ash::vk;

use shared::{bytemuck, UniformObjects};

use crate::{buffer::Buffer, context::Context, util::Destroy};

use self::{
    descriptors::Descriptors, draw_commands::DrawCommands, pass::Pass, pipeline::Pipeline,
    swapchain::Swapchain, sync_state::SyncState, uniforms::Uniforms,
};

mod data {
    use shared::Vertex;
    pub const VERTICES_DATA: &[Vertex] = &[
        Vertex::new([-0.5, -0.5], [1.0, 0.0, 0.0]),
        Vertex::new([0.5, -0.5], [0.0, 1.0, 0.0]),
        Vertex::new([0.5, 0.5], [0.0, 0.0, 1.0]),
        Vertex::new([-0.5, 0.5], [1.0, 1.0, 0.0]),
    ];
    pub fn indices_offset() -> u64 {
        std::mem::size_of_val(VERTICES_DATA) as u64
    }
    pub const INDICES_DATA: &[u16] = &[0, 1, 2, 0, 2, 3];
}

pub struct Renderer {
    pass: Pass,
    pipeline: Pipeline,
    vertex_index_buffer: Buffer,
    descriptors: Descriptors,

    // state
    pub uniforms: Uniforms,
    state: SyncState,

    // Recreate on resize
    swapchain: Swapchain,
    draw_commands: DrawCommands,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let pass = Pass::create(ctx);
        let descriptors = Descriptors::create(ctx);
        let pipeline = Pipeline::create(ctx, *pass, descriptors.layout);
        let vertex_index_buffer = Self::init_vertex_index_buffer(ctx);

        let uniforms = Uniforms::create(ctx);
        descriptors.add_uniforms(ctx, &uniforms);

        let state = SyncState::create(ctx);

        let swapchain = Swapchain::create(ctx, &pass);
        let draw_commands = DrawCommands::create(ctx);
        draw_commands.record(
            ctx,
            &pass,
            &pipeline,
            &vertex_index_buffer,
            &descriptors.sets,
            &swapchain.framebuffers,
        );

        Self {
            pass,
            pipeline,
            vertex_index_buffer,
            descriptors,

            uniforms,
            state,

            swapchain,
            draw_commands,
        }
    }

    fn init_vertex_index_buffer(ctx: &mut Context) -> Buffer {
        let data_sources = &[
            bytemuck::cast_slice(data::VERTICES_DATA),
            bytemuck::cast_slice(data::INDICES_DATA),
        ];
        let create_info = vk::BufferCreateInfo::builder()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        Buffer::create_with_staged_data(ctx, "Vertex Buffer", *create_info, data_sources)
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            ctx.device
                .wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for `in_flight` fence");
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .acquire_next_image_and_signal(self.state.image_available_semaphore()[0]);

        self.uniforms.update(image_index, uniforms);

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.device
                    .reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset `in_flight` fence");
            }

            self.draw_commands.run(
                ctx,
                image_index,
                self.state.image_available_semaphore(),
                self.state.render_finished_semaphore(),
                self.state.in_flight_fence()[0],
            );

            self.swapchain
                .present_to_when(ctx, image_index, self.state.render_finished_semaphore())
        };

        self.state.advance();

        (!needs_recreating)
            .then_some(())
            .ok_or(Error::NeedsRecreating)
    }

    pub fn recreate(&mut self, ctx: &mut Context) {
        unsafe {
            self.swapchain.destroy_with(ctx);
        }

        self.swapchain = Swapchain::create(ctx, &self.pass);
        self.draw_commands.record(
            ctx,
            &self.pass,
            &self.pipeline,
            &self.vertex_index_buffer,
            &self.descriptors.sets,
            &self.swapchain.framebuffers,
        );
    }
}

impl<'a> Destroy<&'a mut Context> for Renderer {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        self.swapchain.destroy_with(ctx);

        self.state.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);

        self.descriptors.destroy_with(ctx);
        self.vertex_index_buffer.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        self.pass.destroy_with(ctx);
    }
}
