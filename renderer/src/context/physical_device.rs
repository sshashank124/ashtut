use std::ops::{Deref, DerefMut};

use ash::vk;

use super::{features, instance::Instance, properties::Properties};

pub struct PhysicalDevice {
    physical_device: vk::PhysicalDevice,
    pub properties: Properties,
}

impl PhysicalDevice {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice) -> Option<Self> {
        firestorm::profile_method!(new);

        if features::supported_by(instance, physical_device) {
            Some(Self {
                physical_device,
                properties: Properties::get_supported(instance, physical_device),
            })
        } else {
            None
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
