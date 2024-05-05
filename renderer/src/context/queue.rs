use std::{collections::HashSet, ops::Deref};

use ash::vk;

use super::{instance::Instance, physical_device::PhysicalDevice, surface::Handle};

pub struct Queues {
    graphics: Queue,
    compute: Queue,
    transfer: Queue,
}

pub struct Queue {
    queue: vk::Queue,
    pub family_index: u32,
}

pub struct Families {
    graphics: u32,
    compute: u32,
    transfer: u32,
}

#[derive(Debug, Default)]
struct FamiliesInfo {
    graphics: Option<u32>,
    compute: Option<u32>,
    transfer: Option<u32>,
}

impl Queues {
    pub fn create(device: &ash::Device, families: &Families) -> Self {
        let graphics = Queue::create(device, families.graphics, 0);
        let compute = Queue::create(device, families.compute, 0);
        let transfer = Queue::create(device, families.transfer, 0);

        Self {
            graphics,
            compute,
            transfer,
        }
    }

    pub const fn graphics(&self) -> &Queue {
        &self.graphics
    }

    pub const fn compute(&self) -> &Queue {
        &self.graphics
    }

    pub const fn transfer(&self) -> &Queue {
        &self.transfer
    }

    pub fn create_infos(indices: &Families) -> Vec<vk::DeviceQueueCreateInfo> {
        indices
            .unique()
            .iter()
            .map(|&index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(index)
                    .queue_priorities(&[1.0_f32])
            })
            .collect::<Vec<_>>()
    }
}

impl Queue {
    fn create(device: &ash::Device, family_index: u32, queue_index: u32) -> Self {
        let queue = unsafe { device.get_device_queue(family_index, queue_index) };

        Self {
            queue,
            family_index,
        }
    }
}

impl Families {
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
            let idx = index as u32;

            let g = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let c = queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let t = queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER);
            let vd = queue_family
                .queue_flags
                .contains(vk::QueueFlags::VIDEO_DECODE_KHR);
            let ve = queue_family
                .queue_flags
                .contains(vk::QueueFlags::VIDEO_ENCODE_KHR);

            if !g && !c && t && !vd && !ve {
                found_indices.transfer = Some(idx);
            } else if !g && c {
                found_indices.compute = Some(idx);
            } else if g && c && surface.is_supported_by(physical_device, idx) {
                // TODO: this should not be checking for the compute flag
                found_indices.graphics = Some(idx);
            }

            if found_indices.is_complete() {
                break;
            }
        }

        Self::try_from(found_indices).ok()
    }

    fn unique(&self) -> HashSet<u32> {
        HashSet::from([self.graphics, self.compute, self.transfer])
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
            compute: value.compute.ok_or(())?,
            transfer: value.transfer.ok_or(())?,
        })
    }
}

impl FamiliesInfo {
    pub const fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.compute.is_some() && self.transfer.is_some()
    }
}
