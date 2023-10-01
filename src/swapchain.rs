use ash::{extensions::khr, vk};

use crate::{
    device::Device,
    instance::Instance,
    physical_device::PhysicalDevice,
    util::{info, Destroy},
};

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub loader: khr::Swapchain,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn create(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
    ) -> Self {
        let surface_details = &physical_device.surface_details;
        let surface_format = Self::choose_best_surface_format(&surface_details.formats);
        let format = surface_format.format;
        let extent = Self::choose_extent(&surface_details.capabilities);

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*physical_device.surface)
            .min_image_count(Self::choose_image_count(&surface_details.capabilities))
            .image_format(format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(surface_details.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(Self::choose_best_present_mode(
                &surface_details.present_modes,
            ))
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

        let image_views = Self::create_image_views(device, &images, format);

        Self {
            swapchain,
            loader,
            format,
            extent,
            image_views,
        }
    }

    fn choose_best_surface_format(
        available_formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        available_formats
            .iter()
            .copied()
            .find(|&format| format == info::PREFERRED_SURFACE_FORMAT)
            .unwrap_or_else(|| available_formats[0])
    }

    fn choose_best_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        available_present_modes
            .iter()
            .copied()
            .find(|&format| format == info::PREFERRED_PRESENT_MODE)
            .unwrap_or(info::FALLBACK_PRESENT_MODE)
    }

    fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

        vk::Extent2D {
            width: info::WINDOW_SIZE
                .0
                .max(capabilities.min_image_extent.width)
                .min(capabilities.max_image_extent.width),
            height: info::WINDOW_SIZE
                .1
                .max(capabilities.min_image_extent.height)
                .min(capabilities.max_image_extent.height),
        }
    }

    fn choose_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
        let image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 {
            image_count.min(capabilities.max_image_count)
        } else {
            image_count
        }
    }

    fn create_image_views(
        device: &Device,
        images: &[vk::Image],
        format: vk::Format,
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
}

impl<'a> Destroy<&'a Device> for Swapchain {
    fn destroy_with(&self, device: &'a Device) {
        unsafe {
            for &image_view in &self.image_views {
                device.destroy_image_view(image_view, None);
            }
            self.loader.destroy_swapchain(self.swapchain, None);
        }
    }
}
