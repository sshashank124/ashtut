use ash::vk;

use crate::{buffer::Buffer, context::Context, memory, Destroy};

pub struct ShaderBindingTable {
    pub buffer: Buffer,
    pub raygen_region: vk::StridedDeviceAddressRegionKHR,
    pub misses_region: vk::StridedDeviceAddressRegionKHR,
    pub closest_hits_region: vk::StridedDeviceAddressRegionKHR,
    pub call_region: vk::StridedDeviceAddressRegionKHR,
}

pub struct RayTracingShaders {
    raygen: vk::ShaderModule,
    misses: Vec<vk::ShaderModule>,
    closest_hits: Vec<vk::ShaderModule>,
}

impl ShaderBindingTable {
    pub fn create(
        ctx: &Context,
        mut rt_shaders: RayTracingShaders,
        pipeline: vk::Pipeline,
    ) -> Self {
        firestorm::profile_method!(create);

        let (handle_size, handle_alignment, base_alignment) = {
            let props = &ctx.physical_device.properties.ray_tracing_pipeline;
            (
                props.shader_group_handle_size as _,
                props.shader_group_handle_alignment as _,
                props.shader_group_base_alignment as _,
            )
        };
        let handle_size_aligned = memory::align_to(handle_size, handle_alignment);

        /* prepare regions */

        let mut raygen_region = vk::StridedDeviceAddressRegionKHR::default()
            .stride(memory::align_to(handle_size, base_alignment) as _)
            .size(memory::align_to(handle_size, base_alignment) as _);
        let mut misses_region = vk::StridedDeviceAddressRegionKHR::default()
            .stride(handle_size_aligned as _)
            .size(memory::align_to(
                rt_shaders.misses.len() * handle_size_aligned,
                base_alignment,
            ) as _);
        let mut closest_hits_region = vk::StridedDeviceAddressRegionKHR::default()
            .stride(handle_size_aligned as _)
            .size(memory::align_to(
                rt_shaders.closest_hits.len() * handle_size_aligned,
                base_alignment,
            ) as _);
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
            "Shader Binding Table".to_owned(),
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
        raygen_file: &str,
        misses_files: &[&str],
        closest_hits_files: &[&str],
    ) -> Self {
        firestorm::profile_method!(new);

        let raygen = ctx.create_shader_module_from_file(raygen_file);
        ctx.set_debug_name(raygen, raygen_file);
        let misses = misses_files
            .iter()
            .map(|shader| {
                let module = ctx.create_shader_module_from_file(shader);
                ctx.set_debug_name(module, shader);
                module
            })
            .collect();
        let closest_hits = closest_hits_files
            .iter()
            .map(|shader| {
                let module = ctx.create_shader_module_from_file(shader);
                ctx.set_debug_name(module, shader);
                module
            })
            .collect();

        Self {
            raygen,
            misses,
            closest_hits,
        }
    }

    pub fn stages_create_infos(&self) -> Vec<vk::PipelineShaderStageCreateInfo> {
        let mut stages = Vec::with_capacity(self.num_stages());

        stages.push(
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(self.raygen)
                .name(crate::cstr!("main")),
        );

        for miss_shader in &self.misses {
            stages.push(
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::MISS_KHR)
                    .module(*miss_shader)
                    .name(crate::cstr!("main")),
            );
        }

        for closest_hit_shader in &self.closest_hits {
            stages.push(
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                    .module(*closest_hit_shader)
                    .name(crate::cstr!("main")),
            );
        }

        stages
    }

    pub fn groups_create_infos(&self) -> Vec<vk::RayTracingShaderGroupCreateInfoKHR> {
        let mut groups = Vec::with_capacity(self.num_stages());

        groups.push(
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR),
        );

        for i in 0..self.misses.len() {
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::default()
                    .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                    .general_shader((1 + i) as _)
                    .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR),
            );
        }

        for i in 0..self.closest_hits.len() {
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::default()
                    .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader((1 + self.misses.len() + i) as _)
                    .any_hit_shader(vk::SHADER_UNUSED_KHR)
                    .intersection_shader(vk::SHADER_UNUSED_KHR),
            );
        }

        groups
    }

    fn num_stages(&self) -> usize {
        1 + self.misses.len() + self.closest_hits.len()
    }
}

impl Destroy<Context> for ShaderBindingTable {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.buffer.destroy_with(ctx);
    }
}

impl Destroy<Context> for RayTracingShaders {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        ctx.destroy_shader_module(self.raygen, None);
        self.misses
            .iter()
            .for_each(|&module| ctx.destroy_shader_module(module, None));
        self.closest_hits
            .iter()
            .for_each(|&module| ctx.destroy_shader_module(module, None));
    }
}
