use std::{ops::Deref, slice};

use ash::vk;

use super::{alloc, buffer::Buffer, context::Context, scope::OneshotScope, Destroy};

pub mod format {
    pub const HDR: ash::vk::Format = ash::vk::Format::R32G32B32A32_SFLOAT;
    pub const COLOR: ash::vk::Format = ash::vk::Format::R8G8B8A8_SRGB;
    pub const DEPTH: ash::vk::Format = ash::vk::Format::D32_SFLOAT;
    pub const SWAPCHAIN: ash::vk::Format = ash::vk::Format::UNDEFINED;
}

pub struct Image<const FORMAT: vk::Format> {
    pub image: vk::Image,
    pub view: vk::ImageView,
    allocation: Option<alloc::Allocation>,
}

pub struct BarrierInfo {
    layout: vk::ImageLayout,
    stage: vk::PipelineStageFlags,
    access: vk::AccessFlags,
}

impl<const FORMAT: vk::Format> Image<FORMAT> {
    pub fn new(
        ctx: &Context,
        image: vk::Image,
        format: vk::Format,
        allocation: Option<alloc::Allocation>,
    ) -> Self {
        let view = {
            let info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(Self::subresource_range());

            unsafe {
                ctx.create_image_view(&info, None)
                    .expect("Failed to create image view")
            }
        };

        Self {
            image,
            view,
            allocation,
        }
    }

    pub fn create(
        ctx: &mut Context,
        scope: &OneshotScope,
        name: &str,
        info: &vk::ImageCreateInfo,
        to: Option<&BarrierInfo>,
    ) -> Self {
        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            initial_layout: vk::ImageLayout::UNDEFINED,
            tiling: vk::ImageTiling::OPTIMAL,
            format: FORMAT,
            usage: Self::usage_flags() | info.usage,
            ..*info
        };

        let image = unsafe {
            ctx.create_image(&image_info, None)
                .expect("Failed to create image")
        };

        let requirements = unsafe { ctx.get_image_memory_requirements(image) };
        let allocation_create_info = alloc::AllocationCreateDesc {
            name,
            requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: alloc::AllocationScheme::GpuAllocatorManaged,
        };

        let allocation = ctx
            .device
            .allocator
            .allocate(&allocation_create_info)
            .expect("Failed to allocate memory");

        unsafe {
            ctx.bind_image_memory(image, allocation.memory(), allocation.offset())
                .expect("Failed to bind memory");
        }

        let image = Self::new(ctx, image, FORMAT, Some(allocation));

        if let Some(to) = to {
            image.transition_layout(ctx, scope, &BarrierInfo::INIT, to);
        }

        image
    }

    const fn subresource_range() -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: Self::aspect_flags(),
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    fn transition_layout(
        &self,
        ctx: &Context,
        scope: &OneshotScope,
        from: &BarrierInfo,
        to: &BarrierInfo,
    ) {
        let barrier = vk::ImageMemoryBarrier::builder()
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
                scope.commands.buffer,
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
            format::DEPTH => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            _ => vk::ImageUsageFlags::SAMPLED,
        }
    }

    const fn aspect_flags() -> vk::ImageAspectFlags {
        match FORMAT {
            format::DEPTH => vk::ImageAspectFlags::DEPTH,
            _ => vk::ImageAspectFlags::COLOR,
        }
    }
}

impl Image<{ format::COLOR }> {
    pub fn create_from_image(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        name: &str,
        img: &image::RgbaImage,
    ) -> Self {
        let staging = {
            let info = vk::BufferCreateInfo::builder().usage(vk::BufferUsageFlags::TRANSFER_SRC);
            Buffer::create_with_data(ctx, &format!("{name} [STAGING]"), *info, img.as_raw())
        };

        let extent = vk::Extent3D {
            width: img.width(),
            height: img.height(),
            depth: 1,
        };

        let info = vk::ImageCreateInfo::builder()
            .extent(extent)
            .usage(vk::ImageUsageFlags::TRANSFER_DST);
        let image = Self::create(ctx, scope, name, &info, Some(&BarrierInfo::TRANSFER_DST));

        // Copy data to image
        image.record_copy_from(ctx, scope, &staging, extent);

        image.transition_layout(
            ctx,
            scope,
            &BarrierInfo::TRANSFER_DST,
            &BarrierInfo::SHADER_READ,
        );

        scope.add_resource(staging);

        image
    }

    pub fn record_copy_from(
        &self,
        ctx: &Context,
        scope: &OneshotScope,
        src: &Buffer,
        extent: vk::Extent3D,
    ) {
        let copy_info = vk::BufferImageCopy::builder()
            .image_extent(extent)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            });

        unsafe {
            ctx.cmd_copy_buffer_to_image(
                scope.commands.buffer,
                **src,
                **self,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&copy_info),
            );
        }
    }
}

impl BarrierInfo {
    const INIT: Self = Self {
        layout: vk::ImageLayout::UNDEFINED,
        stage: vk::PipelineStageFlags::TOP_OF_PIPE,
        access: vk::AccessFlags::empty(),
    };
    pub const GENERAL: Self = Self {
        layout: vk::ImageLayout::GENERAL,
        stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        access: vk::AccessFlags::empty(),
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
}

impl<const FORMAT: vk::Format> Destroy<Context> for Image<FORMAT> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_image_view(self.view, None);
        if let Some(allocation) = self.allocation.take() {
            ctx.destroy_image(self.image, None);
            ctx.allocator
                .free(allocation)
                .expect("Failed to free allocated memory");
        }
    }
}

impl<const FORMAT: vk::Format> Deref for Image<FORMAT> {
    type Target = vk::Image;
    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
