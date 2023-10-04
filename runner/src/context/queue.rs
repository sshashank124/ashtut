use std::collections::HashSet;

use ash::vk;

use super::{instance::Instance, surface::Descriptor};

pub struct Queue {
    pub graphics: vk::Queue,
    pub present: vk::Queue,
    pub families: Families,
}
pub struct Families {
    indices: [u32; 2],
}

#[derive(Default)]
struct FamiliesInfo {
    graphics: Option<u32>,
    present: Option<u32>,
}

impl Queue {
    pub fn create_infos(indices: &Families) -> Vec<vk::DeviceQueueCreateInfo> {
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

impl Families {
    pub const fn graphics(&self) -> u32 {
        self.indices[0]
    }

    pub const fn present(&self) -> u32 {
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
        surface: &Descriptor,
    ) -> Option<Self> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let valid_queue_families = queue_families
            .into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0);

        let mut found_indices = FamiliesInfo::default();
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

impl TryFrom<FamiliesInfo> for Families {
    type Error = ();
    fn try_from(value: FamiliesInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            indices: [value.graphics.ok_or(())?, value.present.ok_or(())?],
        })
    }
}

impl FamiliesInfo {
    pub const fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}
