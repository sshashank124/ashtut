mod device;
mod extensions;
mod features;
mod instance;
mod physical_device;
mod properties;
pub mod queue;
mod surface;

use std::ops::{Deref, DerefMut};

use raw_window_handle::HasWindowHandle;

use self::{device::Device, instance::Instance, physical_device::PhysicalDevice, surface::Surface};

pub struct Context {
    pub device: Device,
    pub surface: Surface,
    pub physical_device: PhysicalDevice,
    _instance: Instance,
}

impl Context {
    pub fn init(name: &str, window: &impl HasWindowHandle) -> Self {
        firestorm::profile_method!(init);

        let instance = Instance::new(name);
        let surface_handle = instance.create_surface_on(window);

        let (physical_device, queue_families, surface_config) =
            instance.get_physical_device_and_info(&surface_handle);

        let surface = Surface::new(surface_handle, surface_config);

        let device = Device::create(&instance, &physical_device, &queue_families);

        Self {
            device,
            surface,
            physical_device,
            _instance: instance,
        }
    }

    pub fn refresh_surface_capabilities(&mut self) -> bool {
        self.surface.refresh_capabilities(&self.physical_device)
    }
}

impl Deref for Context {
    type Target = Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

unsafe fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    std::ffi::CStr::from_ptr(string)
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}
