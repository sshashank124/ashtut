use std::ffi::CString;

use ash::vk;

use crate::util;

use super::{buffer::Buffer, context::Context, Destroy};

pub struct ShaderBindingTable {
    pub buffer: Buffer,
    pub raygen_region: vk::StridedDeviceAddressRegionKHR,
    pub misses_region: vk::StridedDeviceAddressRegionKHR,
    pub closest_hits_region: vk::StridedDeviceAddressRegionKHR,
    pub call_region: vk::StridedDeviceAddressRegionKHR,
}

pub struct RayTracingShaders {
    module: vk::ShaderModule,
    raygen: CString,
    misses: Vec<CString>,
    closest_hits: Vec<CString>,
}

impl ShaderBindingTable {
    pub fn create(
        ctx: &mut Context,
        mut rt_shaders: RayTracingShaders,
        pipeline: vk::Pipeline,
    ) -> Self {
        let (handle_size, handle_alignment, base_alignment) = {
            let props = &ctx.physical_device.properties.ray_tracing_pipeline;
            (
                props.shader_group_handle_size as _,
                props.shader_group_handle_alignment as _,
                props.shader_group_base_alignment as _,
            )
        };
        let handle_size_aligned = util::align_to(handle_size, handle_alignment);

        /* prepare regions */

        let mut raygen_region = vk::StridedDeviceAddressRegionKHR::builder()
            .stride(util::align_to(handle_size, base_alignment) as _)
            .size(util::align_to(handle_size, base_alignment) as _)
            .build();
        let mut misses_region = vk::StridedDeviceAddressRegionKHR::builder()
            .stride(handle_size_aligned as _)
            .size(util::align_to(
                rt_shaders.misses.len() * handle_size_aligned,
                base_alignment,
            ) as _)
            .build();
        let mut closest_hits_region = vk::StridedDeviceAddressRegionKHR::builder()
            .stride(handle_size_aligned as _)
            .size(util::align_to(
                rt_shaders.closest_hits.len() * handle_size_aligned,
                base_alignment,
            ) as _)
            .build();
        let call_region = vk::StridedDeviceAddressRegionKHR::default();

        /* fill shader binding table buffer */

        let handles_data = unsafe {
            ctx.ext
                .ray_tracing
                .get_ray_tracing_shader_group_handles(
                    pipeline,
                    0,
                    rt_shaders.num_stages() as _,
                    rt_shaders.num_stages() * handle_size,
                )
                .expect("Failed to get ray tracing shader group handles")
        };
        let mut handles = handles_data.chunks_exact(handle_size);

        let mut table = vec![
            0;
            (raygen_region.size + misses_region.size + closest_hits_region.size + call_region.size)
                as _
        ];
        table[..handle_size].copy_from_slice(handles.next().unwrap());
        table[raygen_region.size as usize..]
            .chunks_exact_mut(handle_size_aligned)
            .take(rt_shaders.misses.len())
            .zip(&mut handles)
            .for_each(|(dst, src)| dst[..handle_size].copy_from_slice(src));
        table[(raygen_region.size + misses_region.size) as usize..]
            .chunks_exact_mut(handle_size_aligned)
            .take(rt_shaders.closest_hits.len())
            .zip(handles)
            .for_each(|(dst, src)| dst[..handle_size].copy_from_slice(src));

        let buffer = Buffer::create_with_data(
            ctx,
            "Shader Binding Table",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR,
                ..Default::default()
            },
            &table,
        );

        /* update region addresses */

        let buffer_address = buffer.get_device_address(ctx);
        raygen_region.device_address = buffer_address;
        misses_region.device_address = raygen_region.device_address + raygen_region.size;
        closest_hits_region.device_address = misses_region.device_address + misses_region.size;

        unsafe { rt_shaders.destroy_with(ctx) };

        Self {
            buffer,
            raygen_region,
            misses_region,
            closest_hits_region,
            call_region,
        }
    }
}

impl RayTracingShaders {
    pub fn new(
        ctx: &Context,
        shader_file: &str,
        raygen: &str,
        misses: &[&str],
        closest_hits: &[&str],
    ) -> Self {
        let module = ctx.create_shader_module_from_file(shader_file);
        let raygen = util::cstring(raygen);
        let misses = misses.iter().copied().map(util::cstring).collect();
        let closest_hits = closest_hits.iter().copied().map(util::cstring).collect();

        Self {
            module,
            raygen,
            misses,
            closest_hits,
        }
    }

    pub fn stages_create_infos(&self) -> Vec<vk::PipelineShaderStageCreateInfo> {
        let mut stages = Vec::with_capacity(self.num_stages());

        stages.push(
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(self.module)
                .name(&self.raygen)
                .build(),
        );

        for miss in &self.misses {
            stages.push(
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::MISS_KHR)
                    .module(self.module)
                    .name(miss)
                    .build(),
            );
        }

        for closest_hit in &self.closest_hits {
            stages.push(
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                    .module(self.module)
                    .name(closest_hit)
                    .build(),
            );
        }

        stages
    }

    pub fn groups_create_infos(&self) -> Vec<vk::RayTracingShaderGroupCreateInfoKHR> {
        let mut groups = Vec::with_capacity(self.num_stages());

        groups.push(
            vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
                .build(),
        );

        for i in 0..self.misses.len() {
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::builder()
                    .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                    .general_shader((1 + i) as _)
                    .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR)
                    .build(),
            );
        }

        for i in 0..self.closest_hits.len() {
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::builder()
                    .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader((1 + self.misses.len() + i) as _)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR)
                    .build(),
            );
        }

        groups
    }

    fn num_stages(&self) -> usize {
        1 + self.misses.len() + self.closest_hits.len()
    }
}

impl Destroy<Context> for ShaderBindingTable {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.buffer.destroy_with(ctx);
    }
}

impl Destroy<Context> for RayTracingShaders {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_shader_module(self.module, None);
    }
}
