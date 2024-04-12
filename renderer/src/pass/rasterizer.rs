use std::slice;

use ash::vk;

use shared::{inputs, scene};

use crate::{
    context::Context,
    framebuffers::{self, Framebuffers},
    image, pipeline,
    scope::OneshotScope,
    sync_info::SyncInfo,
    Destroy,
};

use super::common;

pub mod conf {
    pub const NAME: &str = "Rasterizer";
    pub const SHADER_VERT: &str = env!("rasterizer.vert.glsl");
    pub const SHADER_FRAG: &str = env!("rasterizer.frag.glsl");
}

pub struct Pipeline {
    pub render_pass: vk::RenderPass,
    pub target: Framebuffers<{ image::Format::Hdr }>,
    pipeline: pipeline::Pipeline<1>,
}

impl Pipeline {
    pub fn create(ctx: &mut Context, scope: &OneshotScope, common: &common::Data) -> Self {
        let render_pass = Self::create_render_pass(ctx);

        let target = Framebuffers::create(
            ctx,
            scope,
            format!("{} Target", conf::NAME),
            render_pass,
            common.resolution,
            std::slice::from_ref(&common.target),
        );

        let (layout, pipeline) = Self::create_pipeline(ctx, render_pass, common);

        let descriptor_sets = common.descriptors.sets.iter().copied().map(|a| [a]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            conf::NAME,
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
            vk::AttachmentDescription::default()
                .format(image::Format::Hdr.into())
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::GENERAL),
            vk::AttachmentDescription::default()
                .format(image::Format::Depth.into())
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];

        let color_attachment_reference = vk::AttachmentReference::default()
            .layout(vk::ImageLayout::GENERAL)
            .attachment(0);

        let depth_attachment_reference = vk::AttachmentReference::default()
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .attachment(1);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(slice::from_ref(&color_attachment_reference))
            .depth_stencil_attachment(&depth_attachment_reference);

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let render_pass_info = vk::RenderPassCreateInfo::default()
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
        common: &common::Data,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let push_constant_ranges = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<inputs::RasterizerConstants>() as _,
        };

        let layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(slice::from_ref(&common.descriptors.layout))
            .push_constant_ranges(slice::from_ref(&push_constant_ranges));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let shader_module_vert = ctx.create_shader_module_from_file(conf::SHADER_VERT);
        let shader_module_frag = ctx.create_shader_module_from_file(conf::SHADER_FRAG);
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader_module_vert)
                .name(crate::cstr!("main")),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader_module_frag)
                .name(crate::cstr!("main")),
        ];

        let (vertex_binding_descriptions, vertex_attribute_descriptions) =
            Self::vertex_binding_info();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_binding_descriptions)
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport = vk::Viewport::default()
            .width(common.resolution.width as _)
            .height(common.resolution.height as _)
            .max_depth(1.0);

        let scissor = vk::Rect2D::default().extent(common.resolution);

        let viewport_info = vk::PipelineViewportStateCreateInfo::default()
            .viewports(slice::from_ref(&viewport))
            .scissors(slice::from_ref(&scissor));

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(slice::from_ref(&color_blend_attachment));

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false);

        let create_info = vk::GraphicsPipelineCreateInfo::default()
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

        unsafe {
            ctx.destroy_shader_module(shader_module_vert, None);
            ctx.destroy_shader_module(shader_module_frag, None);
        }

        (layout, pipeline)
    }

    fn vertex_binding_info() -> (
        [vk::VertexInputBindingDescription; 1],
        [vk::VertexInputAttributeDescription; 2],
    ) {
        let bindings = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<scene::Vertex>() as _,
            input_rate: vk::VertexInputRate::VERTEX,
        }];

        let attributes = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: bytemuck::offset_of!(scene::Vertex, position) as _,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: bytemuck::offset_of!(scene::Vertex, tex_coords) as _,
            },
        ];

        (bindings, attributes)
    }

    pub fn run(&self, ctx: &Context, common: &common::Data, sync_info: &SyncInfo) {
        let commands = self.pipeline.begin_pipeline(ctx, 0);

        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .render_area(common.resolution.into())
            .framebuffer(self.target.framebuffers[0])
            .clear_values(framebuffers::CLEAR_VALUES);

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
                slice::from_ref(&common.scene.vertices),
                &[0],
            );

            ctx.cmd_bind_index_buffer(
                commands.buffer,
                *common.scene.indices,
                0,
                vk::IndexType::UINT32,
            );
        }

        let scene_info = &common.scene.host_info;
        for instance in &scene_info.instances {
            let push_constants = inputs::RasterizerConstants {
                model_transform: instance.transform,
                material_index: scene_info.primitive_infos[instance.primitive_index].material,
                ..Default::default()
            };

            unsafe {
                ctx.cmd_push_constants(
                    commands.buffer,
                    self.pipeline.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
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
                    scene_info.primitive_sizes[instance.primitive_index].indices_size,
                    1,
                    scene_info.primitive_infos[instance.primitive_index].indices_offset,
                    scene_info.primitive_infos[instance.primitive_index]
                        .vertices_offset
                        .try_into()
                        .unwrap(),
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
