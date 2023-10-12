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
        commands::{Commands, TempCommands},
        context::Context,
        image::{DepthImage, Image},
        sampled_image::{SampledColorImage, SampledHdrImage, SampledImage},
        scope::Scope,
        Destroy,
    },
    model::Model,
};

use self::{
    descriptors::Descriptors, pass::Pass, pipeline::Pipeline, swapchain::Swapchain,
    sync_state::SyncState, uniforms::Uniforms,
};

pub mod conf {
    pub const OFFSCREEN_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        width: 1024,
        height: 768,
    };
}

pub struct Renderer {
    // offscreen pass
    offscreen_descriptors: Descriptors,
    offscreen_pass: Pass,
    offscreen_pipeline: Pipeline,

    // tonemap pass
    tonemap_descriptors: Descriptors,
    tonemap_pass: Pass,
    tonemap_pipeline: Pipeline,

    // model
    model: Model,

    // off-screen render target
    render_target: (SampledHdrImage, DepthImage, vk::Framebuffer),

    // drawing
    offscreen_commands: Commands,
    tonemap_commands: Vec<Commands>,

    vertex_index_buffer: Buffer,
    texture: SampledColorImage,

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
        let offscreen_descriptors = Descriptors::create_offscreen(ctx);
        let offscreen_pass = Pass::create_for_offscreen(ctx);
        let offscreen_pipeline = Pipeline::create_offscreen(
            ctx,
            *offscreen_pass,
            conf::OFFSCREEN_RESOLUTION,
            offscreen_descriptors.layout,
        );

        let tonemap_descriptors = Descriptors::create_tonemap(ctx);
        let tonemap_pass = Pass::create_for_tonemap(ctx);
        let tonemap_pipeline =
            Pipeline::create_tonemap(ctx, *tonemap_pass, tonemap_descriptors.layout);

        let model = Model::demo_viking_room();

        let render_target = Self::create_render_targets(ctx, *offscreen_pass);

        let offscreen_commands = Commands::create_on_queue(ctx, ctx.queues.graphics());
        let tonemap_commands = Self::create_tonemap_commands(ctx);

        let mut setup_scope = Self::create_setup_scope(ctx);

        let vertex_index_buffer = Self::init_vertex_index_buffer(ctx, &mut setup_scope, &model);
        let texture = Self::init_texture(ctx, &mut setup_scope, &model);

        let swapchain = Swapchain::create(ctx, &mut setup_scope, *tonemap_pass);

        setup_scope.finish(ctx);

        let uniforms = Uniforms::create(ctx);
        offscreen_descriptors.bind_offscreen_descriptors(ctx, &uniforms, &texture);
        tonemap_descriptors.bind_tonemap_descriptors(ctx, &render_target.0);

        let state = SyncState::create(ctx);

