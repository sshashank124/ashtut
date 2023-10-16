use ash::vk;

#[derive(Default)]
pub struct SyncInfo<'a> {
    pub wait_on: &'a [vk::Semaphore],
    pub signal_to: &'a [vk::Semaphore],
    pub fence: Option<vk::Fence>,
}
