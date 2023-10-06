mod device;
mod features;
mod instance;
mod queue;
mod surface;
mod validator;

use ash::vk;
pub use gpu_allocator::vulkan as gpu_alloc;

use winit::window::Window;

use crate::util::Destroy;

use self::{device::Device, instance::Instance, surface::Surface};

pub struct Context {
    pub instance: Instance,
    physical_device: vk::PhysicalDevice,
    pub surface: Surface,
    pub device: Device,
}

impl Context {
    pub fn init(window: &Window) -> Self {
        let instance = Instance::new(&window.title());
        let surface_handle = instance.create_surface_on(window);

        let (physical_device, queue_families, surface_config) =
            instance.get_physical_device_and_info(&surface_handle);

        let device = Device::new(&instance, physical_device, queue_families);

        let surface = Surface::new(surface_handle, surface_config);

        Self {
            instance,
            physical_device,
            surface,
            device,
        }
    }

    pub fn refresh_surface_capabilities(&mut self) -> bool {
        let is_valid = self.surface.refresh_capabilities(self.physical_device);
        if is_valid {
            self.device.surface_updated();
        }
        is_valid
    }

    pub unsafe fn device_wait_idle(&self) {
        self.device
            .device_wait_idle()
            .expect("Failed to wait for device to idle");
    }
}

impl Destroy<()> for Context {
    unsafe fn destroy_with(&mut self, _: ()) {
        self.device.destroy_with(());
        self.surface.destroy_with(());
        self.instance.destroy_with(());
    }
}

mod config {}
