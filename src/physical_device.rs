use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};

use ash::vk;
use winit::window::Window;

use crate::{
    features::Features,
    instance::Instance,
    surface::{Surface, SurfaceConfigurationOptions},
    util::{self, info, Destroy},
};

pub struct PhysicalDevice {
    pub surface: Surface,
    physical_device: vk::PhysicalDevice,
    pub indices: QueueFamilyIndices,
}

pub struct QueueFamilyIndices {
    families: [u32; 2],
}

#[derive(Default)]
struct QueueFamilyIndicesInfo {
    graphics: Option<u32>,
    present: Option<u32>,
}

impl PhysicalDevice {
    pub fn pick(instance: &Instance, window: &Window) -> Self {
        let surface = Surface::create(instance, window);

        let all_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
        };

        if all_devices.is_empty() {
            panic!("Failed to find a physical device with Vulkan support");
        }

        let (physical_device, indices) = all_devices
            .into_iter()
            .filter(|&physical_device| {
                Self::has_required_device_extensions(instance, physical_device)
                    && Self::supports_required_features(instance, physical_device)
            })
            .map(|physical_device| {
                (
                    physical_device,
                    Self::find_queue_families(instance, physical_device, &surface),
                )
            })
            .find(|(physical_device, indices)| {
                Self::is_suitable(
                    instance,
                    *physical_device,
                    indices,
                    surface.get_config_options(*physical_device),
                )
            })
            .expect("Failed to find a suitable physical device");

        Self {
            surface,
            physical_device,
            indices: indices.into(),
        }
    }

    fn is_suitable(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        indices: &QueueFamilyIndicesInfo,
        surface_details: SurfaceConfigurationOptions,
    ) -> bool {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        let is_discrete_gpu = properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
        let has_queue_family_support = indices.is_complete();
        let has_swapchain_support = surface_details.is_populated();

        is_discrete_gpu && has_queue_family_support && has_swapchain_support
    }

    fn has_required_device_extensions(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> bool {
        let available_extensions: HashSet<_> = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device)
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

    fn supports_required_features(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> bool {
        let features = Features::get_supported(instance, physical_device);
        features.v_1_2.vulkan_memory_model > 0
    }

    fn find_queue_families(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface: &Surface,
    ) -> QueueFamilyIndicesInfo {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let valid_queue_families = queue_families
            .into_iter()
            .enumerate()
            .filter(|(_, queue_family)| queue_family.queue_count > 0);

        let mut indices = QueueFamilyIndicesInfo::default();
        for (index, queue_family) in valid_queue_families {
            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics = Some(index as u32);
            }
            let has_surface_support = unsafe {
                surface
                    .loader
                    .get_physical_device_surface_support(physical_device, index as u32, **surface)
                    .expect("Failed to get physical device surface support info")
            };
            if has_surface_support {
                indices.present = Some(index as u32);
            }

            if indices.is_complete() {
                break;
            }
        }

        indices
    }
    
    pub fn get_surface_config_options(&self) -> SurfaceConfigurationOptions {
        self.surface.get_config_options(self.physical_device)
    }
}

impl Destroy<()> for PhysicalDevice {
    fn destroy_with(&self, _: ()) {
        self.surface.destroy_with(());
    }
}

impl QueueFamilyIndices {
    pub fn graphics(&self) -> u32 {
        self.families[0]
    }
    pub fn present(&self) -> u32 {
        self.families[1]
    }
    pub fn separate_graphics_and_presentation_indices(&self) -> Option<&[u32]> {
        if self.graphics() == self.present() {
            None
        } else {
            Some(&self.families[..2])
        }
    }
    pub fn unique_queue_family_indices(&self) -> HashSet<u32> {
        HashSet::from_iter(self.families)
    }
}

impl From<QueueFamilyIndicesInfo> for QueueFamilyIndices {
    fn from(value: QueueFamilyIndicesInfo) -> Self {
        Self {
            families: [value.graphics.unwrap(), value.present.unwrap()],
        }
    }
}

impl QueueFamilyIndicesInfo {
    fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}

impl Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;
    fn deref(&self) -> &Self::Target {
        &self.physical_device
    }
}

impl DerefMut for PhysicalDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.physical_device
    }
}