        Self {
            offscreen_descriptors,
            offscreen_pass,
            offscreen_pipeline,

            tonemap_descriptors,
            tonemap_pass,
            tonemap_pipeline,

            model,

            render_target,

            offscreen_commands,
            tonemap_commands,

            vertex_index_buffer,
            texture,

            uniforms,
            state,

            swapchain,
        }
    }

    pub fn create_tonemap_commands(ctx: &Context) -> Vec<Commands> {
        (0..ctx.surface.config.image_count)
            .map(|_| Commands::create_on_queue(ctx, ctx.queues.graphics()))
            .collect()
    }

    fn create_render_targets(
        ctx: &mut Context,
        pass: vk::RenderPass,
    ) -> (SampledHdrImage, DepthImage, vk::Framebuffer) {
        let info = vk::ImageCreateInfo::builder().extent(conf::OFFSCREEN_RESOLUTION.into());
        let color = {
            let image = Image::create(ctx, "Render Target [COLOR]", &info);
            SampledHdrImage::from_image(ctx, image)
        };
        let depth = Image::create(ctx, "Render Target [DEPTH]", &info);

        let attachments = [color.image.view, depth.view];
        let framebuffer_info = vk::FramebufferCreateInfo::builder()
            .render_pass(pass)
            .attachments(&attachments)
            .width(conf::OFFSCREEN_RESOLUTION.width)
            .height(conf::OFFSCREEN_RESOLUTION.height)
            .layers(1);
        let framebuffer = unsafe {
            ctx.create_framebuffer(&framebuffer_info, None)
                .expect("Failed to create framebuffer")
        };

        (color, depth, framebuffer)
    }

    fn create_setup_scope(ctx: &Context) -> Scope {
        let commands = TempCommands::create_on_queue(ctx, ctx.device.queues.graphics());
        Scope::begin_on(ctx, commands)
    }

    fn init_vertex_index_buffer(
        ctx: &mut Context,
        setup_scope: &mut Scope,
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

    fn init_texture(
        ctx: &mut Context,
        setup_scope: &mut Scope,
        model: &Model,
    ) -> SampledColorImage {
        let image = Image::create_from_image(ctx, setup_scope, "Texture", &model.texture);
        SampledImage::from_image(ctx, image)
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            let fences = [self.state.offscreen_fence];
            ctx.wait_for_fences(&fences, true, u64::MAX)
                .expect("Failed to wait for `offscreen` fence");
        }

        self.uniforms.update(uniforms);

        self.offscreen_commands.reset(ctx);

        unsafe {
            let fences = [self.state.offscreen_fence];
            ctx.reset_fences(&fences)
                .expect("Failed to reset `offscreen` fence");
        }

        self.draw_offscreen(ctx, self.state.offscreen_fence);

        unsafe {
            ctx.wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for `in_flight` fence");
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .acquire_next_image_and_signal(self.state.image_available_semaphore()[0]);
        let image_index = image_index as usize;

        self.tonemap_commands[image_index].reset(ctx);

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset `in_flight` fence");
            }

            self.draw_tonemap(
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

    pub fn draw_offscreen(&self, ctx: &Context, fence: vk::Fence) {
        self.record_offscreen_commands(ctx);
        let submit_info = vk::SubmitInfo::builder();
        self.offscreen_commands.submit(ctx, &submit_info, fence);
    }

    pub fn draw_tonemap(
        &self,
        ctx: &Context,
        image_index: usize,
        wait_on: &[vk::Semaphore],
        signal_to: &[vk::Semaphore],
        fence: vk::Fence,
    ) {
        self.record_tonemap_commands(ctx, image_index);

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(wait_on)
            .signal_semaphores(signal_to);

        self.tonemap_commands[image_index].submit(ctx, &submit_info, fence);
    }

    fn record_offscreen_commands(&self, ctx: &Context) {
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
            .render_pass(*self.offscreen_pass)
            .render_area(
                vk::Rect2D::builder()
                    .extent(conf::OFFSCREEN_RESOLUTION)
                    .build(),
            )
            .framebuffer(self.render_target.2)
            .clear_values(&clear_values)
            .build();

        self.offscreen_commands.begin_recording(ctx);

        unsafe {
            let command_buffer = self.offscreen_commands.buffer;

            ctx.cmd_begin_render_pass(command_buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.offscreen_pipeline,
            );

            let vertex_buffers = [*self.vertex_index_buffer];
            ctx.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &[0]);

            ctx.cmd_bind_index_buffer(
                command_buffer,
                *self.vertex_index_buffer,
                self.model.mesh.vertex_data_size() as u64,
                vk::IndexType::UINT32,
            );

            let descriptor_sets = [self.offscreen_descriptors.sets[0]];
            ctx.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.offscreen_pipeline.layout,
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

        self.offscreen_commands.finish_recording(ctx);
    }

    fn record_tonemap_commands(&self, ctx: &Context, image_index: usize) {
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
            .render_pass(*self.tonemap_pass)
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

        self.tonemap_commands[image_index].begin_recording(ctx);

        unsafe {
            let command_buffer = self.tonemap_commands[image_index].buffer;

            self.render_target
                .0
                .image
                .transition_layout_ready_to_read(ctx, command_buffer);

            ctx.cmd_begin_render_pass(command_buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.tonemap_pipeline,
            );

            ctx.cmd_set_viewport_with_count(command_buffer, &viewports);

            ctx.cmd_set_scissor_with_count(command_buffer, &scissors);

            let descriptor_sets = [self.tonemap_descriptors.sets[image_index]];
            ctx.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.tonemap_pipeline.layout,
                0,
                &descriptor_sets,
                &[],
            );

            ctx.cmd_draw(command_buffer, 3, 1, 0, 0);

            ctx.cmd_end_render_pass(command_buffer);

            self.render_target
                .0
                .image
                .transition_layout_ready_to_write(ctx, command_buffer);
        }

        self.tonemap_commands[image_index].finish_recording(ctx);
    }

    pub fn recreate(&mut self, ctx: &mut Context) {
        unsafe {
            self.swapchain.destroy_with(ctx);
        }
        let mut setup_scope = Self::create_setup_scope(ctx);
        self.swapchain = Swapchain::create(ctx, &mut setup_scope, *self.tonemap_pass);
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

        self.tonemap_commands.destroy_with(ctx);
        self.offscreen_commands.destroy_with(ctx);

        self.render_target.0.destroy_with(ctx);
        self.render_target.1.destroy_with(ctx);
        ctx.destroy_framebuffer(self.render_target.2, None);

        self.tonemap_pipeline.destroy_with(ctx);
        self.tonemap_pass.destroy_with(ctx);
        self.tonemap_descriptors.destroy_with(ctx);

        self.offscreen_pipeline.destroy_with(ctx);
        self.offscreen_pass.destroy_with(ctx);
        self.offscreen_descriptors.destroy_with(ctx);
    }
}
