use std::{
    collections::HashSet,
    ffi::CString,
    ops::{Deref, DerefMut},
};

use ash::vk;
use winit::window::Window;

use crate::{
    device::{Device, QueueFamilies},
    surface::{Surface, SurfaceConfig, SurfaceConfigurationOptions, SurfaceDescriptor},
    util::{self, info, Destroy},
    validator::Validator,
};

pub struct Instance {
    pub entry: ash::Entry,
    instance: ash::Instance,
    validator: Validator,
}

pub struct Features {
    pub v_1_0: Box<vk::PhysicalDeviceFeatures2>,
    pub v_1_1: Box<vk::PhysicalDeviceVulkan11Features>,
    pub v_1_2: Box<vk::PhysicalDeviceVulkan12Features>,
}

impl Instance {
    pub fn new() -> Self {
        let entry = ash::Entry::linked();
        Validator::check_validation_layer_support(&entry);

        let app_name = CString::new(info::WINDOW_TITLE).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .api_version(info::VK_API_VERSION);

        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(info::REQUIRED_EXTENSIONS);

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

    pub fn create_surface_on(&self, window: &Window) -> SurfaceDescriptor {
        SurfaceDescriptor::new(self, window)
    }

    pub fn request_device_for(&self, surface: SurfaceDescriptor) -> (Device, Surface) {
        let (physical_device, queue_families, surface_config) =
            self.get_physical_device_and_info(&surface);
        let device = Device::new(self, physical_device, queue_families);
        (device, surface.with_config(surface_config))
    }

    fn get_physical_device_and_info(
        &self,
        surface: &SurfaceDescriptor,
    ) -> (vk::PhysicalDevice, QueueFamilies, SurfaceConfig) {
        let all_devices = unsafe {
            self.enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        if all_devices.is_empty() {
            panic!("Failed to find a physical device with Vulkan support");
        }

        let (physical_device, queue_families, surface_config_options) = all_devices
            .into_iter()
            .filter(|&physical_device| {
                self.has_required_device_extensions(physical_device)
                    && self.supports_required_features(physical_device)
            })
            .filter_map(|physical_device| {
                QueueFamilies::find(self, physical_device, surface)
                    .map(|queue_families| (physical_device, queue_families))
            })
            .map(|(physical_device, queue_families)| {
                (
                    physical_device,
                    queue_families,
                    surface.get_config_options_for(physical_device),
                )
            })
            .find(|(_, _, surface_config_options)| Self::is_suitable(surface_config_options))
            .expect("Failed to find a suitable physical device");

        (
            physical_device,
            queue_families,
            surface_config_options.get_optimal(),
        )
    }

    fn is_suitable(surface_config_options: &SurfaceConfigurationOptions) -> bool {
        surface_config_options.has_some()
    }

    fn has_required_device_extensions(&self, physical_device: vk::PhysicalDevice) -> bool {
        let available_extensions: HashSet<_> = unsafe {
            self.enumerate_device_extension_properties(physical_device)
                .expect("Failed to get device extension properties")
                .into_iter()
                .map(|e| util::bytes_to_string(e.extension_name.as_ptr()))
                .collect()
        };

        info::REQUIRED_DEVICE_EXTENSIONS
            .iter()
            .copied()
            .map(util::bytes_to_string)
            .all(|ref required_extension| available_extensions.contains(required_extension))
    }

    fn supports_required_features(&self, physical_device: vk::PhysicalDevice) -> bool {
        let features = self.get_supported_features(physical_device);
        features.v_1_2.vulkan_memory_model > 0
    }

    fn get_supported_features(&self, physical_device: vk::PhysicalDevice) -> Features {
        let mut supported = Features::default();
        unsafe { self.get_physical_device_features2(physical_device, &mut supported.v_1_0) };
        supported
    }
}

impl Destroy<()> for Instance {
    unsafe fn destroy_with(&mut self, _: ()) {
        self.validator.destroy_with(());
        self.instance.destroy_instance(None);
    }
}

impl Default for Features {
    fn default() -> Self {
        let mut v_1_1 = Box::<vk::PhysicalDeviceVulkan11Features>::default();
        let mut v_1_2 = Box::<vk::PhysicalDeviceVulkan12Features>::default();
        let v_1_0 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(v_1_2.as_mut())
            .push_next(v_1_1.as_mut())
            .build()
            .into();
        Self {
            v_1_0,
            v_1_1,
            v_1_2,
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
