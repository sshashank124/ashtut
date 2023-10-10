use std::ops::Deref;

use ash::vk;

use super::Destroy;

pub struct CommandPool {
    pool: vk::CommandPool,
}

impl CommandPool {
    pub fn create(device: &ash::Device, pool_info: &vk::CommandPoolCreateInfo) -> Self {
        let pool = unsafe {
            device
                .create_command_pool(pool_info, None)
                .expect("Failed to create command pool")
        };

        Self { pool }
    }

    pub fn create_multiple(
        device: &ash::Device,
        family_index: u32,
        pool_infos: &[vk::CommandPoolCreateInfo],
    ) -> Vec<Self> {
        pool_infos
            .iter()
            .map(|&pool_info| {
                Self::create(
                    device,
                    &vk::CommandPoolCreateInfo {
                        queue_family_index: family_index,
                        ..pool_info
                    },
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn allocate_command_buffer(
        &self,
        device: &ash::Device,
        buffer_info: &vk::CommandBufferAllocateInfo,
    ) -> vk::CommandBuffer {
        unsafe {
            device
                .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    command_pool: self.pool,
                    command_buffer_count: 1,
                    ..*buffer_info
                })
                .expect("Failed to allocate command buffer")[0]
        }
    }

    pub fn reset(&self, device: &ash::Device) {
        unsafe {
            device
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .expect("Failed to reset command pool");
        }
    }
}

impl Destroy<ash::Device> for CommandPool {
    unsafe fn destroy_with(&mut self, device: &mut ash::Device) {
        device.destroy_command_pool(self.pool, None);
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
