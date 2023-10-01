use ash::vk;

use crate::{
    device::Device,
    instance::Instance,
    physical_device::PhysicalDevice,
    render_pass::RenderPass,
    shader_module::ShaderModule,
    swapchain::Swapchain,
    util::{info, Destroy},
};

pub struct GraphicsPipeline {
    pub render_pass: RenderPass,
    pub swapchain: Swapchain,
    pub pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl GraphicsPipeline {
    pub fn create(instance: &Instance, physical_device: &PhysicalDevice, device: &Device) -> Self {
        let surface_config = physical_device.get_surface_config_options().get_optimal();

        let render_pass = RenderPass::create(device, surface_config.surface_format.format);
        let swapchain = Swapchain::create(
            instance,
            physical_device,
            device,
            &render_pass,
            surface_config,
        );
        let (pipeline, layout) = Self::create_pipeline(device, &swapchain, &render_pass);

        Self {
            render_pass,
            swapchain,
            pipeline,
            layout,
        }
    }

    fn create_pipeline(
        device: &Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let shader_module = ShaderModule::create_from_file(device, info::SHADER_FILE);

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(*shader_module)
                .name(info::VERTEX_SHADER_ENTRY_POINT)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(*shader_module)
                .name(info::FRAGMENT_SHADER_ENTRY_POINT)
                .build(),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewports = [vk::Viewport::builder()
            .width(swapchain.config.extent.width as f32)
            .height(swapchain.config.extent.height as f32)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder()
            .extent(swapchain.config.extent)
            .build()];

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build()];

        let color_blend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder();

        let layout = unsafe {
            device
                .create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create graphics pipeline layout")
        };

        let create_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .layout(layout)
            .render_pass(**render_pass)
            .build()];

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create graphics pipeline")
        }[0];

        shader_module.destroy_with(device);

        (pipeline, layout)
    }
}

impl<'a> Destroy<&'a Device> for GraphicsPipeline {
    fn destroy_with(&self, device: &'a Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            self.swapchain.destroy_with(device);
            self.render_pass.destroy_with(device);
        }
    }
}
