use std::{
    ffi::CString,
    mem,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::{
    util::{info, platform},
    validator::{self, Validator},
};

pub struct Instance {
    pub inner: ash::Instance,
    validator: mem::ManuallyDrop<Validator>,
}

impl Instance {
    pub fn new(entry: &ash::Entry) -> Self {
        validator::check_validation_layer_support(entry).unwrap();

        let app_name = CString::new(info::WINDOW_TITLE).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .api_version(info::VK_API_VERSION);
        
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(platform::REQUIRED_EXTENSION_NAMES);
        
        let mut debug_messenger_create_info = validator::debug_messenger_create_info();
        let instance_create_info = if validator::VALIDATE_LAYERS {
            instance_create_info
                .enabled_layer_names(validator::VALIDATION_LAYERS)
                .push_next(&mut debug_messenger_create_info)
        } else {
            instance_create_info
        };

        let inner = unsafe {
            entry.create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        let validator = mem::ManuallyDrop::new(Validator::setup(entry, &inner));

        Self {
            inner,
            validator,
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            mem::ManuallyDrop::drop(&mut self.validator);
            self.inner.destroy_instance(None);
        }
    }
}

impl Deref for Instance {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for Instance {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}