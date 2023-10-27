use std::{collections::HashSet, ffi::c_void};

use ash::vk;

use super::Destroy;

mod conf {
    pub const VALIDATE_LAYERS: bool = cfg!(debug_assertions);
    pub const VALIDATION_LAYERS: &[*const std::ffi::c_char] = unsafe {
        &[
            std::ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
                .as_ptr(),
        ]
    };
}

pub struct Debug {
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Debug {
    pub fn setup(
        entry: &ash::Entry,
        instance: &ash::Instance,
        debug_info: vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> Self {
        let debug_utils = ash::extensions::ext::DebugUtils::new(entry, instance);

        let debug_messenger = if conf::VALIDATE_LAYERS {
            unsafe {
                debug_utils
                    .create_debug_utils_messenger(&debug_info, None)
                    .expect("Failed to create debug utils messenger")
            }
        } else {
            vk::DebugUtilsMessengerEXT::null()
        };

        Self {
            debug_utils,
            debug_messenger,
        }
    }

    pub fn check_validation_layer_support(entry: &ash::Entry) {
        if !conf::VALIDATE_LAYERS {
            return;
        }

        let available_layers = entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate instance layers")
            .into_iter()
            .map(|l| crate::util::bytes_to_string(l.layer_name.as_ptr()))
            .collect::<HashSet<_>>();

        for &req_layer in conf::VALIDATION_LAYERS {
            let req_layer = crate::util::bytes_to_string(req_layer);
            assert!(
                available_layers.contains(&req_layer),
                "Layer {req_layer} not found"
            );
        }
    }

    pub fn add_validation_to_instance<'a>(
        instance_create_info: vk::InstanceCreateInfoBuilder<'a>,
        debug_info: &'a mut vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> vk::InstanceCreateInfoBuilder<'a> {
        if !conf::VALIDATE_LAYERS {
            return instance_create_info;
        }
        instance_create_info
            .enabled_layer_names(conf::VALIDATION_LAYERS)
            .push_next(debug_info)
    }

    pub fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(debug_callback))
            .build()
    }
}

impl Destroy<()> for Debug {
    unsafe fn destroy_with(&mut self, _: &mut ()) {
        if conf::VALIDATE_LAYERS {
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_messenger, None);
        }
    }
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message = crate::util::bytes_to_string((*p_callback_data).p_message);
    println!("[{message_severity:?}][{message_type:?}] {message}");
    vk::FALSE
}
