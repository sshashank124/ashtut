use std::{
    collections::HashSet,
    ffi::{c_char, c_void, CStr},
};

use ash::vk;

use crate::util::{self, Destroy};

pub const VALIDATE_LAYERS: bool = cfg!(debug_assertions);
pub const VALIDATION_LAYERS: &[*const c_char] =
    unsafe { &[CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()] };

pub struct Validator {
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Validator {
    pub fn setup(
        entry: &ash::Entry,
        instance: &ash::Instance,
        debug_info: vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> Self {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        let debug_messenger = if !VALIDATE_LAYERS {
            vk::DebugUtilsMessengerEXT::null()
        } else {
            unsafe {
                debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .expect("Failed to create debug utils messenger")
            }
        };

        Self {
            debug_utils_loader,
            debug_messenger,
        }
    }

    pub fn check_validation_layer_support(entry: &ash::Entry) {
        if !VALIDATE_LAYERS {
            return;
        }

        let available_layers = entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate instance layers")
            .into_iter()
            .map(|l| util::bytes_to_string(l.layer_name.as_ptr()))
            .collect::<HashSet<_>>();

        for &req_layer in VALIDATION_LAYERS {
            let req_layer = util::bytes_to_string(req_layer);
            if !available_layers.contains(&req_layer) {
                panic!("Layer {req_layer} not found");
            }
        }
    }

    pub fn add_validation_to_instance<'a>(
        instance_create_info: vk::InstanceCreateInfoBuilder<'a>,
        debug_info: &'a mut vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> vk::InstanceCreateInfoBuilder<'a> {
        if !VALIDATE_LAYERS {
            return instance_create_info;
        }
        instance_create_info
            .enabled_layer_names(VALIDATION_LAYERS)
            .push_next(debug_info)
    }

    pub fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
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

impl Destroy<()> for Validator {
    unsafe fn destroy_with(&self, _: ()) {
        if VALIDATE_LAYERS {
            self.debug_utils_loader
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
    let severity = format!("{:?}", message_severity);
    let types = format!("{:?}", message_type);
    let message = util::bytes_to_string((*p_callback_data).p_message);
    println!("[{}][{}] {}", severity, types, message);
    vk::FALSE
}
