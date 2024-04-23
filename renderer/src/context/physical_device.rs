use std::ops::{Deref, DerefMut};

use ash::vk;

use super::{features::Features, instance::Instance, properties::Properties};

pub struct PhysicalDevice {
    physical_device: vk::PhysicalDevice,
    pub properties: Properties,
    pub features: Features,
}

impl PhysicalDevice {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        firestorm::profile_method!(new);

        let properties = Properties::get_supported(instance, physical_device);
        let features = Features::get_supported(instance, physical_device);

        Self {
            physical_device,
            properties,
            features,
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
