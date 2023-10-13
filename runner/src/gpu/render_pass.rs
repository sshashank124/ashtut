use std::ops::Deref;

use ash::vk;

use super::{context::Context, Destroy};

pub struct RenderPass {
    render_pass: vk::RenderPass,
}

impl RenderPass {
    pub fn create(ctx: &Context, info: &vk::RenderPassCreateInfo) -> Self {
        let render_pass = unsafe {
            ctx.create_render_pass(info, None)
                .expect("Failed to create render pass")
        };

        Self { render_pass }
    }
}

impl Destroy<Context> for RenderPass {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_render_pass(self.render_pass, None);
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;
    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}
