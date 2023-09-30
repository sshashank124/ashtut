use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::{
    features::Features,
    instance::Instance,
    physical_device::PhysicalDevice,
    util::{info, Destroy},
};

pub struct Device {
    inner: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

impl Device {
    pub fn create(instance: &Instance, physical_device: &PhysicalDevice) -> Self {
        let queue_create_infos = physical_device
            .indices
            .unique_queue_family_indices()
            .iter()
            .map(|&index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(index)
                    .queue_priorities(&[1.0_f32])
                    .build()
            })
            .collect::<Vec<_>>();
        
        let mut required_features = Features::required();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(info::REQUIRED_DEVICE_EXTENSIONS)
            .push_next(required_features.v_1_0.as_mut());

        let inner = unsafe {
            instance
                .create_device(**physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };

        let graphics_queue =
            unsafe { inner.get_device_queue(physical_device.indices.graphics(), 0) };
        let present_queue = unsafe { inner.get_device_queue(physical_device.indices.present(), 0) };

        Self {
            inner,
            graphics_queue,
            present_queue,
        }
    }
}

impl Destroy<()> for Device {
    fn destroy_with(&self, _: ()) {
        unsafe {
            self.inner.destroy_device(None);
        }
    }
}

impl Deref for Device {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Device {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
