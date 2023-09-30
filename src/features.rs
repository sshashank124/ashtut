use ash::vk;

use crate::instance::Instance;

#[derive(Debug)]
pub struct Features {
    pub v_1_0: Box<vk::PhysicalDeviceFeatures2>,
    pub v_1_1: Box<vk::PhysicalDeviceVulkan11Features>,
    pub v_1_2: Box<vk::PhysicalDeviceVulkan12Features>,
}

impl Features {
    pub fn get_supported(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut supported = Self::default();
        unsafe { instance.get_physical_device_features2(physical_device, &mut supported.v_1_0) };
        supported
    }

    pub fn required() -> Self {
        let mut required = Self::default();
        required.v_1_2.vulkan_memory_model = 1;
        required
    }
}

impl Default for Features {
    fn default() -> Self {
        let mut v_1_1 = Box::<vk::PhysicalDeviceVulkan11Features>::default();
        let mut v_1_2 = Box::<vk::PhysicalDeviceVulkan12Features>::default();
        let v_1_0 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(v_1_2.as_mut())
            .push_next(v_1_1.as_mut())
            .build()
            .into();
        Self {
            v_1_0,
            v_1_1,
            v_1_2,
        }
    }
}
