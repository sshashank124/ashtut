use std::slice;

use ash::vk;
use shared::{bytemuck, PushConstants, Vertex};

use crate::gpu::{
    context::Context,
    framebuffers::{self, Framebuffers},
    image::format,
    pipeline,
    scope::OneshotScope,
    sync_info::SyncInfo,
    Descriptions, Destroy,
};

use super::common;

pub mod conf {
    pub const SHADER_FILE: &str = env!("raster.spv");
    pub const STAGE_VERTEX: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
    pub const STAGE_FRAGMENT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };
}

pub struct Pipeline {
    pub render_pass: vk::RenderPass,
    pub target: Framebuffers<{ format::HDR }>,
    pipeline: pipeline::Pipeline<1>,
}

impl Pipeline {
    pub fn create(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        scene_data: &common::SceneData,
    ) -> Self {
        let render_pass = Self::create_render_pass(ctx);

        let target = Framebuffers::create(
            ctx,
            scope,
            "Render target",
            render_pass,
            super::super::conf::FRAME_RESOLUTION,
            std::slice::from_ref(&scene_data.target),
        );

        let (layout, pipeline) = Self::create_pipeline(ctx, render_pass, scene_data);

        let descriptor_sets = scene_data.descriptors.sets.iter().copied().map(|a| [a]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            descriptor_sets,
            layout,
            pipeline,
            ctx.queues.graphics(),
            1,
        );

        Self {
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

    fn create_pipeline(
        ctx: &Context,
        render_pass: vk::RenderPass,
        scene_data: &common::SceneData,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let push_constant_ranges = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: std::mem::size_of::<PushConstants>() as _,
        };

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(slice::from_ref(&scene_data.descriptors.layout))
            .push_constant_ranges(slice::from_ref(&push_constant_ranges));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

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

        let vertex_binding_descriptions = Vertex::bindings_description();
        let vertex_attribute_descriptions = Vertex::attributes_description();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);

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
            .cull_mode(vk::CullModeFlags::NONE);

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

    pub fn run(&self, ctx: &Context, scene_data: &common::SceneData, sync_info: &SyncInfo) {
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

            ctx.cmd_bind_vertex_buffers(
                commands.buffer,
                0,
                slice::from_ref(&scene_data.geometry.buffer),
                &[0],
            );

            ctx.cmd_bind_index_buffer(
                commands.buffer,
                *scene_data.geometry.buffer,
                scene_data.geometry.indices_offset,
                vk::IndexType::UINT32,
            );
        }

        for instance in &scene_data.instances {
            let primitive = &scene_data.primitives[instance.primitive_index];
            unsafe {
                let push_constants = PushConstants {
                    model_transform: instance.transform,
                };

                ctx.cmd_push_constants(
                    commands.buffer,
                    self.pipeline.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytemuck::bytes_of(&push_constants),
                );

                ctx.cmd_bind_descriptor_sets(
                    commands.buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout,
                    0,
                    &self.pipeline.descriptor_sets[0],
                    &[],
                );

                ctx.cmd_draw_indexed(
                    commands.buffer,
                    primitive.indices.count() as _,
                    1,
                    primitive.indices.start as _,
                    0,
                    0,
                );
            }
        }

        unsafe { ctx.cmd_end_render_pass(commands.buffer) };

        self.pipeline.submit_pipeline(ctx, 0, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.target.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
        ctx.destroy_render_pass(self.render_pass, None);
    }
}
