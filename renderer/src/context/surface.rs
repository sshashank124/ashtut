use std::ops::{Deref, DerefMut};

use ash::{khr, vk};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::{instance::Instance, physical_device::PhysicalDevice};

pub mod conf {
    use ash::vk;

    pub const PREFERRED_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
        format: vk::Format::B8G8R8A8_SRGB,
        color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
    };
    pub const PREFERRED_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO_RELAXED;
    pub const FALLBACK_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::FIFO;
}

pub struct Surface {
    handle: Handle,
    pub config: Config,
}

pub struct Handle {
    surface: vk::SurfaceKHR,
    loader: khr::surface::Instance,
}

pub struct Config {
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
    pub image_count: u32,
}

pub struct ConfigurationOptions {
    capabilities: vk::SurfaceCapabilitiesKHR,
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub const fn new(handle: Handle, config: Config) -> Self {
        Self { handle, config }
    }

    pub fn refresh_capabilities(&mut self, physical_device: &PhysicalDevice) -> bool {
        self.config
            .update_with(&self.get_capabilities(physical_device));
        self.config.valid_extent()
    }
}

impl Handle {
    pub fn new(instance: &Instance, window: &impl HasWindowHandle) -> Self {
        firestorm::profile_method!(new);

        let surface = create_surface(instance, window);
        let loader = khr::surface::Instance::new(&instance.entry, instance);
        Self { surface, loader }
    }

    pub fn get_config_options_for(&self, physical_device: &PhysicalDevice) -> ConfigurationOptions {
        firestorm::profile_method!(get_config_options_for);

        let capabilities = self.get_capabilities(physical_device);
        let surface_formats = unsafe {
            self.loader
                .get_physical_device_surface_formats(**physical_device, self.surface)
                .expect("Failed to get surface formats")
        };
        let present_modes = unsafe {
            self.loader
                .get_physical_device_surface_present_modes(**physical_device, self.surface)
                .expect("Failed to get surface present modes")
        };

        ConfigurationOptions {
            capabilities,
            surface_formats,
            present_modes,
        }
    }

    fn get_capabilities(&self, physical_device: &PhysicalDevice) -> vk::SurfaceCapabilitiesKHR {
        firestorm::profile_method!(get_capabilities);

        unsafe {
            self.loader
                .get_physical_device_surface_capabilities(**physical_device, self.surface)
                .expect("Failed to get surface capabilities")
        }
    }

    pub fn is_supported_by(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> bool {
        firestorm::profile_method!(is_supported_by);

        unsafe {
            self.loader
                .get_physical_device_surface_support(
                    **physical_device,
                    queue_family_index,
                    self.surface,
                )
                .expect("Failed to get physical device surface support info")
        }
    }
}

impl ConfigurationOptions {
    pub fn has_some(&self) -> bool {
        !self.surface_formats.is_empty() && !self.present_modes.is_empty()
    }

    pub fn get_optimal(&self) -> Config {
        let surface_format = Self::choose_best_surface_format(&self.surface_formats);
        let extent = Self::choose_extent(&self.capabilities);
        let image_count = Self::choose_image_count(&self.capabilities);
        let present_mode = Self::choose_best_present_mode(&self.present_modes);

        Config {
            surface_format,
            present_mode,
            extent,
            image_count,
        }
    }

    fn choose_best_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        formats
            .iter()
            .copied()
            .find(|&format| format == conf::PREFERRED_SURFACE_FORMAT)
            .unwrap_or_else(|| formats[0])
    }

    fn choose_best_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
        present_modes
            .iter()
            .copied()
            .find(|&format| format == conf::PREFERRED_PRESENT_MODE)
            .unwrap_or(conf::FALLBACK_PRESENT_MODE)
    }

    fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

        vk::Extent2D {
            width: capabilities.current_extent.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: capabilities.current_extent.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }

    fn choose_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
        let image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 {
            image_count.min(capabilities.max_image_count)
        } else {
            image_count
        }
    }
}

impl Config {
    fn update_with(&mut self, surface_capabilities: &vk::SurfaceCapabilitiesKHR) {
        self.extent = ConfigurationOptions::choose_extent(surface_capabilities);
    }

    const fn valid_extent(&self) -> bool {
        self.extent.width != 0 && self.extent.height != 0
    }
}

impl Deref for Surface {
    type Target = Handle;
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl DerefMut for Surface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handle
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        firestorm::profile_method!(drop);

        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}

impl Deref for Handle {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl DerefMut for Handle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surface
    }
}

fn create_surface(instance: &Instance, window: &impl HasWindowHandle) -> vk::SurfaceKHR {
    firestorm::profile_fn!(create_surface);

    let raw_window_handle = window
        .window_handle()
        .expect("Unable to get window handle")
        .as_raw();

    match raw_window_handle {
        RawWindowHandle::Win32(handle) => {
            let create_info = vk::Win32SurfaceCreateInfoKHR::default()
                .hwnd(handle.hwnd.get() as _)
                .hinstance(handle.hinstance.expect("No Win32 HINSTANCE found").get() as _);
            unsafe {
                khr::win32_surface::Instance::new(&instance.entry, instance)
                    .create_win32_surface(&create_info, None)
                    .expect("Failed to create Windows surface")
            }
        }
        _ => panic!("This platform is not supported yet"),
    }
}
