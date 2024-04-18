use std::slice;

use ash::vk;

use shared::{inputs, scene};

use crate::{
    commands::Commands, context::Context, image, memory, pipeline, sync_info::SyncInfo, Destroy,
};

pub mod conf {
    pub const NAME: &str = "Rasterizer";
    pub const SHADER_VERT: &str = env!("rasterizer.vert.glsl");
    pub const SHADER_FRAG: &str = env!("rasterizer.frag.glsl");
}

pub struct Pipeline {
    depth: image::Image<{ image::Format::Depth }>,
    pipeline: pipeline::Pipeline<1>,
}

impl Pipeline {
    pub fn create<const FORMAT: image::Format>(ctx: &Context, data: &super::Data<FORMAT>) -> Self {
        let commands = Commands::begin_on_queue(
            ctx,
            format!("{} - Initialization", conf::NAME),
            ctx.queues.graphics(),
        );

        let depth = {
            let info = vk::ImageCreateInfo::default().extent(data.target.extent.into());

            image::Image::create(
                ctx,
                commands.buffer,
                format!("{} Target - Depth", conf::NAME),
                &info,
                &memory::purpose::dedicated(),
                Some(&image::BarrierInfo::DEPTH),
            )
        };

        let (layout, pipeline) = Self::create_pipeline(ctx, data);

        let descriptor_sets = data.descriptors.sets.iter().copied().map(|a| [a]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            conf::NAME.to_owned(),
            descriptor_sets,
            layout,
            pipeline,
            ctx.queues.graphics(),
            1,
        );

        commands.finish(ctx, &vk::SubmitInfo::default(), None);

        Self { depth, pipeline }
    }

    fn create_pipeline<const FORMAT: image::Format>(
        ctx: &Context,
        data: &super::Data<FORMAT>,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let push_constant_ranges = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<inputs::RasterizerConstants>() as _,
        };

        let layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(slice::from_ref(&data.descriptors.layout))
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
            .width(data.target.extent.width as _)
            .height(data.target.extent.height as _)
            .max_depth(1.0);

        let scissor = data.target.extent.into();

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

        let color_formats = [FORMAT.into()];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_formats)
            .depth_attachment_format(image::Format::Depth.into());

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
            .push_next(&mut rendering_info);

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

    pub fn run<const FORMAT: image::Format>(
        &self,
        ctx: &Context,
        data: &super::Data<FORMAT>,
        sync_info: &SyncInfo,
    ) {
        let commands = self.pipeline.begin_pipeline(ctx, 0);

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(data.target.view)
            .image_layout(vk::ImageLayout::GENERAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)];

        let depth_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(self.depth.view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(image::Image::CLEAR_VALUE);

        let rendering_info = vk::RenderingInfo::default()
            .render_area(self.depth.extent.into())
            .layer_count(1)
            .color_attachments(&color_attachments)
            .depth_attachment(&depth_attachment);

        unsafe {
            ctx.cmd_begin_rendering(commands.buffer, &rendering_info);

            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pipeline,
            );

            ctx.cmd_bind_vertex_buffers(
                commands.buffer,
                0,
                slice::from_ref(&data.scene.vertices),
                &[0],
            );

            ctx.cmd_bind_index_buffer(
                commands.buffer,
                *data.scene.indices,
                0,
                vk::IndexType::UINT32,
            );
        }

        let scene_info = &data.scene.info.host;
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

        unsafe { ctx.cmd_end_rendering(commands.buffer) };

        self.pipeline.submit_pipeline(ctx, 0, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        self.depth.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
    }
}
