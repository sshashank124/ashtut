use std::{collections::HashSet, ops::Deref};

use ash::vk;

use super::{instance::Instance, physical_device::PhysicalDevice, surface::Handle};

pub struct Queues {
    graphics: Queue,
}

pub struct Queue {
    pub queue: vk::Queue,
    pub family_index: u32,
}

pub struct Families {
    pub graphics: u32,
}

#[derive(Debug, Default)]
struct FamiliesInfo {
    graphics: Option<u32>,
}

impl Queues {
    pub fn create(device: &ash::Device, families: &Families) -> Self {
        let graphics = Queue::create(device, families.graphics);

        Self { graphics }
    }

    pub const fn graphics(&self) -> &Queue {
        &self.graphics
    }

    pub fn create_infos(indices: &Families) -> Vec<vk::DeviceQueueCreateInfo> {
        indices
            .unique()
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

impl Queue {
    pub fn create(device: &ash::Device, family_index: u32) -> Self {
        let queue = unsafe { device.get_device_queue(family_index, 0) };

        Self {
            queue,
            family_index,
        }
    }
}

impl Families {
    pub fn unique(&self) -> HashSet<u32> {
        HashSet::from([self.graphics])
    }

    pub fn find(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        surface: &Handle,
    ) -> Option<Self> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(**physical_device) };

        let valid_queue_families = queue_families
            .into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0)
            .collect::<Vec<_>>();

        let mut found_indices = FamiliesInfo::default();
        for (index, queue_family) in valid_queue_families {
            let g = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let c = queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let t = queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER);

            if g && c && t && surface.is_supported_by(physical_device, index as u32) {
                found_indices.graphics = Some(index as u32);
            }

            if found_indices.is_complete() {
                break;
            }
        }

        Self::try_from(found_indices).ok()
    }
}

impl Deref for Queue {
    type Target = vk::Queue;
    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

impl TryFrom<FamiliesInfo> for Families {
    type Error = ();
    fn try_from(value: FamiliesInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            graphics: value.graphics.ok_or(())?,
        })
    }
}

impl FamiliesInfo {
    pub const fn is_complete(&self) -> bool {
        self.graphics.is_some()
    }
}
