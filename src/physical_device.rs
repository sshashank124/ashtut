use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::instance::Instance;

pub struct PhysicalDevice {
    pub inner: vk::PhysicalDevice,
    pub indices: QueueFamilyIndices,
}

pub struct QueueFamilyIndices {
    pub graphics: u32,
}

#[derive(Default)]
struct QueueFamilyIndicesBuilder {
    graphics: Option<u32>,
}

impl PhysicalDevice {
    pub fn pick(instance: &Instance) -> Self {
        let all_devices = unsafe {
            instance.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };
        
        if all_devices.is_empty() {
            panic!("Failed to find a physical device with Vulkan support");
        }
        
        let (inner, indices) = all_devices.into_iter()
            .map(|physical_device| (physical_device, Self::find_queue_families(instance, physical_device)))
            .find(|(physical_device, indices)| Self::is_suitable(instance, *physical_device, indices))
            .expect("Failed to find a suitable physical device");

        Self {
            inner,
            indices: indices.into(),
        }
    }
    
    fn is_suitable(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        indices: &QueueFamilyIndicesBuilder,
    ) -> bool {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU && indices.complete()
    }
    
    fn find_queue_families(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> QueueFamilyIndicesBuilder {
        let queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        
        let valid_queue_families = queue_families.into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0);
        
        let mut indices = QueueFamilyIndicesBuilder::default();
        for (index, queue_family) in valid_queue_families {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics = Some(index as u32);
            }
            
            if indices.complete() { break; }
        }
        
        indices
    }
}

impl QueueFamilyIndicesBuilder {
    fn complete(&self) -> bool {
        self.graphics.is_some()
    }
}

impl From<QueueFamilyIndicesBuilder> for QueueFamilyIndices {
    fn from(value: QueueFamilyIndicesBuilder) -> Self {
        Self {
            graphics: value.graphics.unwrap()
        }
    }
}

impl Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for PhysicalDevice {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}