use ash::vk;

pub struct SyncInfo {
    pub wait_on: Vec<vk::Semaphore>,
    pub signal_to: Vec<vk::Semaphore>,
    pub fence: Option<vk::Fence>,
}
