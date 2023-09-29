use ash::vk;

use crate::{
    instance::Instance,
    physical_device::PhysicalDevice,
};

pub struct Device {
    inner: ash::Device,
    _graphics_queue: vk::Queue,
}

impl Device {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Self {
        let queue_create_infos = [
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(physical_device.indices.graphics)
                .queue_priorities(&[1.0_f32])
                .build(),
        ];

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos);
        
        let inner = unsafe {
            instance.create_device(**physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };
        
        let _graphics_queue = unsafe { inner.get_device_queue(physical_device.indices.graphics, 0) };

        Self { inner, _graphics_queue }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_device(None);
        }
    }
}