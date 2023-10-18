use ash::extensions::khr;

use super::instance::Instance;

pub const REQUIRED_FOR_INSTANCE: &[*const std::ffi::c_char] = &[
    // Debug
    ash::extensions::ext::DebugUtils::name().as_ptr(),
    // Surface
    khr::Surface::name().as_ptr(),
    khr::Win32Surface::name().as_ptr(),
];

pub const REQUIRED_FOR_DEVICE: &[*const std::ffi::c_char] = &[
    // Core
    khr::Swapchain::name().as_ptr(),
    // Acceleration Structure
    khr::AccelerationStructure::name().as_ptr(),
    khr::DeferredHostOperations::name().as_ptr(),
    // Ray Tracing
    khr::RayTracingPipeline::name().as_ptr(),
];

pub struct Handles {
    pub swapchain: khr::Swapchain,
    pub accel: khr::AccelerationStructure,
}

impl Handles {
    pub fn create(instance: &Instance, device: &ash::Device) -> Self {
        let swapchain = khr::Swapchain::new(instance, device);
        let accel = khr::AccelerationStructure::new(instance, device);

        Self { swapchain, accel }
    }
}
