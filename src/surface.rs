use std::ops::{Deref, DerefMut};

use ash::vk;
use winit::window::Window;

use crate::{
    instance::Instance,
    util::{self, Destroy},
};

pub struct Surface {
    surface: vk::SurfaceKHR,
    pub loader: ash::extensions::khr::Surface,
}

pub struct SurfaceDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub fn create(instance: &Instance, window: &Window) -> Self {
        let surface = util::platform::create_surface(instance, window);
        let loader = ash::extensions::khr::Surface::new(&instance.entry, instance);

        Self { surface, loader }
    }

    pub fn get_details(&self, physical_device: vk::PhysicalDevice) -> SurfaceDetails {
        let capabilities = unsafe {
            self.loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
                .expect("Failed to get surface capabilities")
        };
        let formats = unsafe {
            self.loader
                .get_physical_device_surface_formats(physical_device, self.surface)
                .expect("Failed to get surface formats")
        };
        let present_modes = unsafe {
            self.loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
                .expect("Failed to get surface present modes")
        };

        SurfaceDetails {
            capabilities,
            formats,
            present_modes,
        }
    }
}

impl Destroy<()> for Surface {
    fn destroy_with(&self, _: ()) {
        unsafe { self.loader.destroy_surface(self.surface, None) }
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

impl SurfaceDetails {
    pub fn is_populated(&self) -> bool {
        !self.formats.is_empty() && !self.present_modes.is_empty()
    }
}
