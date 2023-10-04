use ash::vk;

pub struct Features {
    pub v_1_0: Box<vk::PhysicalDeviceFeatures2>,
    pub v_1_1: Box<vk::PhysicalDeviceVulkan11Features>,
    pub v_1_2: Box<vk::PhysicalDeviceVulkan12Features>,
}

impl Features {
    pub fn required() -> Self {
        let mut feature = Self::default();
        feature.v_1_2.vulkan_memory_model = 1;
        feature
    }

    pub const fn supports_requirements(&self) -> bool {
        self.v_1_2.vulkan_memory_model > 0
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
