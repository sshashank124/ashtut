use std::ops::Deref;

use ash::vk;

use crate::gpu::{
    commands::Commands,
    context::Context,
    descriptors::Descriptors,
    framebuffer::Framebuffers,
    image::{format, Image},
    pipeline::Pipeline,
    render_pass::RenderPass,
    sampler::Sampler,
    Destroy,
};

use super::Pass;

mod conf {
    pub const SHADER_FILE: &str = env!("tonemap.spv");
    pub const VERTEX_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
    pub const FRAGMENT_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };
}

pub type TonemapPass = Pass<Tonemap>;

pub struct Tonemap {
    pub sampler: Sampler,
}

impl Tonemap {
    pub fn create(ctx: &mut Context, intermediate_target: &Framebuffers<{ format::HDR }>) -> Self {
        let descriptors = Self::create_descriptors(ctx);
        let render_pass = Self::create_render_pass(ctx);
        let pipeline = Self::create_pipeline(ctx, *render_pass, descriptors.layout);

        let pass = Pass {
            descriptors,
            render_pass,
            pipeline,
        };

        let commands = (0..ctx.surface.config.image_count)
            .map(|_| Commands::create_on_queue(ctx, ctx.queues.graphics()))
            .collect();

        let sampler = Sampler::create(ctx);

        let pass = Self { sampler };

        pass.bind_descriptors(ctx, &intermediate_target.colors[0]);

        pass
    }

    fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let bindings = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build()];
            let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let num_frames = ctx.surface.config.image_count;
            let sizes = [vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(num_frames)
                .build()];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&sizes)
                .max_sets(num_frames);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let layouts = vec![layout; ctx.surface.config.image_count as usize];
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            unsafe {
                ctx.allocate_descriptor_sets(&info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Descriptors { layout, pool, sets }
    }

    fn create_render_pass(ctx: &Context) -> RenderPass {
        let attachments = [
            vk::AttachmentDescription::builder()
                .format(ctx.surface.config.surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build(),
            vk::AttachmentDescription::builder()
                .format(format::DEPTH)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build(),
        ];

        let color_attachment_references = [vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .attachment(0)
            .build()];

        let depth_attachment_reference = vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .attachment(1);

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_references)
            .depth_stencil_attachment(&depth_attachment_reference)
            .build()];

        let dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .build()];

        let info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        RenderPass::create(ctx, &info)
    }

    fn create_pipeline(
        ctx: &Context,
        pass: vk::RenderPass,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Pipeline {
        let shader_module = ctx.create_shader_module_from_file(conf::SHADER_FILE);
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

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder();

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::CLOCKWISE)
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

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false);

        let dynamic_states = [
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
        ];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let create_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .depth_stencil_state(&depth_stencil_info)
            .layout(layout)
            .render_pass(pass)
            .dynamic_state(&dynamic_state_info)
            .build()];

        let pipeline = unsafe {
            ctx.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create pipeline")[0]
        };

        unsafe { ctx.destroy_shader_module(shader_module, None) };

        Pipeline { layout, pipeline }
    }

    pub fn bind_descriptors(
        &self,
        ctx: &Context,
        rendered_image: &Image<{ vk::Format::R32G32B32A32_SFLOAT }>,
    ) {
        for &set in &self.pass.descriptors.sets {
            let rendered_image_info = [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(rendered_image.view)
                .sampler(*self.sampler)
                .build()];

            let writes = [vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&rendered_image_info)
                .build()];

            unsafe {
                ctx.update_descriptor_sets(&writes, &[]);
            }
        }
    }

    pub fn draw(
        &self,
        ctx: &Context,
        image_index: usize,
        wait_on: &[vk::Semaphore],
        signal_to: &[vk::Semaphore],
        fence: vk::Fence,
        framebuffers: (
            &Framebuffers<{ format::HDR }>,
            &Framebuffers<{ vk::Format::UNDEFINED }>,
        ),
    ) {
        self.commands[image_index].reset(ctx);

        self.record_commands(ctx, image_index, framebuffers.0, framebuffers.1);

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(wait_on)
            .signal_semaphores(signal_to);

        self.commands[image_index].submit(ctx, &submit_info, Some(fence));
    }

    fn record_commands(
        &self,
        ctx: &Context,
        image_index: usize,
        intermediate_target: &Framebuffers<{ format::HDR }>,
        render_target: &Framebuffers<{ vk::Format::UNDEFINED }>,
    ) {
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
            .render_pass(*self.pass.render_pass)
            .render_area(
                vk::Rect2D::builder()
                    .extent(ctx.surface.config.extent)
                    .build(),
            )
            .framebuffer(render_target.framebuffers[image_index])
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

        self.commands[image_index].begin_recording(ctx);

        unsafe {
            let command_buffer = self.commands[image_index].buffer;

            intermediate_target.colors[0].transition_layout_ready_to_read(ctx, command_buffer);

            ctx.cmd_begin_render_pass(command_buffer, &pass_info, vk::SubpassContents::INLINE);

            ctx.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pass.pipeline,
            );

            ctx.cmd_set_viewport_with_count(command_buffer, &viewports);

            ctx.cmd_set_scissor_with_count(command_buffer, &scissors);

            let descriptor_sets = [self.pass.descriptors.sets[image_index]];
            ctx.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pass.pipeline.layout,
                0,
                &descriptor_sets,
                &[],
            );

            ctx.cmd_draw(command_buffer, 3, 1, 0, 0);

            ctx.cmd_end_render_pass(command_buffer);

            intermediate_target.colors[0].transition_layout_ready_to_write(ctx, command_buffer);
        }

        self.commands[image_index].finish_recording(ctx);
    }
}

impl Destroy<Context> for Tonemap {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.sampler.destroy_with(ctx);

        self.commands.destroy_with(ctx);
        self.pass.destroy_with(ctx);
    }
}

impl Deref for Tonemap {
    type Target = Pass;
    fn deref(&self) -> &Self::Target {
        &self.pass
    }
}
