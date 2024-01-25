use ash::vk;

use super::instance::Instance;

pub struct Properties {
    // Core
    pub v_1_0: Box<vk::PhysicalDeviceProperties2>,
    // Acceleration Structure
    pub acceleration_structure: Box<vk::PhysicalDeviceAccelerationStructurePropertiesKHR>,
    // Ray Tracing
    pub ray_tracing_pipeline: Box<vk::PhysicalDeviceRayTracingPipelinePropertiesKHR>,
}

impl Properties {
    pub fn get_supported(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut supported = Self::default();
        unsafe {
            instance.get_physical_device_properties2(physical_device, &mut supported.v_1_0);
        }
        supported
    }
}

impl Default for Properties {
    fn default() -> Self {
        let mut acceleration_structure =
            Box::<vk::PhysicalDeviceAccelerationStructurePropertiesKHR>::default();

        let mut ray_tracing_pipeline =
            Box::<vk::PhysicalDeviceRayTracingPipelinePropertiesKHR>::default();

        let v_1_0 = vk::PhysicalDeviceProperties2::builder()
            .push_next(acceleration_structure.as_mut())
            .push_next(ray_tracing_pipeline.as_mut())
            .build()
            .into();

        Self {
            v_1_0,
            acceleration_structure,
            ray_tracing_pipeline,
        }
    }
}
