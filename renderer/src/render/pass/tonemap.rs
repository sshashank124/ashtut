use std::{ops::Deref, slice};

use ash::vk;

use crate::gpu::{
    context::Context,
    descriptors::Descriptors,
    framebuffers::{self, Framebuffers},
    image, pipeline,
    sampler::Sampler,
    sync_info::SyncInfo,
    Destroy,
};

use super::common;

mod conf {
    pub const SHADER_VERT: &str = env!("tonemap.vert.glsl");
    pub const SHADER_FRAG: &str = env!("tonemap.frag.glsl");
}

pub struct Data {
    descriptors: Descriptors,
    input_image: image::Image<{ image::Format::Hdr }>,
    sampler: Sampler,
}

pub struct Pipeline {
    data: Data,
    pub render_pass: vk::RenderPass,
    pipeline: pipeline::Pipeline<1>,
}

impl Data {
    pub fn create(ctx: &Context, common: &common::Data) -> Self {
        let descriptors = Self::create_descriptors(ctx);

        let input_image = image::Image::new(ctx, common.target.image, None);

        let data = Self {
            descriptors,
            input_image,
            sampler: Sampler::create(ctx),
        };
        data.bind_to_descriptor_sets(ctx);
        data
    }

    fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            let info =
                vk::DescriptorSetLayoutCreateInfo::builder().bindings(slice::from_ref(&binding));
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let num_frames = ctx.surface.config.image_count;
            let size = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(num_frames);
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(slice::from_ref(&size))
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

    fn bind_to_descriptor_sets(&self, ctx: &Context) {
        for &set in &self.descriptors.sets {
            let rendered_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(self.input_image.view)
                .sampler(*self.sampler);

            let writes = vk::WriteDescriptorSet::builder()
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

impl Pipeline {
    pub fn create(ctx: &Context, common: &common::Data) -> Self {
        let data = Data::create(ctx, common);

        let render_pass = Self::create_render_pass(ctx);
        let (layout, pipeline) = Self::create_pipeline(ctx, render_pass, data.descriptors.layout);

        let descriptor_sets = data.descriptors.sets.iter().copied().map(|a| [a]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            descriptor_sets,
            layout,
            pipeline,
            ctx.queues.graphics(),
            ctx.surface.config.image_count as _,
        );

        Self {
            data,
            render_pass,
            pipeline,
        }
    }

    fn create_render_pass(ctx: &Context) -> vk::RenderPass {
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
                .format(image::Format::Depth.into())
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
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
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
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ);

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
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let shader_module_vert = ctx.create_shader_module_from_file(conf::SHADER_VERT);
        let shader_module_frag = ctx.create_shader_module_from_file(conf::SHADER_FRAG);
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader_module_vert)
                .name(crate::util::SHADER_ENTRY_POINT)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader_module_frag)
                .name(crate::util::SHADER_ENTRY_POINT)
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

        let dynamic_states = [
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
        ];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

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
            .render_pass(render_pass)
            .dynamic_state(&dynamic_state_info);

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
        output_to: &Framebuffers<{ image::Format::Swapchain }>,
    ) {
        let commands = self.pipeline.begin_pipeline(ctx, idx);

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .render_area(ctx.surface.config.extent.into())
            .framebuffer(output_to.framebuffers[idx])
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
                &self.pipeline.descriptor_sets[idx],
                &[],
            );

            let viewport = vk::Viewport::builder()
                .width(ctx.surface.config.extent.width as f32)
                .height(ctx.surface.config.extent.height as f32)
                .max_depth(1.0);

            let scissor = vk::Rect2D::builder().extent(ctx.surface.config.extent);

            ctx.cmd_set_viewport_with_count(commands.buffer, slice::from_ref(&viewport));
            ctx.cmd_set_scissor_with_count(commands.buffer, slice::from_ref(&scissor));
            ctx.cmd_draw(commands.buffer, 3, 1, 0, 0);

            ctx.cmd_end_render_pass(commands.buffer);
        }

        self.pipeline.submit_pipeline(ctx, idx, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.pipeline.destroy_with(ctx);
        ctx.destroy_render_pass(self.render_pass, None);
        self.data.destroy_with(ctx);
    }
}

impl Destroy<Context> for Data {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.input_image.destroy_with(ctx);
        self.sampler.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}

impl Deref for Pipeline {
    type Target = Data;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
