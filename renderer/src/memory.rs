pub enum Priority {
    Medium,
    High,
}

impl From<Priority> for f32 {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::Medium => 0.5,
            Priority::High => 0.75,
        }
    }
}

pub mod purpose {
    pub fn dedicated() -> vk_mem::AllocationCreateInfo {
        vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::DEDICATED_MEMORY,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            priority: super::Priority::High.into(),
            ..Default::default()
        }
    }

    pub fn device_local(priority: super::Priority) -> vk_mem::AllocationCreateInfo {
        vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            priority: priority.into(),
            ..Default::default()
        }
    }

    pub fn staging() -> vk_mem::AllocationCreateInfo {
        vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            priority: super::Priority::Medium.into(),
            ..Default::default()
        }
    }
}

pub const fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
