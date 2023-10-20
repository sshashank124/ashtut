use std::{
    ops::{Deref, DerefMut},
    slice,
};

use ash::vk;
use shared::Vertex;

use crate::gpu::{
    context::Context,
    descriptors::Descriptors,
    framebuffers::{self, Framebuffers},
    image::{format, Image},
    model::Model,
    pipeline,
    scope::OneshotScope,
    sync_info::SyncInfo,
    uniforms::Uniforms,
    Descriptions, Destroy,
};

pub mod conf {
    pub const SHADER_FILE: &str = env!("raster.spv");
    pub const STAGE_VERTEX: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
    pub const STAGE_FRAGMENT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };
}

pub struct Data {
    pub uniforms: Uniforms,
    models: Vec<Model>,
}

pub struct Pipeline {
    data: Data,
    pub render_pass: vk::RenderPass,
    pub target: Framebuffers<{ format::HDR }>,
    pipeline: pipeline::Pipeline,
}

impl Data {
    pub fn create(ctx: &mut Context) -> Self {
        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.graphics());
        let uniforms = Uniforms::create(ctx);
        let models = vec![Model::demo_viking_room(ctx, &mut init_scope)];
        init_scope.finish(ctx);

        Self { uniforms, models }
    }

    pub fn bind_to_descriptors(&self, ctx: &Context, descriptors: &Descriptors) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(*self.uniforms.buffer)
            .range(vk::WHOLE_SIZE);

        let texture_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.models[0].texture.image.view)
            .sampler(*self.models[0].texture.sampler);

        let writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptors.sets[0])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(slice::from_ref(&buffer_info))
                .build(),
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptors.sets[0])
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&texture_info))
                .build(),
        ];

        unsafe {
            ctx.update_descriptor_sets(&writes, &[]);
        }
    }
}

impl Pipeline {
    pub fn create(ctx: &mut Context, data: Data, target: &Image<{ format::HDR }>) -> Self {
        let render_pass = Self::create_render_pass(ctx);

        let target = Framebuffers::create(
            ctx,
            "Render target",
            render_pass,
            super::super::conf::FRAME_RESOLUTION,
            std::slice::from_ref(target),
        );

        let descriptors = Self::create_descriptors(ctx);
        data.bind_to_descriptors(ctx, &descriptors);

        let (layout, pipeline) = Self::create_pipeline(ctx, render_pass, descriptors.layout);
        let pipeline =
            pipeline::Pipeline::new(ctx, descriptors, layout, pipeline, ctx.queues.graphics(), 1);

        Self {
            data,
            render_pass,
            target,
            pipeline,
        }
    }

    fn create_render_pass(ctx: &Context) -> vk::RenderPass {
        let attachments = [
            vk::AttachmentDescription::builder()
                .format(format::HDR)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::GENERAL)
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

        let color_attachment_reference = vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::GENERAL)
            .attachment(0);

        let depth_attachment_reference = vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .attachment(1);

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(slice::from_ref(&color_attachment_reference))
            .depth_stencil_attachment(&depth_attachment_reference);

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(slice::from_ref(&subpass))
            .dependencies(slice::from_ref(&dependency));

        unsafe {
            ctx.create_render_pass(&render_pass_info, None)
                .expect("Failed to create render pass")
        }
    }

    fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ];
            let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let sizes = [
                vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .build(),
                vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .build(),
            ];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&sizes)
                .max_sets(1);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(slice::from_ref(&layout));
            unsafe {
                ctx.allocate_descriptor_sets(&info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Descriptors { layout, pool, sets }
    }

    fn create_pipeline(
        ctx: &Context,
        render_pass: vk::RenderPass,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let shader_module = ctx.create_shader_module_from_file(conf::SHADER_FILE);
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader_module)
                .name(conf::STAGE_VERTEX)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader_module)
                .name(conf::STAGE_FRAGMENT)
                .build(),
        ];

        let vertex_bindings_description = Vertex::bindings_description();
        let vertex_attributes_description = Vertex::attributes_description();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_bindings_description)
            .vertex_attribute_descriptions(&vertex_attributes_description);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport = vk::Viewport::builder()
            .width(super::super::conf::FRAME_RESOLUTION.width as f32)
            .height(super::super::conf::FRAME_RESOLUTION.height as f32)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder().extent(super::super::conf::FRAME_RESOLUTION);

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(slice::from_ref(&viewport))
            .scissors(slice::from_ref(&scissor));

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::BACK);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(slice::from_ref(&color_blend_attachment));

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false);

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(slice::from_ref(&descriptor_set_layout));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .depth_stencil_state(&depth_stencil_info)
            .layout(layout)
            .render_pass(render_pass);

        let pipeline = unsafe {
            ctx.create_graphics_pipelines(
                vk::PipelineCache::null(),
                slice::from_ref(&create_info),
                None,
            )
            .expect("Failed to create pipeline")[0]
        };

        unsafe { ctx.destroy_shader_module(shader_module, None) };

        (layout, pipeline)
    }

    pub fn run(&self, ctx: &Context, sync_info: &SyncInfo) {
        let commands = self.pipeline.begin_pipeline(ctx, 0);

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .render_area(super::super::conf::FRAME_RESOLUTION.into())
            .framebuffer(self.target.framebuffers[0])
            .clear_values(framebuffers::CLEAR_VALUES)
            .build();

        unsafe {
            ctx.cmd_begin_render_pass(
                commands.buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pipeline,
            );

            ctx.cmd_bind_descriptor_sets(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                slice::from_ref(&self.pipeline.descriptors.sets[0]),
                &[],
            );

            ctx.cmd_bind_vertex_buffers(
                commands.buffer,
                0,
                slice::from_ref(&self.models[0].vertex_index_buffer),
                &[0],
            );

            ctx.cmd_bind_index_buffer(
                commands.buffer,
                *self.models[0].vertex_index_buffer,
                self.models[0].mesh.indices_offset() as _,
                vk::IndexType::UINT32,
            );

            ctx.cmd_draw_indexed(
                commands.buffer,
                self.models[0].mesh.indices.len() as _,
                1,
                0,
                0,
                0,
            );

            ctx.cmd_end_render_pass(commands.buffer);
        }

        self.pipeline.submit_pipeline(ctx, 0, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.target.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        ctx.destroy_render_pass(self.render_pass, None);
        self.data.destroy_with(ctx);
    }
}

impl Destroy<Context> for Data {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.models.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);
    }
}

impl Deref for Pipeline {
    type Target = Data;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Pipeline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
