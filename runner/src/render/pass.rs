use std::ops::Deref;

use ash::vk;

use crate::{context::Context, util::Destroy};

pub struct Pass {
    pass: vk::RenderPass,
}

impl Pass {
    pub fn create(ctx: &Context) -> Self {
        let color_attachments = [vk::AttachmentDescription::builder()
            .format(ctx.surface.config.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build()];

        let color_attachment_references = [vk::AttachmentReference::builder()
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        let subpasses = [vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_references)
            .build()];

        let dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build()];

        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&color_attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let pass = unsafe {
            ctx.create_render_pass(&create_info, None)
                .expect("Failed to create render pass")
        };

        Self { pass }
    }
}

impl<'a> Destroy<&'a Context> for Pass {
    unsafe fn destroy_with(&mut self, ctx: &'a Context) {
        ctx.destroy_render_pass(self.pass, None);
    }
}

impl Deref for Pass {
    type Target = vk::RenderPass;
    fn deref(&self) -> &Self::Target {
        &self.pass
    }
}
