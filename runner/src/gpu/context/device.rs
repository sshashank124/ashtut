use std::{
    fs::File,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use ash::vk;

use super::{
    super::alloc,
    extensions,
    features::Features,
    instance::Instance,
    physical_device::PhysicalDevice,
    queue::{Families, Queues},
    Destroy,
};

pub struct Device {
    device: ash::Device,
    pub ext: extensions::Handles,
    pub queues: Queues,
    pub allocator: ManuallyDrop<alloc::Allocator>,
}

impl Device {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        families: &Families,
    ) -> Self {
        let mut required_features = Features::required();

        let queue_create_infos = Queues::create_infos(families);

        let create_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(extensions::REQUIRED_FOR_DEVICE)
            .push_next(required_features.v_1_0.as_mut())
            .queue_create_infos(&queue_create_infos);

        let device = unsafe {
            instance
                .create_device(**physical_device, &create_info, None)
                .expect("Failed to create logical device")
        };

        let ext = extensions::Handles::create(instance, &device);

        let queues = Queues::create(&device, families);

        let allocator_create_info = alloc::AllocatorCreateDesc {
            instance: (*instance).clone(),
            device: device.clone(),
            physical_device: **physical_device,
            debug_settings: Default::default(),
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        };

        let allocator =
            alloc::Allocator::new(&allocator_create_info).expect("Failed to create allocator");

        Self {
            device,
            ext,
            queues,
            allocator: ManuallyDrop::new(allocator),
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

    pub unsafe fn wait_idle(&self) {
        self.device_wait_idle()
            .expect("Failed to wait for device to idle");
    }
}

impl Destroy<()> for Device {
    unsafe fn destroy_with(&mut self, _: &mut ()) {
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
