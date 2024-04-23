use ash::{ext, khr};

use super::instance::Instance;

pub const REQUIRED_FOR_INSTANCE: &[*const std::ffi::c_char] = &[
    // Debug
    ext::debug_utils::NAME.as_ptr(),
    // Surface
    khr::surface::NAME.as_ptr(),
    khr::win32_surface::NAME.as_ptr(),
];

pub const REQUIRED_FOR_DEVICE: &[*const std::ffi::c_char] = &[
    // Core
    khr::swapchain::NAME.as_ptr(),
    // Acceleration Structure
    khr::acceleration_structure::NAME.as_ptr(),
    khr::deferred_host_operations::NAME.as_ptr(),
    // Ray Tracing
    khr::ray_tracing_pipeline::NAME.as_ptr(),
    // Additional
    ext::memory_priority::NAME.as_ptr(),
    ext::pageable_device_local_memory::NAME.as_ptr(),
];

pub struct Handles {
    pub debug_utils: ext::debug_utils::Device,
    pub swapchain: khr::swapchain::Device,
    pub accel: khr::acceleration_structure::Device,
    pub ray_tracing: khr::ray_tracing_pipeline::Device,
}

impl Handles {
    pub fn create(instance: &Instance, device: &ash::Device) -> Self {
        firestorm::profile_method!(create_shader_module_from_file);

        let debug_utils = ext::debug_utils::Device::new(instance, device);
        let swapchain = khr::swapchain::Device::new(instance, device);
        let accel = khr::acceleration_structure::Device::new(instance, device);
        let ray_tracing = khr::ray_tracing_pipeline::Device::new(instance, device);

        Self {
            debug_utils,
            swapchain,
            accel,
            ray_tracing,
        }
    }
}
