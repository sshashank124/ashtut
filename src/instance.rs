use std::{
    ffi::CString,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::{
    util::{info, Destroy},
    validator::{self, Validator},
};

pub struct Instance {
    pub entry: ash::Entry,
    instance: ash::Instance,
    validator: Validator,
}

impl Instance {
    pub fn create() -> Self {
        let entry = ash::Entry::linked();

        validator::check_validation_layer_support(&entry);

        let app_name = CString::new(info::WINDOW_TITLE).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .api_version(info::VK_API_VERSION);

        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(info::REQUIRED_EXTENSIONS);

        let mut debug_messenger_create_info = validator::debug_messenger_create_info();
        let instance_create_info = if validator::VALIDATE_LAYERS {
            instance_create_info
                .enabled_layer_names(validator::VALIDATION_LAYERS)
                .push_next(&mut debug_messenger_create_info)
        } else {
            instance_create_info
        };

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        let validator = Validator::setup(&entry, &instance);

        Self { entry, instance, validator }
    }
}

impl Destroy<()> for Instance {
    fn destroy_with(&self, _: ()) {
        unsafe {
            self.validator.destroy_with(());
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
