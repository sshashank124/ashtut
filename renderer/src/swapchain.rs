use std::slice;

use ash::vk;

use crate::{context::Context, framebuffers::Framebuffers, image, scope::OneshotScope, Destroy};

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    images: Vec<image::Image<{ image::Format::Swapchain }>>,
    pub target: Framebuffers<{ image::Format::Swapchain }>,
}

impl Swapchain {
    pub fn create(ctx: &mut Context, scope: &OneshotScope, render_pass: vk::RenderPass) -> Self {
        let create_info = vk::SwapchainCreateInfoKHR::default()
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

        let swapchain = unsafe {
            ctx.ext
                .swapchain
                .create_swapchain(&create_info, None)
                .expect("Failed to create swapchain")
        };

        let images = unsafe {
            ctx.ext
                .swapchain
                .get_swapchain_images(swapchain)
                .expect("Failed to get swapchain images")
        }
        .into_iter()
        .enumerate()
        .map(|(idx, image)| {
            image::Image::new_of_format(
                ctx,
                format!("Swapchain - #{idx}"),
                image,
                ctx.surface.config.surface_format.format,
                None,
            )
        })
        .collect::<Vec<_>>();

        let target = Framebuffers::create(
            ctx,
            scope,
            "Swapchain",
            render_pass,
            ctx.surface.config.extent,
            &images,
        );

        Self {
            swapchain,
            images,
            target,
        }
    }

    pub fn get_next_image(&self, ctx: &Context, signal_to: vk::Semaphore) -> (u32, bool) {
        unsafe {
            ctx.ext
                .swapchain
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
        let image_index = image_index as _;
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_on)
            .swapchains(slice::from_ref(&self.swapchain))
            .image_indices(slice::from_ref(&image_index));

        unsafe {
            ctx.ext
                .swapchain
                .queue_present(**ctx.queues.graphics(), &present_info)
                .unwrap_or(true)
        }
    }
}

impl Destroy<Context> for Swapchain {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.target.destroy_with(ctx);
        self.images.destroy_with(ctx);
        ctx.ext.swapchain.destroy_swapchain(self.swapchain, None);
    }
}
