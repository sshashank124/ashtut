use ash::vk;

use super::instance::Instance;

pub struct Properties {
    // Core
    pub v_1_0: Box<vk::PhysicalDeviceProperties2>,
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
        let mut ray_tracing_pipeline =
            Box::<vk::PhysicalDeviceRayTracingPipelinePropertiesKHR>::default();

        let v_1_0 = vk::PhysicalDeviceProperties2::builder()
            .push_next(ray_tracing_pipeline.as_mut())
            .build()
            .into();

        Self {
            v_1_0,
            ray_tracing_pipeline,
        }
    }
}
