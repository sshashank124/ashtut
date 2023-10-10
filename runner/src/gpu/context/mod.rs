mod device;
mod features;
mod instance;
mod physical_device;
pub mod queue;
mod surface;
mod validator;

use std::ops::{Deref, DerefMut};

pub use gpu_allocator::vulkan as gpu_alloc;

use winit::window::Window;

use self::{device::Device, instance::Instance, physical_device::PhysicalDevice, surface::Surface};

use super::Destroy;

pub struct Context {
    pub instance: Instance,
    pub physical_device: PhysicalDevice,
    pub surface: Surface,
    pub device: Device,
}

impl Context {
    pub fn init(window: &Window) -> Self {
        let instance = Instance::new(&window.title());
        let surface_handle = instance.create_surface_on(window);

        let (physical_device, queue_families, surface_config) =
            instance.get_physical_device_and_info(&surface_handle);

        let device = Device::new(&instance, &physical_device, &queue_families);

        let surface = Surface::new(surface_handle, surface_config);

        Self {
            instance,
            physical_device,
            surface,
            device,
        }
    }

    pub fn refresh_surface_capabilities(&mut self) -> bool {
        self.surface.refresh_capabilities(&self.physical_device)
    }
}

impl Destroy<()> for Context {
    unsafe fn destroy_with(&mut self, input: &mut ()) {
        self.device.destroy_with(input);
        self.surface.destroy_with(input);
        self.instance.destroy_with(input);
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