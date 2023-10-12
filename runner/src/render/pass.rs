use std::ops::Deref;

use ash::vk;

use crate::gpu::{
    context::Context,
    image::{self, HdrColor, Props},
    Destroy,
};

pub struct Pass {
    pass: vk::RenderPass,
}

impl Pass {
    pub fn create_for_offscreen(ctx: &Context) -> Self {
        let attachments = [
            vk::AttachmentDescription::builder()
                .format(HdrColor::FORMAT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(HdrColor::FINAL_LAYOUT)
                .build(),
            vk::AttachmentDescription::builder()
                .format(image::Depth::FORMAT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(image::Depth::FINAL_LAYOUT)
                .build(),
        ];

        let color_attachment_references = [vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .attachment(0)
            .build()];

        let depth_attachment_reference = vk::AttachmentReference::builder()
            .layout(image::Depth::FINAL_LAYOUT)
            .attachment(1);

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_references)
            .depth_stencil_attachment(&depth_attachment_reference)
            .build()];

        let dependencies = [
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::SHADER_READ)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build(),
        ];

        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let pass = unsafe {
            ctx.create_render_pass(&create_info, None)
                .expect("Failed to create offscreen pass")
        };

        Self { pass }
    }

    pub fn create_for_tonemap(ctx: &Context) -> Self {
        let attachments = [
            vk::AttachmentDescription::builder()
                .format(ctx.surface.config.surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build(),
            vk::AttachmentDescription::builder()
                .format(image::Depth::FORMAT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(image::Depth::FINAL_LAYOUT)
                .build(),
        ];

        let color_attachment_references = [vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .attachment(0)
            .build()];

        let depth_attachment_reference = vk::AttachmentReference::builder()
            .layout(image::Depth::FINAL_LAYOUT)
            .attachment(1);

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_references)
            .depth_stencil_attachment(&depth_attachment_reference)
            .build()];

        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses);

        let pass = unsafe {
            ctx.create_render_pass(&create_info, None)
                .expect("Failed to create tonemap pass")
        };

        Self { pass }
    }
}

impl Destroy<Context> for Pass {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_render_pass(self.pass, None);
    }
}

impl Deref for Pass {
    type Target = vk::RenderPass;
    fn deref(&self) -> &Self::Target {
        &self.pass
    }
}
