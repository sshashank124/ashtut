use std::{
    collections::HashSet,
    ffi::CString,
    ops::{Deref, DerefMut},
};

use ash::vk;
use winit::window::Window;

use super::{
    extensions, physical_device::PhysicalDevice, queue, surface, validator::Validator, Destroy,
};

mod conf {
    pub const VK_API_VERSION: u32 = ash::vk::make_api_version(0, 1, 3, 0);
}

pub struct Instance {
    pub entry: ash::Entry,
    instance: ash::Instance,
    validator: Validator,
}

impl Instance {
    pub fn new(app_name: &str) -> Self {
        let entry = ash::Entry::linked();
        Validator::check_validation_layer_support(&entry);

        let app_name = CString::new(app_name).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .api_version(conf::VK_API_VERSION);

        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(extensions::REQUIRED_FOR_INSTANCE);

        let mut debug_info = Validator::debug_messenger_create_info();
        let instance_create_info =
            Validator::add_validation_to_instance(instance_create_info, &mut debug_info);

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        let validator = Validator::setup(&entry, &instance, debug_info);

        Self {
            entry,
            instance,
            validator,
        }
    }

    pub fn create_surface_on(&self, window: &Window) -> surface::Handle {
        surface::Handle::new(self, window)
    }

    pub fn get_physical_device_and_info(
        &self,
        surface: &surface::Handle,
    ) -> (PhysicalDevice, queue::Families, surface::Config) {
        let all_devices = unsafe {
            self.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        assert!(
            !all_devices.is_empty(),
            "Failed to find a physical device with Vulkan support"
        );

        let (physical_device, queue_families, surface_config_options) = all_devices
            .into_iter()
            .filter(|&physical_device| self.has_required_device_extensions(physical_device))
            .filter_map(|pd| {
                let physical_device = PhysicalDevice::new(self, pd);
                physical_device
                    .features
                    .supports_requirements()
                    .then_some(physical_device)
            })
            .filter_map(|physical_device| {
                queue::Families::find(self, &physical_device, surface).map(|queue_families| {
                    let surface_config_options = surface.get_config_options_for(&physical_device);
                    (physical_device, queue_families, surface_config_options)
                })
            })
            .find(|(_, _, surface_config_options)| Self::is_suitable(surface_config_options))
            .expect("Failed to find a suitable physical device");

        (
            physical_device,
            queue_families,
            surface_config_options.get_optimal(),
        )
    }

    fn is_suitable(surface_config_options: &surface::ConfigurationOptions) -> bool {
        surface_config_options.has_some()
    }

    fn has_required_device_extensions(&self, physical_device: vk::PhysicalDevice) -> bool {
        let available_extensions: HashSet<_> = unsafe {
            self.enumerate_device_extension_properties(physical_device)
                .expect("Failed to get device extension properties")
                .into_iter()
                .map(|e| crate::util::bytes_to_string(e.extension_name.as_ptr()))
                .collect()
        };

        extensions::REQUIRED_FOR_DEVICE
            .iter()
            .copied()
            .map(crate::util::bytes_to_string)
            .all(|ref required_extension| available_extensions.contains(required_extension))
    }
}

impl Destroy<()> for Instance {
    unsafe fn destroy_with(&mut self, ctx: &mut ()) {
        self.validator.destroy_with(ctx);
        self.instance.destroy_instance(None);
    }
}

impl Deref for Instance {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl DerefMut for Instance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.instance
    }
}
