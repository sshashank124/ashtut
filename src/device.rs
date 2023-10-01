use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::{
    instance::{Features, Instance},
    surface::SurfaceDescriptor,
    util::{info, Destroy},
};

pub struct Device {
    device: ash::Device,
    pub queue: Queue,
}

pub struct Queue {
    pub graphics: vk::Queue,
    pub present: vk::Queue,
    pub families: QueueFamilies,
}
pub struct QueueFamilies {
    indices: [u32; 2],
}

#[derive(Default)]
struct QueueFamiliesInfo {
    graphics: Option<u32>,
    present: Option<u32>,
}

impl Device {
    pub fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        families: QueueFamilies,
    ) -> Self {
        let mut required_features: Features = info::required_features();
        let queue_create_infos = Queue::create_infos(&families);
        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(info::REQUIRED_DEVICE_EXTENSIONS)
            .push_next(required_features.v_1_0.as_mut());

        let device = unsafe {
            instance
                .create_device(physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };

        let queue = unsafe {
            Queue {
                graphics: device.get_device_queue(families.graphics(), 0),
                present: device.get_device_queue(families.present(), 0),
                families,
            }
        };

        Self { device, queue }
    }

    pub fn create_command_pool(&self) -> vk::CommandPool {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.queue.families.graphics())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe {
            self.device
                .create_command_pool(&create_info, None)
                .expect("Failed to create command pool")
        }
    }

    pub fn create_semaphore(&self, name: &str) -> vk::Semaphore {
        let create_info = vk::SemaphoreCreateInfo::builder();
        unsafe {
            self.device
                .create_semaphore(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` semaphore: {}", name, err))
        }
    }

    pub fn create_fence(&self, name: &str, signaled: bool) -> vk::Fence {
        let create_info = vk::FenceCreateInfo::builder().flags(if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        });
        unsafe {
            self.device
                .create_fence(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` fence: {}", name, err))
        }
    }

    pub unsafe fn wait_until_idle(&self) {
        self.device_wait_idle()
            .expect("Failed to wait for device to idle");
    }
}

impl Queue {
    pub fn create_infos(indices: &QueueFamilies) -> Vec<vk::DeviceQueueCreateInfo> {
        indices
            .unique_queue_family_indices()
            .iter()
            .map(|&index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(index)
                    .queue_priorities(&[1.0_f32])
                    .build()
            })
            .collect::<Vec<_>>()
    }
}

impl QueueFamilies {
    pub fn graphics(&self) -> u32 {
        self.indices[0]
    }
    pub fn present(&self) -> u32 {
        self.indices[1]
    }
    pub fn separate_graphics_and_presentation_indices(&self) -> Option<&[u32]> {
        if self.graphics() == self.present() {
            None
        } else {
            Some(&self.indices[..2])
        }
    }
    pub fn unique_queue_family_indices(&self) -> HashSet<u32> {
        HashSet::from_iter(self.indices)
    }

    pub fn find(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface: &SurfaceDescriptor,
    ) -> Option<Self> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let valid_queue_families = queue_families
            .into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0);

        let mut found_indices = QueueFamiliesInfo::default();
        for (index, queue_family) in valid_queue_families {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                found_indices.graphics = Some(index as u32);
            }

            if surface.is_supported_by(physical_device, index as u32) {
                found_indices.present = Some(index as u32);
            }

            if found_indices.is_complete() {
                break;
            }
        }

        Self::try_from(found_indices).ok()
    }
}

impl TryFrom<QueueFamiliesInfo> for QueueFamilies {
    type Error = ();
    fn try_from(value: QueueFamiliesInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            indices: [value.graphics.ok_or(())?, value.present.ok_or(())?],
        })
    }
}

impl QueueFamiliesInfo {
    pub fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}

impl Destroy<()> for Device {
    unsafe fn destroy_with(&self, _: ()) {
        self.device.destroy_device(None);
    }
}

impl Deref for Device {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Device {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}
