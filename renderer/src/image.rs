use std::{marker::ConstParamTy, ops::Deref, slice};

use ash::vk;
use vk_mem::Alloc;

use crate::{buffer::Buffer, context::Context, memory, scope::Scope, Destroy};

#[derive(PartialEq, Eq, ConstParamTy)]
pub enum Format {
    Hdr,
    Color,
    Depth,
    Swapchain,
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::Hdr => Self::R32G32B32A32_SFLOAT,
            Format::Color => Self::R8G8B8A8_SRGB,
            Format::Depth => Self::D16_UNORM,
            Format::Swapchain => Self::B8G8R8A8_SRGB,
        }
    }
}

pub struct Image<const FORMAT: Format> {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub extent: vk::Extent2D,
    allocation: Option<vk_mem::Allocation>, // None if memory is not managed by us (eg. swapchain)
}

pub struct BarrierInfo {
    pub layout: vk::ImageLayout,
    pub stage: vk::PipelineStageFlags,
    pub access: vk::AccessFlags,
}

impl<const FORMAT: Format> Image<FORMAT> {
    pub fn new_of_format(
        ctx: &Context,
        name: String,
        image: vk::Image,
        extent: vk::Extent2D,
        format: vk::Format,
        allocation: Option<vk_mem::Allocation>,
    ) -> Self {
        firestorm::profile_method!(new_of_format);

        let view = {
            let info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(Self::subresource_range());

            unsafe {
                ctx.create_image_view(&info, None)
                    .expect("Failed to create image view")
            }
        };
        ctx.set_debug_name(view, &(name + " - Image View"));

        Self {
            image,
            view,
            extent,
            allocation,
        }
    }

    pub fn new(
        ctx: &Context,
        name: String,
        image: vk::Image,
        extent: vk::Extent2D,
        allocation: Option<vk_mem::Allocation>,
    ) -> Self {
        Self::new_of_format(ctx, name, image, extent, FORMAT.into(), allocation)
    }

    pub fn create(
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        name: String,
        info: &vk::ImageCreateInfo,
        memory_purpose: &vk_mem::AllocationCreateInfo,
        to: Option<&BarrierInfo>,
    ) -> Self {
        firestorm::profile_method!(create);

        let name = name + " - Image";

        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::UNDEFINED,
            tiling: vk::ImageTiling::OPTIMAL,
            format: FORMAT.into(),
            usage: Self::usage_flags() | info.usage,
            ..*info
        };

        let (image, allocation) = unsafe {
            ctx.allocator
                .create_image(&image_info, memory_purpose)
                .expect("Failed to create image with allocated memory")
        };
        ctx.set_debug_name(image, &name);

        let image = Self::new(
            ctx,
            name,
            image,
            vk::Extent2D {
                width: image_info.extent.width,
                height: image_info.extent.height,
            },
            Some(allocation),
        );

        if let Some(to) = to {
            image.transition_layout(ctx, command_buffer, &BarrierInfo::INIT, to);
        }

        image
    }

    const fn subresource_range() -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: Self::aspect_flags(),
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        }
    }

    pub fn transition_layout(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        from: &BarrierInfo,
        to: &BarrierInfo,
    ) {
        firestorm::profile_method!(transition_layout);

        let barrier = vk::ImageMemoryBarrier::default()
            .image(self.image)
            .old_layout(from.layout)
            .new_layout(to.layout)
            .src_access_mask(from.access)
            .dst_access_mask(to.access)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(Self::subresource_range());

        unsafe {
            ctx.cmd_pipeline_barrier(
                command_buffer,
                from.stage,
                to.stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                slice::from_ref(&barrier),
            );
        }
    }

    const fn usage_flags() -> vk::ImageUsageFlags {
        match FORMAT {
            Format::Depth => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            _ => vk::ImageUsageFlags::SAMPLED,
        }
    }

    const fn aspect_flags() -> vk::ImageAspectFlags {
        match FORMAT {
            Format::Depth => vk::ImageAspectFlags::DEPTH,
            _ => vk::ImageAspectFlags::COLOR,
        }
    }
}

impl Image<{ Format::Color }> {
    pub fn create_from_image(
        ctx: &Context,
        scope: &mut Scope,
        name: String,
        img: &image::RgbaImage,
    ) -> Self {
        firestorm::profile_method!(create_from_image);

        let staging = {
            let info = vk::BufferCreateInfo::default().usage(vk::BufferUsageFlags::TRANSFER_SRC);
            Buffer::create_with_data(ctx, name.clone() + " - Staging", info, img)
        };

        let extent = vk::Extent3D {
            width: img.width(),
            height: img.height(),
            depth: 1,
        };

        let info = vk::ImageCreateInfo::default()
            .extent(extent)
            .usage(vk::ImageUsageFlags::TRANSFER_DST);
        let image = Self::create(
            ctx,
            scope.commands.buffer,
            name,
            &info,
            &memory::purpose::device_local(memory::Priority::Medium),
            Some(&BarrierInfo::TRANSFER_DST),
        );

        // Copy data to image
        image.cmd_copy_from(ctx, scope.commands.buffer, &staging, extent);

        image.transition_layout(
            ctx,
            scope.commands.buffer,
            &BarrierInfo::TRANSFER_DST,
            &BarrierInfo::SHADER_READ,
        );

        scope.add_resource(staging);

        image
    }

    fn cmd_copy_from(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        src: &Buffer,
        extent: vk::Extent3D,
    ) {
        firestorm::profile_method!(cmd_copy_from);

        let copy_info = vk::BufferImageCopy::default()
            .image_extent(extent)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe {
            ctx.cmd_copy_buffer_to_image(
                command_buffer,
                **src,
                **self,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&copy_info),
            );
        }
    }
}

impl Image<{ Format::Depth }> {
    pub const CLEAR_VALUE: vk::ClearValue = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.,
            stencil: 0,
        },
    };
}

impl BarrierInfo {
    pub const INIT: Self = Self {
        layout: vk::ImageLayout::UNDEFINED,
        stage: vk::PipelineStageFlags::TOP_OF_PIPE,
        access: vk::AccessFlags::empty(),
    };
    pub const GENERAL: Self = Self {
        layout: vk::ImageLayout::GENERAL,
        stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        access: vk::AccessFlags::empty(),
    };
    pub const COLOR_ATTACHMENT: Self = Self {
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        stage: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        access: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
    };
    pub const TRANSFER_DST: Self = Self {
        layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        stage: vk::PipelineStageFlags::TRANSFER,
        access: vk::AccessFlags::TRANSFER_WRITE,
    };
    pub const SHADER_READ: Self = Self {
        layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
        access: vk::AccessFlags::SHADER_READ,
    };
    pub const DEPTH: Self = Self {
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        stage: vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
        access: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
    };
    pub const PRESENTATION: Self = Self {
        layout: vk::ImageLayout::PRESENT_SRC_KHR,
        stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        access: vk::AccessFlags::empty(),
    };
}

impl<const FORMAT: Format> Destroy<Context> for Image<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        ctx.destroy_image_view(self.view, None);
        if let Some(mut allocation) = self.allocation.take() {
            ctx.allocator.destroy_image(self.image, &mut allocation);
        }
    }
}

impl<const FORMAT: Format> Deref for Image<FORMAT> {
    type Target = vk::Image;
    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
