use ash::vk;

use crate::gpu::{context::Context, sampled_image::SampledImage, Destroy};

use super::uniforms::Uniforms;

pub struct Descriptors {
    pub layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
}

impl Descriptors {
    pub fn create(ctx: &Context) -> Self {
        let layout = Self::create_layout(ctx);
        let pool = Self::create_pool(ctx);
        let sets = Self::create_sets(ctx, layout, pool);

        Self { layout, pool, sets }
    }

    fn create_layout(ctx: &Context) -> vk::DescriptorSetLayout {
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
    }

    fn create_pool(ctx: &Context) -> vk::DescriptorPool {
        let num_frames = ctx.surface.config.image_count;
        let sizes = [
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(num_frames)
                .build(),
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(num_frames)
                .build(),
        ];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&sizes)
            .max_sets(num_frames);
        unsafe {
            ctx.create_descriptor_pool(&info, None)
                .expect("Failed to create descriptor pool")
        }
    }

    fn create_sets(
        ctx: &Context,
        layout: vk::DescriptorSetLayout,
        pool: vk::DescriptorPool,
    ) -> Vec<vk::DescriptorSet> {
        let layouts = vec![layout; ctx.surface.config.image_count as usize];

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        unsafe {
            ctx.allocate_descriptor_sets(&alloc_info)
                .expect("Failed to allocate descriptor sets")
        }
    }

    pub fn bind_descriptors(
        &self,
        ctx: &Context,
        uniforms: &Uniforms,
        sampled_image: &SampledImage,
    ) {
        for (&set, uniform_buffer) in self.sets.iter().zip(&uniforms.buffers) {
            let buffer_infos = [vk::DescriptorBufferInfo::builder()
                .buffer(**uniform_buffer)
                .range(vk::WHOLE_SIZE)
                .build()];

            let sampled_image_info = [vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(sampled_image.image.view)
                .sampler(*sampled_image.sampler)
                .build()];

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&buffer_infos)
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&sampled_image_info)
                    .build(),
            ];

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
