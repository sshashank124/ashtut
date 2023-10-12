use ash::vk;

use crate::gpu::{
    context::Context,
    sampled_image::{SampledColorImage, SampledHdrImage},
    Destroy,
};

use super::uniforms::Uniforms;

pub struct Descriptors {
    pub layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
}

impl Descriptors {
    pub fn create_offscreen(ctx: &Context) -> Self {
        let layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ];
            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
            unsafe {
                ctx.create_descriptor_set_layout(&layout_info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let sizes = [
                vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .build(),
                vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .build(),
            ];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&sizes)
                .max_sets(1);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let layouts = [layout];
            let alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            unsafe {
                ctx.allocate_descriptor_sets(&alloc_info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Self { layout, pool, sets }
    }

    pub fn create_tonemap(ctx: &Context) -> Self {
        let layout = {
            let bindings = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build()];
            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
            unsafe {
                ctx.create_descriptor_set_layout(&layout_info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let num_frames = ctx.surface.config.image_count;
            let sizes = [vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(num_frames)
                .build()];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&sizes)
                .max_sets(num_frames);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let layouts = vec![layout; ctx.surface.config.image_count as usize];
            let alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            unsafe {
                ctx.allocate_descriptor_sets(&alloc_info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Self { layout, pool, sets }
    }

    pub fn bind_offscreen_descriptors(
        &self,
        ctx: &Context,
        uniforms: &Uniforms,
        sampled_image: &SampledColorImage,
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

    pub fn bind_tonemap_descriptors(&self, ctx: &Context, rendered_image: &SampledHdrImage) {
        for &set in &self.sets {
            let rendered_image_info = [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(rendered_image.image.view)
                .sampler(*rendered_image.sampler)
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
