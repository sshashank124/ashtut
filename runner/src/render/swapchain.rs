use ash::{extensions::khr, vk};

use crate::gpu::{context::Context, image::DepthImage, scope::TempScope, Destroy};

use super::pass::Pass;

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub loader: khr::Swapchain,
    pub image_views: Vec<vk::ImageView>,
    pub depth_image: DepthImage,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl Swapchain {
    pub fn create(ctx: &mut Context, scope: &mut TempScope, pass: &Pass) -> Self {
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(**ctx.surface)
            .min_image_count(ctx.surface.config.image_count)
            .image_format(ctx.surface.config.surface_format.format)
            .image_color_space(ctx.surface.config.surface_format.color_space)
            .image_extent(ctx.surface.config.extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(ctx.surface.config.present_mode)
            .clipped(true);

        let loader = khr::Swapchain::new(&ctx.instance, ctx);

        let swapchain = unsafe {
            loader
                .create_swapchain(&create_info, None)
                .expect("Failed to create swapchain")
        };

        let images = unsafe {
            loader
                .get_swapchain_images(swapchain)
                .expect("Failed to get swapchain images")
        };

        ctx.surface.config.image_count = images.len() as u32;

        let image_views = Self::create_image_views(ctx, &images);

        let depth_image = DepthImage::init(ctx, scope, "Z-Buffer");

        let framebuffers = Self::create_framebuffers(ctx, **pass, &image_views, &depth_image);

        Self {
            swapchain,
            loader,
            image_views,
            depth_image,
            framebuffers,
        }
    }

    fn create_image_views(ctx: &Context, images: &[vk::Image]) -> Vec<vk::ImageView> {
        images
            .iter()
            .map(|&image| {
                let subresource_range = vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1);
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(ctx.surface.config.surface_format.format)
                    .subresource_range(*subresource_range);
                unsafe {
                    ctx.create_image_view(&create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect()
    }

    fn create_framebuffers(
        ctx: &Context,
        pass: vk::RenderPass,
        image_views: &[vk::ImageView],
        depth_image: &DepthImage,
    ) -> Vec<vk::Framebuffer> {
        image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view, depth_image.view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(pass)
                    .attachments(&attachments)
                    .width(ctx.surface.config.extent.width)
                    .height(ctx.surface.config.extent.height)
                    .layers(1);
                unsafe {
                    ctx.create_framebuffer(&create_info, None)
                        .expect("Failed to create framebuffer")
                }
            })
            .collect()
    }

    pub fn acquire_next_image_and_signal(&self, signal_to: vk::Semaphore) -> (u32, bool) {
        unsafe {
            self.loader
                .acquire_next_image(self.swapchain, u64::MAX, signal_to, vk::Fence::null())
                .unwrap_or((0, true))
        }
    }

    // Returns true if the swapchain needs recreating
    pub fn present_to_when(
        &self,
        ctx: &Context,
        image_index: usize,
        wait_on: &[vk::Semaphore],
    ) -> bool {
        let swapchains = [self.swapchain];
        let image_indices = [image_index as u32];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(wait_on)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.loader
                .queue_present(**ctx.queues.graphics(), &present_info)
                .unwrap_or(true)
        }
    }
}

impl Destroy<Context> for Swapchain {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        for &framebuffer in &self.framebuffers {
            ctx.destroy_framebuffer(framebuffer, None);
        }
        self.depth_image.destroy_with(ctx);
        for &image_view in &self.image_views {
            ctx.destroy_image_view(image_view, None);
        }
        self.loader.destroy_swapchain(self.swapchain, None);
    }
}
