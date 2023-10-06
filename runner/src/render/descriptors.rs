use ash::vk;

use crate::{context::Context, util::Destroy};

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
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        unsafe {
            ctx.device
                .create_descriptor_set_layout(&layout_info, None)
                .expect("Failed to create descriptor set layout")
        }
    }

    fn create_pool(ctx: &Context) -> vk::DescriptorPool {
        let num_uniform_buffers = ctx.surface.config.image_count;
        let sizes = [vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(num_uniform_buffers)
            .build()];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&sizes)
            .max_sets(num_uniform_buffers);
        unsafe {
            ctx.device
                .create_descriptor_pool(&info, None)
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
            ctx.device
                .allocate_descriptor_sets(&alloc_info)
                .expect("Failed to allocate descriptor sets")
        }
    }

    pub fn add_uniforms(&self, ctx: &Context, uniforms: &Uniforms) {
        for (&set, uniform_buffer) in self.sets.iter().zip(&uniforms.buffers) {
            let buffer_infos = [vk::DescriptorBufferInfo::builder()
                .buffer(**uniform_buffer)
                .range(vk::WHOLE_SIZE)
                .build()];

            let writes = [vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos)
                .build()];

            unsafe {
                ctx.device.update_descriptor_sets(&writes, &[]);
            }
        }
    }
}

impl<'a> Destroy<&'a mut Context> for Descriptors {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        ctx.device.destroy_descriptor_pool(self.pool, None);
        ctx.device.destroy_descriptor_set_layout(self.layout, None);
    }
}
