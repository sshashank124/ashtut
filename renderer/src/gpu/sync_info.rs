use ash::vk;

#[derive(Default)]
pub struct SyncInfo {
    pub wait_on: Option<vk::Semaphore>,
    pub signal_to: Option<vk::Semaphore>,
    pub fence: Option<vk::Fence>,
}
