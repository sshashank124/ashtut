use ash::vk;

use super::{
    context::Context,
    image::{format, Image},
    sampled_image::SampledImage,
    sampler::Sampler,
    uniforms::Uniforms,
    Destroy,
};

pub struct Descriptors {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
}

impl Descriptors {
    pub fn bind_offscreen_descriptors(
        &self,
        ctx: &Context,
        uniforms: &Uniforms,
        sampled_image: &SampledImage<{ format::COLOR }>,
    ) {
        let buffer_infos = [vk::DescriptorBufferInfo::builder()
            .buffer(*uniforms.buffer)
            .range(vk::WHOLE_SIZE)
            .build()];

        let sampled_image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(sampled_image.image.view)
            .sampler(*sampled_image.sampler)
            .build()];

        let writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.sets[0])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build(),
            vk::WriteDescriptorSet::builder()
                .dst_set(self.sets[0])
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&sampled_image_info)
                .build(),
        ];

        unsafe {
            ctx.update_descriptor_sets(&writes, &[]);
        }
    }

    pub fn bind_tonemap_descriptors(
        &self,
        ctx: &Context,
        rendered_image: &Image<{ vk::Format::R32G32B32A32_SFLOAT }>,
        sampler: &Sampler,
    ) {
        for &set in &self.sets {
            let rendered_image_info = [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(rendered_image.view)
                .sampler(**sampler)
                .build()];

            let writes = [vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&rendered_image_info)
                .build()];

            unsafe {
                ctx.update_descriptor_sets(&writes, &[]);
            }
        }
    }
}

impl Destroy<Context> for Descriptors {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_descriptor_pool(self.pool, None);
        ctx.destroy_descriptor_set_layout(self.layout, None);
    }
}
