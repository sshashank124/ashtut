use std::ops::{Deref, DerefMut};

use ash::vk;
use winit::window::Window;

use crate::{
    instance::Instance,
    util::{self, info, Destroy},
};

pub struct Surface {
    surface: vk::SurfaceKHR,
    loader: ash::extensions::khr::Surface,
    pub config: SurfaceConfig,
}

pub struct SurfaceDescriptor {
    surface: vk::SurfaceKHR,
    loader: ash::extensions::khr::Surface,
}

pub struct SurfaceConfig {
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
    pub image_count: u32,
}

pub struct SurfaceConfigurationOptions {
    capabilities: vk::SurfaceCapabilitiesKHR,
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SurfaceDescriptor {
    pub fn new(instance: &Instance, window: &Window) -> Self {
        let surface = util::platform::create_surface(instance, window);
        let loader = ash::extensions::khr::Surface::new(&instance.entry, instance);

        Self { surface, loader }
    }

    pub fn with_config(self, config: SurfaceConfig) -> Surface {
        Surface {
            surface: self.surface,
            loader: self.loader,
            config,
        }
    }

    pub fn get_config_options_for(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> SurfaceConfigurationOptions {
        let capabilities = self.get_capabilities(physical_device);
        let surface_formats = unsafe {
            self.loader
                .get_physical_device_surface_formats(physical_device, self.surface)
                .expect("Failed to get surface formats")
        };
        let present_modes = unsafe {
            self.loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
                .expect("Failed to get surface present modes")
        };

        SurfaceConfigurationOptions {
            capabilities,
            surface_formats,
            present_modes,
        }
    }

    pub fn get_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> vk::SurfaceCapabilitiesKHR {
        unsafe {
            self.loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
                .expect("Failed to get surface capabilities")
        }
    }

    pub fn is_supported_by(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> bool {
        unsafe {
            self.loader
                .get_physical_device_surface_support(
                    physical_device,
                    queue_family_index,
                    self.surface,
                )
                .expect("Failed to get physical device surface support info")
        }
    }
}

impl SurfaceConfigurationOptions {
    pub fn has_some(&self) -> bool {
        !self.surface_formats.is_empty() && !self.present_modes.is_empty()
    }

    pub fn get_optimal(&self) -> SurfaceConfig {
        let surface_format = Self::choose_best_surface_format(&self.surface_formats);
        let extent = Self::choose_extent(&self.capabilities);
        let image_count = Self::choose_image_count(&self.capabilities);
        let present_mode = Self::choose_best_present_mode(&self.present_modes);

        SurfaceConfig {
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
            .find(|&format| format == info::PREFERRED_SURFACE_FORMAT)
            .unwrap_or_else(|| formats[0])
    }

    fn choose_best_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
        present_modes
            .iter()
            .copied()
            .find(|&format| format == info::PREFERRED_PRESENT_MODE)
            .unwrap_or(info::FALLBACK_PRESENT_MODE)
    }

    pub fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

        vk::Extent2D {
            width: capabilities
                .current_extent
                .width
                .max(capabilities.min_image_extent.width)
                .min(capabilities.max_image_extent.width),
            height: capabilities
                .current_extent
                .height
                .max(capabilities.min_image_extent.height)
                .min(capabilities.max_image_extent.height),
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

impl Destroy<()> for Surface {
    unsafe fn destroy_with(&self, _: ()) {
        self.loader.destroy_surface(self.surface, None);
    }
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl DerefMut for Surface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surface
    }
}
