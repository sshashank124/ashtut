use ash::vk;

use super::{
    context::Context,
    image::{format, Image},
    render_pass::RenderPass,
    Destroy,
};

pub const CLEAR_VALUES: &[vk::ClearValue] = &[
    vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    },
    vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    },
];

pub struct Framebuffers<const FORMAT: vk::Format> {
    pub colors: Vec<Image<FORMAT>>,
    pub depth: Image<{ format::DEPTH }>,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl<const FORMAT: vk::Format> Framebuffers<{ FORMAT }> {
    pub fn create_for_images(
        ctx: &mut Context,
        name: &str,
        render_pass: &RenderPass,
        resolution: vk::Extent2D,
        colors: Vec<Image<{ FORMAT }>>,
    ) -> Self {
        let depth = {
            let info = vk::ImageCreateInfo {
                extent: resolution.into(),
                ..Default::default()
            };
            Image::create(ctx, &format!("{name} [DEPTH]"), &info)
        };

        let framebuffers = colors
            .iter()
            .map(|image| {
                let attachments = [image.view, depth.view];
                let info = vk::FramebufferCreateInfo::builder()
                    .render_pass(**render_pass)
                    .attachments(&attachments)
                    .width(resolution.width)
                    .height(resolution.height)
                    .layers(1);
                unsafe { ctx.create_framebuffer(&info, None) }
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to create framebuffers");

        Self {
            colors,
            depth,
            framebuffers,
        }
    }

    pub fn create_new(
        ctx: &mut Context,
        name: &str,
        render_pass: &RenderPass,
        resolution: vk::Extent2D,
    ) -> Self {
        let colors = {
            let info = vk::ImageCreateInfo {
                extent: resolution.into(),
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..Default::default()
            };
            vec![Image::create(ctx, &format!("{name} [COLOR]"), &info)]
        };

        Self::create_for_images(ctx, name, render_pass, resolution, colors)
    }
}

impl<const FORMAT: vk::Format> Destroy<Context> for Framebuffers<{ FORMAT }> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.framebuffers
            .iter()
            .for_each(|&fb| ctx.destroy_framebuffer(fb, None));
        self.depth.destroy_with(ctx);
        self.colors.destroy_with(ctx);
    }
}
