pub mod pathtracer;
pub mod rasterizer;
pub mod tonemap;

use std::slice;

use ash::vk;
use shared::inputs;

use crate::{
    commands::Commands, context::Context, descriptors::Descriptors, image, memory,
    uniforms::Uniforms, world::World, Destroy,
};

mod conf {
    pub const MAX_NUM_TEXTURES: u32 = 128;
}

pub struct Data<const FORMAT: image::Format> {
    pub descriptors: Descriptors,
    pub uniforms: Uniforms,
    pub world: World,
    pub target: image::Image<FORMAT>,
}

impl<const FORMAT: image::Format> Data<FORMAT> {
    pub fn create(
        ctx: &Context,
        scene: scene::Scene,
        resolution: (u32, u32),
        camera: inputs::Camera,
    ) -> Self {
        firestorm::profile_method!(create);

        let descriptors = Self::create_descriptors(ctx);
        let uniforms = Uniforms::create(ctx, camera);
        let world = World::create(ctx, scene);

        let commands = Commands::begin_on_queue(
            ctx,
            "Common - Initialization".to_owned(),
            ctx.queues.graphics(),
        );

        let target = {
            let info = vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width: resolution.0,
                    height: resolution.1,
                    depth: 1,
                },
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                ..Default::default()
            };
            image::Image::create(
                ctx,
                commands.buffer,
                "Render Target".to_owned(),
                &info,
                &memory::purpose::dedicated(),
                Some(&image::BarrierInfo::GENERAL),
            )
        };

        commands.finish(ctx, &vk::SubmitInfo::default(), None);

        let data = Self {
            descriptors,
            uniforms,
            world,
            target,
        };
        data.bind_to_descriptor_sets(ctx);
        data
    }

    pub fn create_descriptors(ctx: &Context) -> Descriptors {
        firestorm::profile_method!(create_descriptors);

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
                    .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(3)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(4)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(conf::MAX_NUM_TEXTURES)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::RAYGEN_KHR),
            ];
            let binding_flags = [
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
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
                    .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
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
            let mut set_counts = vk::DescriptorSetVariableDescriptorCountAllocateInfo::default()
                .descriptor_counts(&[conf::MAX_NUM_TEXTURES]);

            let info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(slice::from_ref(&layout))
                .push_next(&mut set_counts);

            unsafe {
                ctx.allocate_descriptor_sets(&info)
                    .expect("Failed to allocate descriptor sets")
            }
        };

        Descriptors { layout, pool, sets }
    }

    fn bind_to_descriptor_sets(&self, ctx: &Context) {
        firestorm::profile_method!(bind_to_descriptor_sets);

        let uniforms_info = self.uniforms.buffer_info();

        let scene_desc_info = vk::DescriptorBufferInfo::default()
            .buffer(*self.world.scene_desc)
            .range(vk::WHOLE_SIZE);

        let mut accel_info = vk::WriteDescriptorSetAccelerationStructureKHR::default()
            .acceleration_structures(slice::from_ref(&self.world.accel.tlas));

        let target_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(self.target.view);

        let textures_info: Vec<_> = self
            .world
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
                    .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .push_next(&mut accel_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(3)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(slice::from_ref(&target_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(4)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&textures_info),
            ];

            unsafe {
                ctx.update_descriptor_sets(&writes, &[]);
            }
        }
    }
}

impl<const FORMAT: image::Format> Destroy<Context> for Data<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.target.destroy_with(ctx);
        self.world.destroy_with(ctx);
        self.uniforms.destroy_with(ctx);
        self.descriptors.destroy_with(ctx);
    }
}
