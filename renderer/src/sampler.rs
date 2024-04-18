use std::ops::{Deref, DerefMut};

use ash::vk;

use super::{context::Context, Destroy};

pub struct Sampler {
    sampler: vk::Sampler,
}

impl Sampler {
    pub fn create(ctx: &Context, name: String) -> Self {
        let info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(
                ctx.physical_device
                    .properties
                    .v_1_0
                    .limits
                    .max_sampler_anisotropy,
            )
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS);

        let sampler = unsafe {
            ctx.create_sampler(&info, None)
                .expect("Failed to create sampler")
        };
        ctx.set_debug_name(sampler, &(name + " - Sampler"));

        Self { sampler }
    }
}

impl Destroy<Context> for Sampler {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        ctx.destroy_sampler(self.sampler, None);
    }
}

impl Deref for Sampler {
    type Target = vk::Sampler;
    fn deref(&self) -> &Self::Target {
        &self.sampler
    }
}

impl DerefMut for Sampler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sampler
    }
}
