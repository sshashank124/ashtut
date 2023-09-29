use std::{
    ffi::{CStr, c_char, c_void},
    collections::HashSet
};

use ash::vk;
use thiserror::Error;

pub const VALIDATE_LAYERS: bool = cfg!(debug_assertions);
pub const VALIDATION_LAYERS: &[*const c_char] = unsafe {&[
    CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr(),
]};

pub struct Validator {
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

#[derive(Error, Debug)] pub enum CheckValidationLayerError {
    #[error("Failed to enumerate instance layer properties")]
    CouldNotEnumerate(#[from] ash::vk::Result),
    #[error("Failed to parse raw string")]
    InvalidLayerName(#[from] std::str::Utf8Error),
    #[error("No layers found")]
    NoLayersFound,
    #[error("Layer {0} not found")]
    LayerNotFound(String),
}

impl Validator {
    pub fn setup(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Self {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        let debug_messenger = if !VALIDATE_LAYERS {
            vk::DebugUtilsMessengerEXT::null()
        } else {
            let create_info = debug_messenger_create_info();
            unsafe {
                debug_utils_loader.create_debug_utils_messenger(&create_info, None)
                    .expect("Failed to create debug utils messenger")
            }
        };
        
        Self {
            debug_utils_loader,
            debug_messenger,
        }
    }
}

impl Drop for Validator {
    fn drop(&mut self) {
        if VALIDATE_LAYERS {
            unsafe {
                self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
            }
        }
    }
}

pub fn check_validation_layer_support(entry: &ash::Entry) -> Result<(), CheckValidationLayerError> {
    if VALIDATE_LAYERS {
        let available_layers = entry.enumerate_instance_layer_properties()?
            .into_iter()
            .map(|l| Ok(unsafe { CStr::from_ptr(l.layer_name.as_ptr()) }.to_str()?.to_owned()))
            .collect::<Result<HashSet<_>, CheckValidationLayerError>>()?;
        
        if available_layers.is_empty() {
            return Err(CheckValidationLayerError::NoLayersFound);
        }
        
        for req_layer in VALIDATION_LAYERS {
            let req_layer = unsafe { CStr::from_ptr(*req_layer) }.to_str()?;
            if !available_layers.contains(req_layer) {
                return Err(CheckValidationLayerError::LayerNotFound(req_layer.to_owned()));
            }
        }
    }

    Ok(())
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = format!("{:?}", message_severity);
    let types = format!("{:?}", message_type);
    let message = CStr::from_ptr((*p_callback_data).p_message);
    println!("[Debug][{}][{}] {:?}", severity, types, message);
    vk::FALSE
}

pub fn debug_messenger_create_info<'a>() -> vk::DebugUtilsMessengerCreateInfoEXTBuilder<'a> {
    vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                          | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                          // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                          // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        ).message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                      | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                      | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION)
        .pfn_user_callback(Some(debug_callback))
}
