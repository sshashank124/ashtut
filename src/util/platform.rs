use std::ffi::c_void;

use ash::vk;
use ash::extensions::khr::Win32Surface;
use winit::{
    platform::windows::WindowExtWindows,
    window::Window,
};

use crate::instance::Instance;

pub fn create_surface(
    entry: &ash::Entry,
    instance: &Instance,
    window: &Window,
) -> vk::SurfaceKHR {
    let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
        .hinstance(window.hinstance() as *const c_void)
        .hwnd(window.hwnd() as *const c_void);
    unsafe {
        Win32Surface::new(entry, instance).create_win32_surface(&create_info, None)
            .expect("Failed to create Windows surface")
    }
}