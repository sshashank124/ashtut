use std::{
    ops::{Deref, DerefMut},
    slice,
};

use ash::vk;

use crate::gpu::{
    acceleration_structure::AccelerationStructures,
    context::Context,
    descriptors::Descriptors,
    pipeline,
    shader_binding_table::{RayTracingShaders, ShaderBindingTable},
    sync_info::SyncInfo,
    Destroy,
};

use super::common;

pub mod conf {
    pub const SHADER_FILE: &str = env!("pathtracer.spv");
    pub const STAGE_RAY_GENERATION: &str = "ray_generation";
    pub const STAGES_MISS: &[&str] = &["miss"];
    pub const STAGES_CLOSEST_HIT: &[&str] = &["closest_hit"];
}

pub struct Data {
    pub descriptors: Descriptors,
    accel: AccelerationStructures,
}

pub struct Pipeline {
    data: Data,
    pipeline: pipeline::Pipeline<2>,
    shader_binding_table: ShaderBindingTable,
}

impl Data {
    pub fn create(ctx: &mut Context, common: &common::Data) -> Self {
        let descriptors = Self::create_descriptors(ctx);

        let accel = AccelerationStructures::build(ctx, &common.scene);

        let data = Self { descriptors, accel };
        data.bind_to_descriptor_sets(ctx, common);
        data
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

    fn bind_to_descriptor_sets(&self, ctx: &Context, common: &common::Data) {
        let mut accel_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(slice::from_ref(&self.accel.tlas));

        let target_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(common.target.view);

        for &set in &self.descriptors.sets {
            let mut accel_write = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .push_next(&mut accel_info)
                .build();
            accel_write.descriptor_count = 1;

            let writes = [
                accel_write,
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
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
}

impl Pipeline {
    pub fn create(ctx: &mut Context, common: &common::Data) -> Self {
        let data = Data::create(ctx, common);

        let ray_tracing_shaders = RayTracingShaders::new(
            ctx,
            conf::SHADER_FILE,
            conf::STAGE_RAY_GENERATION,
            conf::STAGES_MISS,
            conf::STAGES_CLOSEST_HIT,
        );

        let (layout, pipeline) = Self::create_pipeline(
            ctx,
            &[common.descriptors.layout, data.descriptors.layout],
            &ray_tracing_shaders,
        );

        let shader_binding_table = ShaderBindingTable::create(ctx, ray_tracing_shaders, pipeline);

        let descriptor_sets = common
            .descriptors
            .sets
            .iter()
            .copied()
            .zip(data.descriptors.sets.iter().copied())
            .map(|(a, b)| [a, b]);

        let pipeline = pipeline::Pipeline::new(
            ctx,
            descriptor_sets,
            layout,
            pipeline,
            ctx.queues.graphics(),
            1,
        );

        Self {
            data,
            pipeline,
            shader_binding_table,
        }
    }

    fn create_pipeline(
        ctx: &Context,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        ray_tracing_shaders: &RayTracingShaders,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(descriptor_set_layouts);

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
                &self.pipeline.descriptor_sets[0],
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
        self.accel.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
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
