use std::slice;

use ash::vk;

use crate::gpu::{
    context::Context,
    descriptors::Descriptors,
    image::{format, Image},
    model::Model,
    scope::OneshotScope,
    uniforms::Uniforms,
    Destroy,
};

pub struct Data {
    pub descriptors: Descriptors,
    pub target: Image<{ format::HDR }>,
    pub uniforms: Uniforms,
    pub models: Vec<Model>,
}

impl Data {
    pub fn create(ctx: &mut Context, resolution: vk::Extent2D) -> Self {
        let descriptors = Self::create_descriptors(ctx);

        let target = {
            let info = vk::ImageCreateInfo {
                extent: resolution.into(),
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                ..Default::default()
            };
            Image::create(ctx, "Intermediate Target", &info)
        };

        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.graphics());
        let uniforms = Uniforms::create(ctx);
        let models = vec![Model::demo_viking_room(ctx, &mut init_scope)];
        init_scope.finish(ctx);

        let data = Self {
            descriptors,
            target,
            uniforms,
            models,
        };
        data.bind_to_descriptor_sets(ctx);
        data
    }

    pub fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::RAYGEN_KHR)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(
                        vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                    )
                    .build(),
            ];
            let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
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
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(slice::from_ref(&layout));
            unsafe {
                ctx.allocate_descriptor_sets(&info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Descriptors { layout, pool, sets }
    }

    fn bind_to_descriptor_sets(&self, ctx: &Context) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(*self.uniforms.buffer)
            .range(vk::WHOLE_SIZE);

        let texture_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.models[0].texture.image.view)
            .sampler(*self.models[0].texture.sampler);

        for &set in &self.descriptors.sets {
            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(slice::from_ref(&buffer_info))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(&texture_info))
                    .build(),
            ];

            unsafe {
                ctx.update_descriptor_sets(&writes, &[]);
            }
        }
    }
}

impl Destroy<Context> for Data {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.models.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);
        self.target.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
