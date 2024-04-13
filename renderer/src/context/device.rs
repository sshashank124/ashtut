use std::{
    fs::File,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use ash::vk;
use gpu_allocator::vulkan as gpu_alloc;

use crate::Destroy;

use super::{
    extensions,
    features::Features,
    instance::Instance,
    physical_device::PhysicalDevice,
    queue::{Families, Queues},
};

pub struct Device {
    device: ash::Device,
    pub ext: extensions::Handles,
    pub queues: Queues,
    pub allocator: ManuallyDrop<gpu_alloc::Allocator>,
}

impl Device {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        families: &Families,
    ) -> Self {
        let (required_features, mut additional_required_features) = Features::required();
        let mut required_features = additional_required_features
            .iter_mut()
            .fold(required_features, |acc_features, f| acc_features.push_next(f.as_mut()));

        let queue_create_infos = Queues::create_infos(families);

        let create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(extensions::REQUIRED_FOR_DEVICE)
            .push_next(&mut required_features)
            .queue_create_infos(&queue_create_infos);

        let device = unsafe {
            instance
                .create_device(**physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };

        let ext = extensions::Handles::create(instance, &device);

        let queues = Queues::create(&device, families);

        let allocator_create_info = gpu_alloc::AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: device.clone(),
            physical_device: **physical_device,
            debug_settings: gpu_allocator::AllocatorDebugSettings {
                log_memory_information: true,
                log_leaks_on_shutdown: true,
                store_stack_traces: true,
                log_allocations: true,
                log_frees: true,
                log_stack_traces: true,
            },
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        };

        let allocator =
            gpu_alloc::Allocator::new(&allocator_create_info).expect("Failed to create allocator");

        Self {
            device,
            ext,
            queues,
            allocator: ManuallyDrop::new(allocator),
        }
    }

    pub fn create_semaphore(&self, name: impl AsRef<str>) -> vk::Semaphore {
        let create_info = vk::SemaphoreCreateInfo::default();
        unsafe {
            self.device
                .create_semaphore(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` semaphore: {err}", name.as_ref()))
        }
    }

    pub fn create_fence(&self, name: impl AsRef<str>, signaled: bool) -> vk::Fence {
        let create_info = vk::FenceCreateInfo::default().flags(if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        });
        unsafe {
            self.device
                .create_fence(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` fence: {err}", name.as_ref()))
        }
    }

    pub fn create_shader_module_from_file(&self, filepath: &str) -> vk::ShaderModule {
        let shader_code = {
            let mut file = File::open(filepath).expect("Unable to open shader file");
            ash::util::read_spv(&mut file).expect("Unable to parse shader file")
        };
        let create_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);
        unsafe {
            self.create_shader_module(&create_info, None)
                .expect("Failed to create shader module")
        }
    }

    pub unsafe fn wait_idle(&self) {
        self.device_wait_idle()
            .expect("Failed to wait for device to idle");
    }

    pub fn set_debug_name<H: vk::Handle>(&self, object: H, name: impl AsRef<str>) {
        let object_name = std::ffi::CString::new(name.as_ref()).unwrap();
        let name_info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(object)
            .object_name(&object_name);

        unsafe {
            self.ext
                .debug_utils
                .set_debug_utils_object_name(&name_info)
                .expect("Failed to set object debug name");
        }
    }
}

impl Destroy<()> for Device {
    unsafe fn destroy_with(&mut self, (): &mut ()) {
        ManuallyDrop::drop(&mut self.allocator);
        self.device.destroy_device(None);
    }
}

impl Deref for Device {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for Device {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}
