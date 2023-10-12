mod descriptors;
mod pass;
mod pipeline;
mod swapchain;
mod sync_state;
mod uniforms;

use ash::vk;

use shared::{bytemuck, UniformObjects};

use crate::{
    gpu::{
        buffer::Buffer,
        commands::Commands,
        context::Context,
        image::{HdrImage, Image},
        sampled_image::SampledImage,
        scope::{Scope, TempScope},
        Destroy,
    },
    model::Model,
};

use self::{
    descriptors::Descriptors, pass::Pass, pipeline::Pipeline, swapchain::Swapchain,
    sync_state::SyncState, uniforms::Uniforms,
};

mod conf {
    pub const RENDER_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        width: 1024,
        height: 768,
    };
}

pub struct Renderer {
    pass: Pass,
    pipeline: Pipeline,
    descriptors: Descriptors,

    // model
    model: Model,

    // off-screen render target
    render_target: HdrImage,

    // drawing
    render_scopes: Vec<Scope>,

    vertex_index_buffer: Buffer,
    texture: SampledImage,

    // state
    pub uniforms: Uniforms,
    state: SyncState,

    // Recreate on resize
    swapchain: Swapchain,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let pass = Pass::create(ctx);
        let descriptors = Descriptors::create(ctx);
        let pipeline = Pipeline::create(ctx, *pass, descriptors.layout);

        let model = Model::demo_viking_room();

        let render_target = Self::create_render_target(ctx);

        let render_scopes = Self::create_render_scopes(ctx);

        let mut setup_scope = Self::create_setup_scope(ctx);

        let vertex_index_buffer = Self::init_vertex_index_buffer(ctx, &mut setup_scope, &model);
        let texture = Self::init_texture(ctx, &mut setup_scope, &model);

        let swapchain = Swapchain::create(ctx, &mut setup_scope, &pass);

        setup_scope.finish(ctx);

        let uniforms = Uniforms::create(ctx);
        descriptors.bind_descriptors(ctx, &uniforms, &texture);

        let state = SyncState::create(ctx);

        Self {
            pass,
            pipeline,
            descriptors,

            model,

            render_target,

            render_scopes,

            vertex_index_buffer,
            texture,

            uniforms,
            state,

            swapchain,
        }
    }

    pub fn create_render_scopes(ctx: &Context) -> Vec<Scope> {
        (0..ctx.surface.config.image_count)
            .map(|_| Scope::create_on(Commands::create_on_queue(ctx, ctx.queues.graphics())))
            .collect()
    }

    fn create_render_target(ctx: &mut Context) -> HdrImage {
        let info = vk::ImageCreateInfo::builder().extent(conf::RENDER_RESOLUTION.into());
        Image::create(ctx, "Render Target", &info)
    }

    fn create_setup_scope(ctx: &Context) -> TempScope {
        let commands = Commands::create_on_queue(ctx, ctx.device.queues.graphics());
        TempScope::begin_on(ctx, commands)
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        setup_scope: &mut TempScope,
        model: &Model,
    ) -> Buffer {
        let data_sources = &[
            bytemuck::cast_slice(&model.mesh.vertices),
            bytemuck::cast_slice(&model.mesh.indices),
        ];
        let create_info = vk::BufferCreateInfo::builder()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER);

        Buffer::create_with_staged_data(
            ctx,
            setup_scope,
            "Vertex+Index Buffer",
            *create_info,
            data_sources,
        )
    }

    fn init_texture(ctx: &mut Context, setup_scope: &mut TempScope, model: &Model) -> SampledImage {
        let image = Image::create_from_image(ctx, setup_scope, "Texture", &model.texture);
        SampledImage::from_image(ctx, image)
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for `in_flight` fence");
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .acquire_next_image_and_signal(self.state.image_available_semaphore()[0]);
        let image_index = image_index as usize;

        self.render_scopes[image_index].commands.reset(ctx);

        self.uniforms.update(image_index, uniforms);

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset `in_flight` fence");
            }

            self.draw(
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

    pub fn draw(
        &self,
        ctx: &Context,
        image_index: usize,
        wait_on: &[vk::Semaphore],
        signal_to: &[vk::Semaphore],
        fence: vk::Fence,
    ) {
        self.record_commands_for_frame(ctx, image_index);

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(wait_on)
            .signal_semaphores(signal_to);

        self.render_scopes[image_index]
            .commands
            .submit(ctx, &submit_info, fence);
    }

    fn record_commands_for_frame(&self, ctx: &Context, image_index: usize) {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(*self.pass)
            .render_area(
                vk::Rect2D::builder()
                    .extent(ctx.surface.config.extent)
                    .build(),
            )
            .framebuffer(self.swapchain.framebuffers[image_index])
            .clear_values(&clear_values)
            .build();

        let viewports = [vk::Viewport::builder()
            .width(ctx.surface.config.extent.width as f32)
            .height(ctx.surface.config.extent.height as f32)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder()
            .extent(ctx.surface.config.extent)
            .build()];

        self.render_scopes[image_index]
            .commands
            .begin_recording(ctx);

        unsafe {
            let command_buffer = self.render_scopes[image_index].commands.buffer;

            ctx.cmd_begin_render_pass(command_buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pipeline,
            );

            ctx.cmd_set_viewport_with_count(command_buffer, &viewports);

            ctx.cmd_set_scissor_with_count(command_buffer, &scissors);

            let vertex_buffers = [*self.vertex_index_buffer];
            ctx.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &[0]);

            ctx.cmd_bind_index_buffer(
                command_buffer,
                *self.vertex_index_buffer,
                self.model.mesh.vertex_data_size() as u64,
                vk::IndexType::UINT32,
            );

            let descriptor_sets = [self.descriptors.sets[image_index]];
            ctx.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &descriptor_sets,
                &[],
            );

            ctx.cmd_draw_indexed(
                command_buffer,
                self.model.mesh.indices.len() as u32,
                1,
                0,
                0,
                0,
            );

            ctx.cmd_end_render_pass(command_buffer);
        }

        self.render_scopes[image_index]
            .commands
            .finish_recording(ctx);
    }

    pub fn recreate(&mut self, ctx: &mut Context) {
        unsafe {
            self.swapchain.destroy_with(ctx);
        }
        let mut setup_scope = Self::create_setup_scope(ctx);
        self.swapchain = Swapchain::create(ctx, &mut setup_scope, &self.pass);
        setup_scope.finish(ctx);
    }
}

impl Destroy<Context> for Renderer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.swapchain.destroy_with(ctx);

        self.state.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);

        self.texture.destroy_with(ctx);
        self.vertex_index_buffer.destroy_with(ctx);

        self.render_scopes.destroy_with(ctx);

        self.render_target.destroy_with(ctx);

        self.descriptors.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        self.pass.destroy_with(ctx);
    }
}
