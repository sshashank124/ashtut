use std::{
    fs::File,
    ops::{Deref, DerefMut},
};

use ash::vk;

use crate::{device::Device, util::Destroy};

pub struct ShaderModule {
    inner: vk::ShaderModule,
}

impl ShaderModule {
    pub fn create_from_file(device: &Device, filepath: &str) -> Self {
        let code = {
            let mut file = File::open(filepath).expect("Unable to open shader file");
            ash::util::read_spv(&mut file).expect("Unable to parse shader file")
        };
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&code);

        let inner = unsafe {
            device
                .create_shader_module(&create_info, None)
                .expect("Failed to create shader module")
        };

        Self { inner }
    }
}

impl<'a> Destroy<&'a Device> for ShaderModule {
    fn destroy_with(&self, device: &'a Device) {
        unsafe { device.destroy_shader_module(self.inner, None) };
    }
}

impl Deref for ShaderModule {
    type Target = vk::ShaderModule;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ShaderModule {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
