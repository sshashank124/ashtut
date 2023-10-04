use std::{
    fs::File,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::util::Destroy;

use super::{features::Features, gpu_alloc, instance::Instance, queue};

pub mod conf {
    pub const REQUIRED_EXTENSIONS: &[*const std::ffi::c_char] = &[
        ash::extensions::khr::Swapchain::name().as_ptr(),
        ash::vk::KhrVulkanMemoryModelFn::name().as_ptr(),
    ];
}

pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    device: ash::Device,
    pub queue: queue::Queue,
    pub allocator: ManuallyDrop<gpu_alloc::Allocator>,
}

impl Device {
    pub fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        families: queue::Families,
    ) -> Self {
        let mut required_features = Features::required();
        let queue_create_infos = queue::Queue::create_infos(&families);
        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(conf::REQUIRED_EXTENSIONS)
            .push_next(required_features.v_1_0.as_mut());

        let device = unsafe {
            instance
                .create_device(physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };

        let queue = unsafe {
            queue::Queue {
                graphics: device.get_device_queue(families.graphics(), 0),
                present: device.get_device_queue(families.present(), 0),
                families,
            }
        };

        let allocator_create_info = gpu_alloc::AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: device.clone(),
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        };

        let allocator =
            gpu_alloc::Allocator::new(&allocator_create_info).expect("Failed to create allocator");

        Self {
            physical_device,
            device,
            queue,
            allocator: ManuallyDrop::new(allocator),
        }
    }

    pub fn create_command_pool(&self) -> vk::CommandPool {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(self.queue.families.graphics())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe {
            self.device
                .create_command_pool(&create_info, None)
                .expect("Failed to create command pool")
        }
    }

    pub fn create_semaphore(&self, name: &str) -> vk::Semaphore {
        let create_info = vk::SemaphoreCreateInfo::builder();
        unsafe {
            self.device
                .create_semaphore(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` semaphore: {}", name, err))
        }
    }

    pub fn create_fence(&self, name: &str, signaled: bool) -> vk::Fence {
        let create_info = vk::FenceCreateInfo::builder().flags(if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        });
        unsafe {
            self.device
                .create_fence(&create_info, None)
                .unwrap_or_else(|err| panic!("Failed to create `{}` fence: {}", name, err))
        }
    }

    pub fn create_shader_module_from_file(&self, filepath: &str) -> vk::ShaderModule {
        let shader_code = {
            let mut file = File::open(filepath).expect("Unable to open shader file");
            ash::util::read_spv(&mut file).expect("Unable to parse shader file")
        };
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);
        unsafe {
            self.create_shader_module(&create_info, None)
                .expect("Failed to create shader module")
        }
    }
}

impl Destroy<()> for Device {
    unsafe fn destroy_with(&mut self, _: ()) {
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
