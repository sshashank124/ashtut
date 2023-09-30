use std::ffi::{c_char, CStr};

use ash::{
    extensions::khr,
    vk,
};

// Window Info
pub const WINDOW_TITLE: &str = "Learning Vulkan & Ash";
pub const WINDOW_SIZE: (u32, u32) = (800, 600);

// Vulkan Info
pub const VK_API_VERSION: u32 = vk::make_api_version(0, 1, 3, 261);

pub const REQUIRED_EXTENSIONS: &[*const c_char] = &[
    khr::Surface::name().as_ptr(),
    khr::Win32Surface::name().as_ptr(),
    ash::extensions::ext::DebugUtils::name().as_ptr(),
];

pub const REQUIRED_DEVICE_EXTENSIONS: &[*const c_char] = &[
    khr::Swapchain::name().as_ptr(),
    vk::KhrVulkanMemoryModelFn::name().as_ptr(),
];

pub const PREFERRED_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
    format: vk::Format::B8G8R8A8_SRGB,
    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
};

// Swapchain
pub const PREFERRED_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
pub const FALLBACK_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO;

// Shaders
pub const SHADER_FILE: &str = env!("raster.spv");
pub const VERTEX_SHADER_ENTRY_POINT: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"vert_main\0") };
pub const FRAGMENT_SHADER_ENTRY_POINT: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"frag_main\0") };