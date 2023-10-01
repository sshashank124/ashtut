use ash::{extensions::khr, vk};

use crate::{
    device::Device, instance::Instance, physical_device::PhysicalDevice, render_pass::RenderPass,
    surface::SurfaceConfiguration, util::Destroy,
};

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub loader: khr::Swapchain,
    pub config: SurfaceConfiguration,
    pub image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl Swapchain {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
        render_pass: &RenderPass,
        mut config: SurfaceConfiguration,
    ) -> Self {
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*physical_device.surface)
            .min_image_count(config.image_count)
            .image_format(config.surface_format.format)
            .image_color_space(config.surface_format.color_space)
            .image_extent(config.extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(config.present_mode)
            .clipped(true);

        let create_info = if let Some(different_indices) = physical_device
            .indices
            .separate_graphics_and_presentation_indices()
        {
            create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(different_indices)
        } else {
            create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        };

        let loader = khr::Swapchain::new(instance, device);

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

        config.image_count = images.len() as u32;

        let image_views = Self::create_image_views(device, config.surface_format.format, &images);
        let framebuffers =
            Self::create_framebuffers(device, render_pass, config.extent, &image_views);

        Self {
            swapchain,
            loader,
            config,
            image_views,
            framebuffers,
        }
    }

    fn create_image_views(
        device: &Device,
        format: vk::Format,
        images: &[vk::Image],
    ) -> Vec<vk::ImageView> {
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
                    .format(format)
                    .subresource_range(*subresource_range);
                unsafe {
                    device
                        .create_image_view(&create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect()
    }

    fn create_framebuffers(
        device: &Device,
        render_pass: &RenderPass,
        extent: vk::Extent2D,
        image_views: &[vk::ImageView],
    ) -> Vec<vk::Framebuffer> {
        image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(**render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
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

impl<'a> Destroy<&'a Device> for Swapchain {
    fn destroy_with(&self, device: &'a Device) {
        unsafe {
            for &framebuffer in &self.framebuffers {
                device.destroy_framebuffer(framebuffer, None);
            }
            for &image_view in &self.image_views {
                device.destroy_image_view(image_view, None);
            }
            self.loader.destroy_swapchain(self.swapchain, None);
        }
    }
}
