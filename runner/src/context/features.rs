use ash::vk;

use super::instance::Instance;

#[derive(Debug)]
pub struct Features {
    // Core
    pub v_1_0: Box<vk::PhysicalDeviceFeatures2>,
    pub v_1_1: Box<vk::PhysicalDeviceVulkan11Features>,
    pub v_1_2: Box<vk::PhysicalDeviceVulkan12Features>,
    // Acceleration Structure
    pub acceleration_structure: Box<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>,
    pub ray_tracing_pipeline: Box<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>,
}

impl Features {
    pub fn required() -> Self {
        let mut features = Self::default();
        features.v_1_2.vulkan_memory_model = 1;
        features
    }

    pub const fn supports_requirements(&self) -> bool {
        self.v_1_2.vulkan_memory_model > 0
    }

    pub fn get_supported(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut supported = Self::default();
        unsafe {
            instance.get_physical_device_features2(physical_device, &mut supported.v_1_0);
        }
        supported
    }
}

impl Default for Features {
    fn default() -> Self {
        let mut v_1_1 = Box::<vk::PhysicalDeviceVulkan11Features>::default();
        let mut v_1_2 = Box::<vk::PhysicalDeviceVulkan12Features>::default();

        let mut acceleration_structure =
            Box::<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>::default();
        let mut ray_tracing_pipeline =
            Box::<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>::default();

        let v_1_0 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(ray_tracing_pipeline.as_mut())
            .push_next(acceleration_structure.as_mut())
            .push_next(v_1_2.as_mut())
            .push_next(v_1_1.as_mut())
            .build()
            .into();
        Self {
            v_1_0,
            v_1_1,
            v_1_2,
            acceleration_structure,
            ray_tracing_pipeline,
        }
    }
}
