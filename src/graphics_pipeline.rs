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
    pub swapchain: Swapchain,
    pub render_pass: RenderPass,
    layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl GraphicsPipeline {
    pub fn create(instance: &Instance, physical_device: &PhysicalDevice, device: &Device) -> Self {
        let swapchain = Swapchain::create(instance, physical_device, device);
        let render_pass = RenderPass::create(device, swapchain.format);
        let (layout, pipeline) = Self::create_pipeline(device, &swapchain, &render_pass);
        let framebuffers = Self::create_framebuffers(device, &swapchain, &render_pass);

        Self {
            swapchain,
            render_pass,
            layout,
            pipeline,
            framebuffers,
        }
    }

    fn create_pipeline(
        device: &Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
    ) -> (vk::PipelineLayout, vk::Pipeline) {
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
            .width(swapchain.extent.width as f32)
            .height(swapchain.extent.height as f32)
            .max_depth(1.0)
            .build()];

        let scissors = [vk::Rect2D::builder().extent(swapchain.extent).build()];

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
        
        (layout, pipeline)
    }

    fn create_framebuffers(
        device: &Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
    ) -> Vec<vk::Framebuffer> {
        swapchain
            .image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(**render_pass)
                    .attachments(&attachments)
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .layers(1);
                unsafe {
                    device
                        .create_framebuffer(&create_info, None)
                        .expect("Failed to create framebuffer")
                }
            })
            .collect()
    }
}

impl<'a> Destroy<&'a Device> for GraphicsPipeline {
    fn destroy_with(&self, device: &'a Device) {
        unsafe {
            for &framebuffer in &self.framebuffers {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            self.render_pass.destroy_with(device);
            self.swapchain.destroy_with(device);
        }
    }
}