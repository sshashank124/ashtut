use ash::vk;

// Window Info
pub const WINDOW_TITLE: &str = "Learning Vulkan & Ash";
pub const WINDOW_SIZE: (u32, u32) = (800, 600);

// Vulkan Info
pub const VK_API_VERSION: u32 = vk::make_api_version(0, 1, 3, 261);