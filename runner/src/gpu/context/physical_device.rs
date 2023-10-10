use std::ops::{Deref, DerefMut};

use ash::vk;

use super::instance::Instance;

pub struct PhysicalDevice {
    physical_device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
}

impl PhysicalDevice {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        Self {
            physical_device,
            properties,
        }
    }
}

impl Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;
    fn deref(&self) -> &Self::Target {
        &self.physical_device
    }
}

impl DerefMut for PhysicalDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.physical_device
    }
}
