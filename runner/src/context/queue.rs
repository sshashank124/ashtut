use std::{collections::HashSet, ops::Deref};

use ash::vk;

use crate::util::Destroy;

use super::{device::Device, instance::Instance, surface::Handle};

pub struct Queues {
    pub graphics: Queue,
    pub transfer: Queue,
    pub families: Families,
}

pub struct Queue {
    pub queue: vk::Queue,
    pub command_pools: Vec<vk::CommandPool>,
}

pub struct Families {
    pub graphics: u32,
    pub transfer: u32,
}

#[derive(Debug, Default)]
struct FamiliesInfo {
    graphics: Option<u32>,
    transfer: Option<u32>,
}

impl Queues {
    pub fn create(device: &ash::Device, families: Families) -> Self {
        let graphics = {
            let pool_infos = [vk::CommandPoolCreateInfo::builder()
                .queue_family_index(families.graphics)
                .build()];

            Queue::create(device, &pool_infos)
        };

        let transfer = {
            let pool_info = [
                vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(families.transfer)
                    .build(),
                vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(families.transfer)
                    .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                    .build(),
            ];

            Queue::create(device, &pool_info)
        };

        Self {
            graphics,
            transfer,
            families,
        }
    }

    pub fn graphics_pool(&self) -> vk::CommandPool {
        self.graphics.command_pools[0]
    }

    /*
    pub fn transfer_pool(&self) -> vk::CommandPool {
        self.transfer.command_pools[0]
    }
    */

    pub fn transient_transfer_pool(&self) -> vk::CommandPool {
        self.transfer.command_pools[1]
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

    pub fn surface_updated(&self, device: &Device) {
        self.graphics.reset(device);
    }
}

impl Queue {
    pub fn create(device: &ash::Device, pool_infos: &[vk::CommandPoolCreateInfo]) -> Self {
        let queue = unsafe { device.get_device_queue(pool_infos[0].queue_family_index, 0) };

        let command_pools = pool_infos
            .iter()
            .map(|pool_info| unsafe { device.create_command_pool(pool_info, None) })
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to create command pool for queue_family@{}",
                    pool_infos[0].queue_family_index
                )
            });

        Self {
            queue,
            command_pools,
        }
    }

    pub fn reset(&self, device: &Device) {
        unsafe {
            self.command_pools
                .iter()
                .try_for_each(|&command_pool| {
                    device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                })
                .expect("Failed to reset command pools");
        }
    }
}

impl Families {
    pub fn unique(&self) -> HashSet<u32> {
        HashSet::from([self.graphics, self.transfer])
    }

    pub fn find(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface: &Handle,
    ) -> Option<Self> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let valid_queue_families = queue_families
            .into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0)
            .collect::<Vec<_>>();

        let mut found_indices = FamiliesInfo::default();
        for &(index, queue_family) in &valid_queue_families {
            let g = queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let c = queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let t = queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER);

            if g && surface.is_supported_by(physical_device, index as u32) {
                found_indices.graphics = Some(index as u32);
            }

            if t && !g && !c {
                found_indices.transfer = Some(index as u32);
            }

            if found_indices.is_complete() {
                break;
            }
        }

        if !found_indices.is_complete() {
            for &(index, queue_family) in &valid_queue_families {
                if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                    found_indices.transfer = Some(index as u32);
                }

                if found_indices.is_complete() {
                    break;
                }
            }
        }

        Self::try_from(found_indices).ok()
    }
}

impl<'a> Destroy<&'a ash::Device> for Queues {
    unsafe fn destroy_with(&mut self, device: &'a ash::Device) {
        self.graphics.destroy_with(device);
        self.transfer.destroy_with(device);
    }
}

impl<'a> Destroy<&'a ash::Device> for Queue {
    unsafe fn destroy_with(&mut self, device: &'a ash::Device) {
        for &command_pool in &self.command_pools {
            device.destroy_command_pool(command_pool, None);
        }
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
            transfer: value.transfer.ok_or(())?,
        })
    }
}

impl FamiliesInfo {
    pub const fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.transfer.is_some()
    }
}
