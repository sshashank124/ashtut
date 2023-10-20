use std::{
    ops::{Deref, DerefMut},
    slice,
};

use ash::vk;

use crate::gpu::{
    acceleration_structure::AccelerationStructures,
    context::Context,
    descriptors::Descriptors,
    image::{format, Image},
    model::Model,
    pipeline,
    scope::OneshotScope,
    shader_binding_table::{RayTracingShaders, ShaderBindingTable},
    sync_info::SyncInfo,
    Destroy,
};

pub mod conf {
    pub const SHADER_FILE: &str = env!("pathtracer.spv");
    pub const STAGE_RAY_GENERATION: &str = "ray_generation";
    pub const STAGES_MISS: &[&str] = &["miss"];
    pub const STAGES_CLOSEST_HIT: &[&str] = &["closest_hit"];
}

pub struct Data {
    pub target: Image<{ format::HDR }>,
    models: Vec<Model>,
    accel: AccelerationStructures,
}

pub struct Pipeline {
    data: Data,
    pipeline: pipeline::Pipeline,
    shader_binding_table: ShaderBindingTable,
}

impl Data {
    pub fn create(ctx: &mut Context, target: &Image<{ format::HDR }>) -> Self {
        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.graphics());
        let target = Image::new(ctx, target.image, format::HDR, None);
        let models = vec![Model::demo_viking_room(ctx, &mut init_scope)];
        init_scope.finish(ctx);

        let accel = AccelerationStructures::build(ctx, &models);

        Self {
            target,
            models,
            accel,
        }
    }

    pub fn bind_to_descriptors(&self, ctx: &Context, descriptors: &Descriptors) {
        let mut accel_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(slice::from_ref(&self.accel.tlas));

        let target_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.target.view);

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
                .image_info(slice::from_ref(&target_info))
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
        data.bind_to_descriptors(ctx, &descriptors);

        let ray_tracing_shaders = RayTracingShaders::new(
            ctx,
            conf::SHADER_FILE,
            conf::STAGE_RAY_GENERATION,
            conf::STAGES_MISS,
            conf::STAGES_CLOSEST_HIT,
        );
        let (layout, pipeline) =
            Self::create_pipeline(ctx, descriptors.layout, &ray_tracing_shaders);
        let shader_binding_table = ShaderBindingTable::create(ctx, ray_tracing_shaders, pipeline);

        let pipeline =
            pipeline::Pipeline::new(ctx, descriptors, layout, pipeline, ctx.queues.graphics(), 1);

        Self {
            data,
            pipeline,
            shader_binding_table,
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
        descriptor_set_layout: vk::DescriptorSetLayout,
        ray_tracing_shaders: &RayTracingShaders,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(slice::from_ref(&descriptor_set_layout));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let stages = ray_tracing_shaders.stages_create_infos();
        let groups = ray_tracing_shaders.groups_create_infos();
        let create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .stages(&stages)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(1)
            .layout(layout);

        let pipeline = unsafe {
            ctx.ext
                .ray_tracing
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    slice::from_ref(&create_info),
                    None,
                )
                .expect("Failed to create pipeline")[0]
        };

        (layout, pipeline)
    }

    pub fn run(&self, ctx: &Context, sync_info: &SyncInfo) {
        let commands = self.pipeline.begin_pipeline(ctx, 0);

        unsafe {
            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                *self.pipeline,
            );

            ctx.cmd_bind_descriptor_sets(
                commands.buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline.layout,
                0,
                slice::from_ref(&self.pipeline.descriptors.sets[0]),
                &[],
            );

            ctx.ext.ray_tracing.cmd_trace_rays(
                commands.buffer,
                &self.shader_binding_table.raygen_region,
                &self.shader_binding_table.misses_region,
                &self.shader_binding_table.closest_hits_region,
                &self.shader_binding_table.call_region,
                super::super::conf::FRAME_RESOLUTION.width,
                super::super::conf::FRAME_RESOLUTION.height,
                1,
            );
        }

        self.pipeline.submit_pipeline(ctx, 0, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.shader_binding_table.destroy_with(ctx);
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
