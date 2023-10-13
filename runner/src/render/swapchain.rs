use ash::{extensions::khr, vk};

use crate::gpu::{
    context::Context, framebuffer::Framebuffers, image::Image, render_pass::RenderPass, Destroy,
};

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub loader: khr::Swapchain,
    pub render_target: Framebuffers<{ vk::Format::UNDEFINED }>,
}

impl Swapchain {
    pub fn create(ctx: &mut Context, render_pass: &RenderPass) -> Self {
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
        }
        .into_iter()
        .map(|image| Image::new(ctx, image, ctx.surface.config.surface_format.format, None))
        .collect::<Vec<_>>();

        let render_target = Framebuffers::create_for_images(
            ctx,
            "Swapchain",
            render_pass,
            ctx.surface.config.extent,
            images,
        );

        Self {
            swapchain,
            loader,
            render_target,
        }
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
        self.render_target.destroy_with(ctx);
        self.loader.destroy_swapchain(self.swapchain, None);
    }
}
