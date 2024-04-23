use std::{ops::Deref, slice};

use ash::vk;

use crate::{
    context::Context, descriptors::Descriptors, image, pipeline, sampler::Sampler,
    sync_info::SyncInfo, Destroy,
};

mod conf {
    pub const NAME: &str = "Tonemap";
    pub const SHADER_VERT: &str = env!("tonemap.vert.glsl");
    pub const SHADER_FRAG: &str = env!("tonemap.frag.glsl");
}

pub struct Data<const FORMAT: image::Format> {
    descriptors: Descriptors,
    input_image: image::Image<FORMAT>,
    sampler: Sampler,
}

pub struct Pipeline<const INPUT_FORMAT: image::Format, const OUTPUT_FORMAT: image::Format> {
    data: Data<INPUT_FORMAT>,
    pipeline: pipeline::Pipeline<1>,
}

impl<const FORMAT: image::Format> Data<FORMAT> {
    pub fn create(ctx: &Context, data: &super::Data<FORMAT>) -> Self {
        firestorm::profile_method!(create);

        let descriptors = Self::create_descriptors(ctx);

        let input_image = image::Image::new(
            ctx,
            format!("{} Input", conf::NAME),
            data.target.image,
            data.target.extent,
            None,
        );

        let data = Self {
            descriptors,
            input_image,
            sampler: Sampler::create(ctx, conf::NAME.to_owned()),
        };
        data.bind_to_descriptor_sets(ctx);
        data
    }

    fn create_descriptors(ctx: &Context) -> Descriptors {
        firestorm::profile_method!(create_descriptors);

        let layout = {
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            let info =
                vk::DescriptorSetLayoutCreateInfo::default().bindings(slice::from_ref(&binding));
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let num_frames = ctx.surface.config.image_count;
            let size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(num_frames);
            let info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(slice::from_ref(&size))
                .max_sets(num_frames);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let layouts = vec![layout; ctx.surface.config.image_count as usize];
            let info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            unsafe {
                ctx.allocate_descriptor_sets(&info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Descriptors { layout, pool, sets }
    }

    fn bind_to_descriptor_sets(&self, ctx: &Context) {
        firestorm::profile_method!(bind_to_descriptor_sets);

        for &set in &self.descriptors.sets {
            let rendered_image_info = vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(self.input_image.view)
                .sampler(*self.sampler);

            let writes = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&rendered_image_info));

            unsafe {
                ctx.update_descriptor_sets(slice::from_ref(&writes), &[]);
            }
        }
    }
}

impl<const INPUT_FORMAT: image::Format, const OUTPUT_FORMAT: image::Format>
    Pipeline<INPUT_FORMAT, OUTPUT_FORMAT>
{
    pub fn create(ctx: &Context, data: &super::Data<INPUT_FORMAT>) -> Self {
        firestorm::profile_method!(create);

        let data = Data::create(ctx, data);

        let (layout, pipeline) = Self::create_pipeline(ctx, data.descriptors.layout);

        let descriptor_sets = data.descriptors.sets.iter().copied().map(|a| [a]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            conf::NAME.to_owned(),
            descriptor_sets,
            layout,
            pipeline,
            ctx.queues.graphics(),
            ctx.surface.config.image_count as _,
        );

        Self { data, pipeline }
    }

    fn create_pipeline(
        ctx: &Context,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        firestorm::profile_method!(create_pipeline);

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

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport_info = vk::PipelineViewportStateCreateInfo::default();

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .line_width(1.0)
            .front_face(vk::FrontFace::CLOCKWISE)
            .cull_mode(vk::CullModeFlags::BACK);

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

        let dynamic_states = [
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
        ];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(slice::from_ref(&descriptor_set_layout));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let color_formats = [OUTPUT_FORMAT.into()];
        let mut rendering_info =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);

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
            .dynamic_state(&dynamic_state_info)
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

    pub fn run(
        &self,
        ctx: &Context,
        idx: usize,
        sync_info: &SyncInfo,
        output_to: &image::Image<{ OUTPUT_FORMAT }>,
    ) {
        firestorm::profile_method!(run);

        let commands = self.pipeline.begin_pipeline(ctx, idx);

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(output_to.view)
            .image_layout(vk::ImageLayout::GENERAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)];

        let rendering_info = vk::RenderingInfo::default()
            .render_area(output_to.extent.into())
            .layer_count(1)
            .color_attachments(&color_attachments);

        unsafe {
            output_to.transition_layout(
                ctx,
                commands.buffer,
                &image::BarrierInfo {
                    layout: vk::ImageLayout::UNDEFINED,
                    stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    access: vk::AccessFlags::empty(),
                },
                &image::BarrierInfo::COLOR_ATTACHMENT,
            );

            ctx.cmd_begin_rendering(commands.buffer, &rendering_info);

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
                &self.pipeline.descriptor_sets[idx],
                &[],
            );

            let viewport = vk::Viewport::default()
                .width(ctx.surface.config.extent.width as f32)
                .height(ctx.surface.config.extent.height as f32)
                .max_depth(1.0);

            let scissor = vk::Rect2D::default().extent(ctx.surface.config.extent);

            ctx.cmd_set_viewport_with_count(commands.buffer, slice::from_ref(&viewport));
            ctx.cmd_set_scissor_with_count(commands.buffer, slice::from_ref(&scissor));
            ctx.cmd_draw(commands.buffer, 3, 1, 0, 0);

            ctx.cmd_end_rendering(commands.buffer);

            output_to.transition_layout(
                ctx,
                commands.buffer,
                &image::BarrierInfo::COLOR_ATTACHMENT,
                &image::BarrierInfo::PRESENTATION,
            );
        }

        self.pipeline.submit_pipeline(ctx, idx, sync_info);
    }
}

impl<const INPUT_FORMAT: image::Format, const OUTPUT_FORMAT: image::Format> Destroy<Context>
    for Pipeline<INPUT_FORMAT, OUTPUT_FORMAT>
{
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.pipeline.destroy_with(ctx);
        self.data.destroy_with(ctx);
    }
}

impl<const FORMAT: image::Format> Destroy<Context> for Data<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.input_image.destroy_with(ctx);
        self.sampler.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}

impl<const INPUT_FORMAT: image::Format, const OUTPUT_FORMAT: image::Format> Deref
    for Pipeline<INPUT_FORMAT, OUTPUT_FORMAT>
{
    type Target = Data<INPUT_FORMAT>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
