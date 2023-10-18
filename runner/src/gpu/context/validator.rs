use std::collections::HashSet;

use ash::vk;

mod conf {
    pub const VALIDATE_LAYERS: bool = cfg!(debug_assertions);
    pub const VALIDATION_LAYERS: &[*const std::ffi::c_char] = unsafe {
        &[
            std::ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
                .as_ptr(),
        ]
    };
}

pub struct Validator {
    _debug_utils_loader: ash::extensions::ext::DebugUtils,
}

impl Validator {
    pub fn setup(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

        Self {
            _debug_utils_loader: debug_utils_loader,
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

    pub fn add_validation_to_instance(
        instance_create_info: vk::InstanceCreateInfoBuilder<'_>,
    ) -> vk::InstanceCreateInfoBuilder<'_> {
        if !conf::VALIDATE_LAYERS {
            return instance_create_info;
        }
        instance_create_info.enabled_layer_names(conf::VALIDATION_LAYERS)
    }
}
