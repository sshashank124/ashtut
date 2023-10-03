use std::{fs::File, ops::Range};

use ash::vk;

use crate::device::Device;

pub mod info;
pub mod platform;

pub trait Descriptors {
    type BindingType;
    const NUM_BINDINGS: usize;
    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS];

    type AttributeType;
    const NUM_ATTRIBUTES: usize;
    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES];
}

pub trait Destroy<Input> {
    unsafe fn destroy_with(&mut self, input: Input);
}

pub fn create_shader_module_from_file(device: &Device, filepath: &str) -> vk::ShaderModule {
    let code = {
        let mut file = File::open(filepath).expect("Unable to open shader file");
        ash::util::read_spv(&mut file).expect("Unable to parse shader file")
    };
    let create_info = vk::ShaderModuleCreateInfo::builder().code(&code);

    unsafe {
        device
            .create_shader_module(&create_info, None)
            .expect("Failed to create shader module")
    }
}

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub fn solo_range(i: usize) -> Range<usize> {
    i..i + 1
}
