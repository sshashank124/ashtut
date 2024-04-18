use std::slice;

use ash::vk;

use shared::inputs;

use crate::{
    context::Context,
    image, pipeline,
    shader_binding_table::{RayTracingShaders, ShaderBindingTable},
    sync_info::SyncInfo,
    Destroy,
};

use super::common;

pub mod conf {
    pub const NAME: &str = "Pathtracer";
    pub const SHADER_RAY_GENERATION: &str = env!("pathtracer.rgen.glsl");
    pub const SHADER_MISSES: &[&str] = &[env!("pathtracer.rmiss.glsl")];
    pub const SHADER_CLOSEST_HITS: &[&str] = &[env!("pathtracer.rchit.glsl")];
}

pub struct Pipeline {
    pipeline: pipeline::Pipeline<1>,
    shader_binding_table: ShaderBindingTable,
}

impl Pipeline {
    pub fn create<const FORMAT: image::Format>(ctx: &Context, data: &common::Data<FORMAT>) -> Self {
        let ray_tracing_shaders = RayTracingShaders::new(
            ctx,
            conf::SHADER_RAY_GENERATION,
            conf::SHADER_MISSES,
            conf::SHADER_CLOSEST_HITS,
        );

        let (layout, pipeline) = Self::create_pipeline(ctx, data, &ray_tracing_shaders);

        let shader_binding_table = ShaderBindingTable::create(ctx, ray_tracing_shaders, pipeline);

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

        Self {
            pipeline,
            shader_binding_table,
        }
    }

    fn create_pipeline<const FORMAT: image::Format>(
        ctx: &Context,
        data: &common::Data<FORMAT>,
        ray_tracing_shaders: &RayTracingShaders,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
        let push_constant_ranges = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
            offset: 0,
            size: std::mem::size_of::<inputs::PathtracerConstants>() as _,
        };

        let layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(slice::from_ref(&data.descriptors.layout))
            .push_constant_ranges(slice::from_ref(&push_constant_ranges));

        let layout = unsafe {
            ctx.create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        let stages = ray_tracing_shaders.stages_create_infos();
        let groups = ray_tracing_shaders.groups_create_infos();
        let create_info = vk::RayTracingPipelineCreateInfoKHR::default()
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

    pub fn run<const FORMAT: image::Format>(
        &self,
        ctx: &Context,
        data: &common::Data<FORMAT>,
        frame: u32,
        sync_info: &SyncInfo,
    ) {
        let commands = self.pipeline.begin_pipeline(ctx, 0);

        let push_constants = inputs::PathtracerConstants { frame };

        unsafe {
            ctx.cmd_bind_pipeline(
                commands.buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                *self.pipeline,
            );

            ctx.cmd_push_constants(
                commands.buffer,
                self.pipeline.layout,
                vk::ShaderStageFlags::RAYGEN_KHR,
                0,
                bytemuck::bytes_of(&push_constants),
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
                data.target.extent.width,
                data.target.extent.height,
                1,
            );
        }

        self.pipeline.submit_pipeline(ctx, 0, sync_info);
    }
}

impl Destroy<Context> for Pipeline {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        self.shader_binding_table.destroy_with(ctx);
        self.pipeline.destroy_with(ctx);
    }
}
