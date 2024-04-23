use std::{
    collections::HashSet,
    ffi::CString,
    ops::{Deref, DerefMut},
};

use ash::vk;
use raw_window_handle::HasWindowHandle;

use super::{bytes_to_string, extensions, physical_device::PhysicalDevice, queue, surface};

pub struct Instance {
    pub entry: ash::Entry,
    instance: ash::Instance,
}

impl Instance {
    pub fn new(app_name: &str) -> Self {
        firestorm::profile_method!(new);

        let entry = ash::Entry::linked();

        let app_name = CString::new(app_name).unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .api_version(crate::conf::VK_API_VERSION);

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extensions::REQUIRED_FOR_INSTANCE);

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        Self { entry, instance }
    }

    pub fn create_surface_on(&self, window: &impl HasWindowHandle) -> surface::Handle {
        surface::Handle::new(self, window)
    }

    pub fn get_physical_device_and_info(
        &self,
        surface: &surface::Handle,
    ) -> (PhysicalDevice, queue::Families, surface::Config) {
        firestorm::profile_method!(get_physical_device_and_info);

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
        firestorm::profile_method!(has_required_device_extensions);

        let available_extensions: HashSet<_> = unsafe {
            self.enumerate_device_extension_properties(physical_device)
                .expect("Failed to get device extension properties")
                .into_iter()
                .map(|e| bytes_to_string(e.extension_name.as_ptr()))
                .collect()
        };

        extensions::REQUIRED_FOR_DEVICE
            .iter()
            .copied()
            .map(|s| unsafe { bytes_to_string(s) })
            .all(|ref required_extension| available_extensions.contains(required_extension))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        firestorm::profile_method!(drop);

        unsafe {
            self.instance.destroy_instance(None);
        }
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
