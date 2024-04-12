use std::slice;

use ash::vk;

use crate::{
    context::Context, descriptors::Descriptors, image, scope::OneshotScope, uniforms::Uniforms,
    world::Scene, Destroy,
};

mod conf {
    pub const NAME: &str = "Common";
    pub const MAX_NUM_TEXTURES: u32 = 128;
}

pub struct Data {
    pub descriptors: Descriptors,
    pub target: image::Image<{ image::Format::Hdr }>,
    pub resolution: vk::Extent2D,
    pub uniforms: Uniforms,
    pub scene: Scene,
}

impl Data {
    pub fn create(ctx: &mut Context, scene: scene::Scene, resolution: (u32, u32)) -> Self {
        let descriptors = Self::create_descriptors(ctx);

        let resolution = vk::Extent2D {
            width: resolution.0,
            height: resolution.1,
        };

        let mut init_scope = OneshotScope::begin_on(
            ctx,
            format!("{} Initialization", conf::NAME),
            ctx.queues.graphics(),
        );

        let target = {
            let info = vk::ImageCreateInfo {
                extent: resolution.into(),
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                ..Default::default()
            };
            image::Image::create(
                ctx,
                &init_scope,
                format!("{} Target", conf::NAME),
                &info,
                Some(&image::BarrierInfo::GENERAL),
            )
        };

        let scene = Scene::create(ctx, &mut init_scope, scene);

        let uniforms = Uniforms::create(ctx);

        init_scope.finish(ctx);

        let data = Self {
            descriptors,
            target,
            resolution,
            uniforms,
            scene,
        };
        data.bind_to_descriptor_sets(ctx);
        data
    }

    pub fn create_descriptors(ctx: &Context) -> Descriptors {
        let layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::RAYGEN_KHR),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(
                        vk::ShaderStageFlags::FRAGMENT
                            | vk::ShaderStageFlags::RAYGEN_KHR
                            | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                    ),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(conf::MAX_NUM_TEXTURES)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::RAYGEN_KHR),
            ];
            let binding_flags = [
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::PARTIALLY_BOUND,
            ];
            let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
                .binding_flags(&binding_flags);
            let info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings)
                .push_next(&mut binding_flags_info);
            unsafe {
                ctx.create_descriptor_set_layout(&info, None)
                    .expect("Failed to create descriptor set layout")
            }
        };

        let pool = {
            let sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(conf::MAX_NUM_TEXTURES),
            ];
            let info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&sizes)
                .max_sets(1);
            unsafe {
                ctx.create_descriptor_pool(&info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let sets = {
            let info = vk::DescriptorSetAllocateInfo::default()
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
        let uniforms_info = vk::DescriptorBufferInfo::default()
            .buffer(*self.uniforms.buffer)
            .range(vk::WHOLE_SIZE);

        let scene_desc_info = vk::DescriptorBufferInfo::default()
            .buffer(*self.scene.scene_desc)
            .range(vk::WHOLE_SIZE);

        let textures_info: Vec<_> = self
            .scene
            .textures
            .iter()
            .map(|tex| {
                vk::DescriptorImageInfo::default()
                    .image_view(tex.view)
                    .image_layout(image::BarrierInfo::SHADER_READ.layout)
                    .sampler(*tex.sampler)
            })
            .collect();

        for &set in &self.descriptors.sets {
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(slice::from_ref(&uniforms_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(slice::from_ref(&scene_desc_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&textures_info),
            ];

            unsafe {
                ctx.update_descriptor_sets(&writes, &[]);
            }
        }
    }
}

impl Destroy<Context> for Data {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.scene.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);
        self.target.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
