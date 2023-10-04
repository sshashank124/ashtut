mod device;
mod features;
mod instance;
mod queue;
mod surface;
mod validator;

pub use gpu_allocator::vulkan as gpu_alloc;

use winit::window::Window;

use crate::util::Destroy;

use self::{device::Device, instance::Instance, surface::Surface};

pub struct Context {
    pub instance: Instance,
    pub surface: Surface,
    pub device: Device,
}

impl Context {
    pub fn init(window: &Window) -> Self {
        let instance = Instance::new(&window.title());
        let surface_descriptor = instance.create_surface_on(window);
        let (device, surface) = instance.request_device_for(surface_descriptor);

        Self {
            instance,
            surface,
            device,
        }
    }

    pub fn refresh_surface_capabilities(&mut self) -> bool {
        self.surface
            .refresh_capabilities(self.device.physical_device)
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
