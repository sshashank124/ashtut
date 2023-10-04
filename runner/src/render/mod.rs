mod commands;
mod pass;
mod swapchain;
mod sync_state;

use ash::vk;

use shared::Vertex;

use crate::{
    buffer::Buffer,
    context::Context,
    util::{Descriptors, Destroy},
};

use self::{commands::Commands, pass::Pass, swapchain::Swapchain, sync_state::SyncState};

mod conf {
    pub const SHADER_FILE: &str = env!("raster.spv");
    pub const VERTEX_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
    pub const FRAGMENT_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };

    use shared::Vertex;
    pub const VERTICES_DATA: [Vertex; 3] = [
        Vertex::new([0.0, -0.5], [1.0, 0.0, 0.0]),
        Vertex::new([-0.5, 0.5], [0.0, 1.0, 0.0]),
        Vertex::new([0.5, 0.5], [0.0, 0.0, 1.0]),
    ];
}

pub struct Pipeline {
    pass: Pass,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    vertex_buffer: Buffer,
    state: SyncState,

    // Recreate on resize
    swapchain: Swapchain,
    commands: Commands,
}

pub enum Error {
    NeedsRecreating,
}

impl Pipeline {
    pub fn create(ctx: &mut Context) -> Self {
        let pass = Pass::create(ctx);
        let (layout, pipeline) = Self::create_pipeline(ctx, *pass);
        let vertex_buffer = Self::create_vertex_buffer(ctx);
        let state = SyncState::create(ctx);

        let swapchain = Swapchain::create(ctx, *pass);
        let commands = Commands::create(ctx);
        commands.record(
            ctx,
            *pass,
            pipeline,
            *vertex_buffer,
            &swapchain.framebuffers,
        );

        Self {
            pass,
            layout,
            pipeline,
            vertex_buffer,
            state,

            swapchain,
            commands,
        }
    }

    fn create_pipeline(ctx: &Context, pass: vk::RenderPass) -> (vk::PipelineLayout, vk::Pipeline) {
        let shader_module = ctx.device.create_shader_module_from_file(conf::SHADER_FILE);

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader_module)
                .name(conf::VERTEX_SHADER_ENTRY_POINT)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader_module)
                .name(conf::FRAGMENT_SHADER_ENTRY_POINT)
                .build(),
        ];

        let vertex_bindings_description = Vertex::bindings_description();
        let vertex_attributes_description = Vertex::attributes_description();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_bindings_description)
            .vertex_attribute_descriptions(&vertex_attributes_description);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder();

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build()];

        let color_blend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let dynamic_states = [
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
        ];

        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder();

        let layout = unsafe {
            ctx.device
                .create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create graphics pipeline layout")
        };

        let create_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .layout(layout)
            .render_pass(pass)
            .dynamic_state(&dynamic_state_info)
            .build()];

        let pipeline = unsafe {
            ctx.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create graphics pipeline")[0]
        };

        unsafe { ctx.device.destroy_shader_module(shader_module, None) };

        (layout, pipeline)
    }

    fn create_vertex_buffer(ctx: &mut Context) -> Buffer {
        let create_info = vk::BufferCreateInfo::builder()
            .size(std::mem::size_of_val(&conf::VERTICES_DATA) as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        Buffer::create_with(
            ctx,
            "Vertex Buffer",
            &create_info,
            gpu_allocator::MemoryLocation::CpuToGpu,
            &conf::VERTICES_DATA,
        )
    }

    pub fn render(&mut self, ctx: &Context) -> Result<(), Error> {
        unsafe {
            ctx.device
                .wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for `in_flight` fence");
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .acquire_next_image_and_signal(self.state.image_available_semaphore()[0]);

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.device
                    .reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset `in_flight` fence");
            }

            self.commands.run(
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
        self.commands.reset(ctx);
        unsafe {
            self.swapchain.destroy_with(ctx);
        }

        self.swapchain = Swapchain::create(ctx, *self.pass);
        self.commands.record(
            ctx,
            *self.pass,
            self.pipeline,
            *self.vertex_buffer,
            &self.swapchain.framebuffers,
        );
    }
}

impl<'a> Destroy<&'a mut Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        self.commands.destroy_with(ctx);
        self.swapchain.destroy_with(ctx);

        self.state.destroy_with(ctx);
        self.vertex_buffer.destroy_with(ctx);
        ctx.device.destroy_pipeline(self.pipeline, None);
        ctx.device.destroy_pipeline_layout(self.layout, None);
        self.pass.destroy_with(ctx);
    }
}
