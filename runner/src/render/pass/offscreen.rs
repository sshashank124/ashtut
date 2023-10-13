use std::ops::Deref;

use ash::vk;
use shared::{bytemuck, UniformObjects, Vertex};

use crate::{
    gpu::{
        buffer::Buffer,
        commands::Commands,
        context::Context,
        descriptors::Descriptors,
        framebuffer::Framebuffers,
        image::{format, Image},
        pipeline::Pipeline,
        render_pass::RenderPass,
        scope::Scope,
        texture::Texture,
        uniforms::Uniforms,
        Descriptions, Destroy,
    },
    model::Model,
};

use super::Pass;

pub mod conf {
    pub const FRAME_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        width: 1024,
        height: 768,
    };
    pub const SHADER_FILE: &str = env!("raster.spv");
    pub const VERTEX_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
    pub const FRAGMENT_SHADER_ENTRY_POINT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };
}

pub type OffscreenPass = Pass<Offscreen>;

pub struct Offscreen {
    uniforms: Uniforms,
    model: Model,
    vertex_index_buffer: Buffer,
    texture: Texture<{ format::COLOR }>,
}

impl Offscreen {
    pub fn create(ctx: &mut Context, setup_scope: &mut Scope) -> Self {
        let descriptors = Self::create_descriptors(ctx);
        let render_pass = Self::create_render_pass(ctx);
        let pipeline = Self::create_pipeline(
            ctx,
            *render_pass,
            descriptors.layout,
            conf::FRAME_RESOLUTION,
        );

        let commands = Commands::create_on_queue(ctx, ctx.queues.graphics());

        let pass = Pass {
            descriptors,
            render_pass,
            pipeline,
        };

        let uniforms = Uniforms::create(ctx);
        let model = Model::demo_viking_room();
        let vertex_index_buffer = Self::init_vertex_index_buffer(ctx, setup_scope, &model);
        let texture = Self::init_texture(ctx, setup_scope, &model);

        let pass = Self {
            uniforms,
            model,
            vertex_index_buffer,
            texture,
        };

        pass.bind_descriptors(ctx);

        pass
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
            let layouts = [layout];
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
                .format(format::HDR)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
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
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
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
        extent: vk::Extent2D,
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

        let vertex_bindings_description = Vertex::bindings_description();
        let vertex_attributes_description = Vertex::attributes_description();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_bindings_description)
            .vertex_attribute_descriptions(&vertex_attributes_description);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewports = [vk::Viewport::builder()
            .width(extent.width as f32)
            .height(extent.height as f32)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder().extent(extent).build()];

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
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
            .build()];

        let pipeline = unsafe {
            ctx.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create pipeline")[0]
        };

        unsafe { ctx.destroy_shader_module(shader_module, None) };

        Pipeline { layout, pipeline }
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
    ) -> Texture<{ format::COLOR }> {
        let image = Image::create_from_image(ctx, setup_scope, "Texture", &model.texture);
        Texture::from_image(ctx, image)
    }

    pub fn bind_descriptors(&self, ctx: &Context) {
        let buffer_infos = [vk::DescriptorBufferInfo::builder()
            .buffer(*self.uniforms.buffer)
            .range(vk::WHOLE_SIZE)
            .build()];

        let sampled_image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.texture.image.view)
            .sampler(*self.texture.sampler)
            .build()];

        let writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.pass.descriptors.sets[0])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build(),
            vk::WriteDescriptorSet::builder()
                .dst_set(self.pass.descriptors.sets[0])
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&sampled_image_info)
                .build(),
        ];

        unsafe {
            ctx.update_descriptor_sets(&writes, &[]);
        }
    }

    pub fn draw(
        &mut self,
        ctx: &Context,
        render_target: &Framebuffers<{ format::HDR }>,
        uniforms: &UniformObjects,
    ) {
        self.uniforms.update(uniforms);
        self.commands.reset(ctx);
        self.record_commands(ctx, render_target);
        let submit_info = vk::SubmitInfo::builder();
        self.commands.submit(ctx, &submit_info, None);
    }

    fn record_commands(&self, ctx: &Context, render_target: &Framebuffers<{ format::HDR }>) {
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
            .render_area(vk::Rect2D::builder().extent(conf::FRAME_RESOLUTION).build())
            .framebuffer(render_target.framebuffers[0])
            .clear_values(&clear_values)
            .build();

        self.commands.begin_recording(ctx);

        unsafe {
            ctx.cmd_begin_render_pass(
                self.commands.buffer,
                &pass_info,
                vk::SubpassContents::INLINE,
            );

            ctx.cmd_bind_pipeline(
                self.commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *self.pass.pipeline,
            );

            let vertex_buffers = [*self.vertex_index_buffer];
            ctx.cmd_bind_vertex_buffers(self.commands.buffer, 0, &vertex_buffers, &[0]);

            ctx.cmd_bind_index_buffer(
                self.commands.buffer,
                *self.vertex_index_buffer,
                self.model.mesh.vertex_data_size() as u64,
                vk::IndexType::UINT32,
            );

            let descriptor_sets = [self.pass.descriptors.sets[0]];
            ctx.cmd_bind_descriptor_sets(
                self.commands.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pass.pipeline.layout,
                0,
                &descriptor_sets,
                &[],
            );

            ctx.cmd_draw_indexed(
                self.commands.buffer,
                self.model.mesh.indices.len() as u32,
                1,
                0,
                0,
                0,
            );

            ctx.cmd_end_render_pass(self.commands.buffer);
        }

        self.commands.finish_recording(ctx);
    }
}

impl Destroy<Context> for Offscreen {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.texture.destroy_with(ctx);
        self.vertex_index_buffer.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);

        self.commands.destroy_with(ctx);
        self.pass.destroy_with(ctx);
    }
}

impl Deref for Offscreen {
    type Target = Pass;
    fn deref(&self) -> &Self::Target {
        &self.pass
    }
}
