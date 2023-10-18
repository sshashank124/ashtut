use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::gpu::{
    acceleration_structure::AccelerationStructures,
    context::Context,
    descriptors::Descriptors,
    image::{format, Image},
    model::Model,
    pipeline,
    scope::OneshotScope,
    sync_info::SyncInfo,
    Destroy,
};

pub mod conf {
    const HEIGHT: u32 = 768;
    pub const FRAME_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        height: HEIGHT,
        width: (HEIGHT as f32 * super::super::super::conf::ASPECT_RATIO) as _,
    };

    pub const SHADER_FILE: &str = env!("pathtracer.spv");
    pub const STAGE_RAY_GENERATION: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"ray_generation\0") };
    pub const STAGE_MISS: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"miss\0") };
    pub const STAGE_CLOSEST_HIT: &std::ffi::CStr =
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"closest_hit\0") };
}

pub struct Data {
    models: Vec<Model>,
    accel: AccelerationStructures,
    target: Image<{ format::HDR }>,
}

pub struct Pipeline {
    data: Data,
    pipeline: pipeline::Pipeline,
}

impl Data {
    pub fn create(ctx: &mut Context) -> Self {
        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.graphics());
        let models = vec![Model::demo_viking_room(ctx, &mut init_scope)];
        let target = {
            let info = vk::ImageCreateInfo {
                extent: conf::FRAME_RESOLUTION.into(),
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                ..Default::default()
            };
            Image::create(ctx, "Pathtracer Target", &info)
        };
        init_scope.finish(ctx);

        let accel = AccelerationStructures::build(ctx, &models);

        Self {
            models,
            accel,
            target,
        }
    }

    pub fn bind_to_descriptors(&self, ctx: &Context, descriptors: &Descriptors) {
        let tlas = [*self.accel.tlas];
        let mut accel_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(&tlas);

        let target_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.target.view)
            .build()];

        let writes = [
            vk::WriteDescriptorSet {
                descriptor_count: 1,
                ..vk::WriteDescriptorSet::builder()
                    .dst_set(descriptors.sets[0])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .push_next(&mut accel_info)
                    .build()
            },
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptors.sets[0])
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(&target_info)
                .build(),
        ];

        unsafe {
            ctx.update_descriptor_sets(&writes, &[]);
        }
    }
}

impl Pipeline {
    pub fn create(ctx: &mut Context, data: Data) -> Self {
        let descriptors = Self::create_descriptors(ctx);
        let (layout, pipeline) = Self::create_pipeline(ctx, descriptors.layout);

        data.bind_to_descriptors(ctx, &descriptors);

        let pipeline =
            pipeline::Pipeline::new(ctx, descriptors, layout, pipeline, ctx.queues.graphics(), 1);

        Self { data, pipeline }
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

    fn create_pipeline(
        ctx: &Context,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let shader_module = ctx.create_shader_module_from_file(conf::SHADER_FILE);
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(shader_module)
                .name(conf::STAGE_RAY_GENERATION)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::MISS_KHR)
                .module(shader_module)
                .name(conf::STAGE_MISS)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                .module(shader_module)
                .name(conf::STAGE_CLOSEST_HIT)
                .build(),
        ];

        let shader_groups = [
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(1)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(2)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
        ];

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let create_infos = [vk::RayTracingPipelineCreateInfoKHR::builder()
            .stages(&shader_stages)
            .groups(&shader_groups)
            .max_pipeline_ray_recursion_depth(1)
            .layout(layout)
            .build()];

        let pipeline = unsafe {
            ctx.ext
                .ray_tracing
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    &create_infos,
                    None,
                )
                .expect("Failed to create pipeline")[0]
        };

        unsafe { ctx.destroy_shader_module(shader_module, None) };

        (layout, pipeline)
    }

    pub fn run(&self, ctx: &Context, idx: usize, sync_info: &SyncInfo) {
        let commands = self.pipeline.begin_pipeline(ctx, idx);

        unsafe {
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
                self.pipeline.descriptor_set(idx),
                &[],
            );

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

            ctx.cmd_end_render_pass(commands.buffer);
        }

        self.pipeline.submit_pipeline(ctx, idx, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.pipeline.destroy_with(ctx);
        self.data.destroy_with(ctx);
    }
}

impl Destroy<Context> for Data {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.target.destroy_with(ctx);
        self.accel.destroy_with(ctx);
        self.models.destroy_with(ctx);
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
