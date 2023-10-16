use ash::vk;
use shared::Vertex;

use crate::gpu::{
    acceleration_structure::AccelerationStructures,
    commands::Commands,
    context::Context,
    descriptors::Descriptors,
    model::Model,
    pipeline::{Contents, RayTrace},
    scope::OneshotScope,
    uniforms::Uniforms,
    Descriptions, Destroy,
};

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

pub struct PathTracer {
    pub uniforms: Uniforms,
    models: Vec<Model>,
    accels: AccelerationStructures,
}

impl PathTracer {
    pub fn create(ctx: &mut Context) -> Self {
        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.graphics());

        let uniforms = Uniforms::create(ctx);
        let models = vec![Model::demo_viking_room(ctx, &mut init_scope)];

        init_scope.finish(ctx);

        let accels = AccelerationStructures::build(ctx, &models);

        Self {
            uniforms,
            models,
            accels,
        }
    }
}

impl Contents<RayTrace> for PathTracer {
    fn num_command_sets(_: &Context) -> u32 {
        1
    }

    fn render_area(_: &Context) -> vk::Rect2D {
        vk::Rect2D {
            extent: conf::FRAME_RESOLUTION,
            ..Default::default()
        }
    }

    fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
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
                    .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .build(),
                vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
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

    fn create_specialization(ctx: &Context) -> RayTrace {
        RayTrace::create()
    }

    fn create_pipeline(
        ctx: &Context,
        spec: &RayTrace,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
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
            .width(conf::FRAME_RESOLUTION.width as f32)
            .height(conf::FRAME_RESOLUTION.height as f32)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder().extent(conf::FRAME_RESOLUTION).build()];

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
            .build()];

        let pipeline = unsafe {
            ctx.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create pipeline")[0]
        };

        unsafe { ctx.destroy_shader_module(shader_module, None) };

        (layout, pipeline)
    }

    fn bind_descriptors(&self, ctx: &Context, descriptors: &Descriptors) {
        let tlas = [*self.accels.tlas];
        let mut accel_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(&tlas);

        let sampled_image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.models[0].texture.image.view)
            .sampler(*self.models[0].texture.sampler)
            .build()];

        let writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptors.sets[0])
                .push_next(&mut accel_info)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .build(),
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptors.sets[0])
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(&sampled_image_info)
                .build(),
        ];

        unsafe {
            ctx.update_descriptor_sets(&writes, &[]);
        }
    }

    fn record_commands(&self, ctx: &Context, commands: &Commands) {
        unsafe {
            let vertex_buffers = [*self.models[0].vertex_index_buffer];
            ctx.cmd_bind_vertex_buffers(commands.buffer, 0, &vertex_buffers, &[0]);

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
        }
    }
}

impl Destroy<Context> for PathTracer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.accels.destroy_with(ctx);
        self.models.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);
    }
}
