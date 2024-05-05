use std::{
    fs::File,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use ash::vk;

use super::{
    extensions, features,
    instance::Instance,
    physical_device::PhysicalDevice,
    queue::{Families, Queues},
};

pub struct Device {
    device: ash::Device,
    pub ext: extensions::Handles,
    pub queues: Queues,
    pub allocator: ManuallyDrop<vk_mem::Allocator>,
}

impl Device {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        families: &Families,
    ) -> Self {
        firestorm::profile_method!(create);

        let device = {
            let (required_features, mut additional_required_features) = features::required();
            let mut required_features = additional_required_features
                .iter_mut()
                .fold(required_features, |acc_features, f| {
                    acc_features.push_next(f.as_mut())
                });

            let queue_create_infos = Queues::create_infos(families);

            let create_info = vk::DeviceCreateInfo::default()
                .enabled_extension_names(extensions::REQUIRED_FOR_DEVICE)
                .push_next(&mut required_features)
                .queue_create_infos(&queue_create_infos);

            unsafe {
                instance
                    .create_device(**physical_device, &create_info, None)
                    .expect("Failed to create logical device")
            }
        };

        let ext = extensions::Handles::create(instance, &device);

        let queues = Queues::create(&device, families);

        let allocator = {
            let mut create_info =
                vk_mem::AllocatorCreateInfo::new(instance, &device, **physical_device);
            create_info.vulkan_api_version = crate::conf::VK_API_VERSION;
            create_info.flags = vk_mem::AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION
                | vk_mem::AllocatorCreateFlags::KHR_BIND_MEMORY2
                | vk_mem::AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS
                | vk_mem::AllocatorCreateFlags::EXT_MEMORY_PRIORITY;

            ManuallyDrop::new(
                unsafe { vk_mem::Allocator::new(create_info) }.expect("Failed to create allocator"),
            )
        };

        Self {
            device,
            ext,
            queues,
            allocator,
        }
    }

    pub fn create_semaphore(&self, name: &str) -> vk::Semaphore {
        firestorm::profile_method!(create_semaphore);

        let semaphore = {
            let create_info = vk::SemaphoreCreateInfo::default();

            unsafe {
                self.device
                    .create_semaphore(&create_info, None)
                    .unwrap_or_else(|err| panic!("Failed to create `{name}` semaphore: {err}"))
            }
        };
        self.set_debug_name(semaphore, name);

        semaphore
    }

    pub fn create_fence(&self, name: &str, signaled: bool) -> vk::Fence {
        firestorm::profile_method!(create_fence);

        let fence = {
            let create_info = vk::FenceCreateInfo::default().flags(if signaled {
                vk::FenceCreateFlags::SIGNALED
            } else {
                vk::FenceCreateFlags::empty()
            });

            unsafe {
                self.device
                    .create_fence(&create_info, None)
                    .unwrap_or_else(|err| panic!("Failed to create `{name}` fence: {err}"))
            }
        };
        self.set_debug_name(fence, name);

        fence
    }

    pub fn create_shader_module_from_file(&self, filepath: &str) -> vk::ShaderModule {
        firestorm::profile_method!(create_shader_module_from_file);

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
        firestorm::profile_method!(wait_idle);

        self.device_wait_idle()
            .expect("Failed to wait for device to idle");
    }

    pub fn set_debug_name<H: vk::Handle>(&self, object: H, name: &str) {
        let object_name = std::ffi::CString::new(name).unwrap();
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

impl Drop for Device {
    fn drop(&mut self) {
        firestorm::profile_method!(drop);

        unsafe {
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
        }
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
