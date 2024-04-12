use ash::vk;

use super::instance::Instance;

pub struct Properties {
    pub v_1_0: V10Properties,
    pub acceleration_structure: AccelerationStructureProperties,
    pub ray_tracing_pipeline: RayTracingPipelineProperties,
}

pub struct V10Properties {
    pub limits: vk::PhysicalDeviceLimits,
}
pub struct AccelerationStructureProperties {
    pub min_acceleration_structure_scratch_offset_alignment: u32,
}
pub struct RayTracingPipelineProperties {
    pub shader_group_base_alignment: u32,
    pub shader_group_handle_alignment: u32,
    pub shader_group_handle_size: u32,
}

impl Properties {
    pub fn get_supported(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut ray_tracing_pipeline = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
        let mut acceleration_structure =
            vk::PhysicalDeviceAccelerationStructurePropertiesKHR::default();

        let mut v_1_0 = vk::PhysicalDeviceProperties2::default()
            .push_next(&mut ray_tracing_pipeline)
            .push_next(&mut acceleration_structure);

        unsafe { instance.get_physical_device_properties2(physical_device, &mut v_1_0) };

        Self {
            v_1_0: V10Properties::from(v_1_0.properties),
            acceleration_structure: AccelerationStructureProperties::from(acceleration_structure),
            ray_tracing_pipeline: RayTracingPipelineProperties::from(ray_tracing_pipeline),
        }
    }
}

impl From<vk::PhysicalDeviceProperties> for V10Properties {
    fn from(p: vk::PhysicalDeviceProperties) -> Self {
        Self { limits: p.limits }
    }
}

impl From<vk::PhysicalDeviceAccelerationStructurePropertiesKHR<'_>>
    for AccelerationStructureProperties
{
    fn from(p: vk::PhysicalDeviceAccelerationStructurePropertiesKHR) -> Self {
        Self {
            min_acceleration_structure_scratch_offset_alignment: p
                .min_acceleration_structure_scratch_offset_alignment,
        }
    }
}

impl From<vk::PhysicalDeviceRayTracingPipelinePropertiesKHR<'_>> for RayTracingPipelineProperties {
    fn from(p: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR) -> Self {
        Self {
            shader_group_base_alignment: p.shader_group_base_alignment,
            shader_group_handle_alignment: p.shader_group_handle_alignment,
            shader_group_handle_size: p.shader_group_handle_size,
        }
    }
}
