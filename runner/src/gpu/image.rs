use std::ops::Deref;

use ash::vk;

use super::{alloc, buffer::Buffer, context::Context, scope::Scope, Destroy};

#[allow(clippy::module_name_repetitions)]
pub type HdrImage = Image<HdrColor>;
#[allow(clippy::module_name_repetitions)]
pub type ColorImage = Image<Color>;
#[allow(clippy::module_name_repetitions)]
pub type DepthImage = Image<Depth>;

pub trait Props {
    const FORMAT: vk::Format;
    const ASPECT_FLAGS: vk::ImageAspectFlags;
    const FINAL_LAYOUT: vk::ImageLayout;
    fn usage() -> vk::ImageUsageFlags;
}

pub struct HdrColor;
impl Props for HdrColor {
    const FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;
    const ASPECT_FLAGS: vk::ImageAspectFlags = vk::ImageAspectFlags::COLOR;
    const FINAL_LAYOUT: vk::ImageLayout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
    fn usage() -> vk::ImageUsageFlags {
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
    }
}

pub struct Color;
impl Props for Color {
    const FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
    const ASPECT_FLAGS: vk::ImageAspectFlags = vk::ImageAspectFlags::COLOR;
    const FINAL_LAYOUT: vk::ImageLayout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    fn usage() -> vk::ImageUsageFlags {
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST
    }
}

pub struct Depth;
impl Props for Depth {
    const FORMAT: vk::Format = vk::Format::D16_UNORM;
    const ASPECT_FLAGS: vk::ImageAspectFlags = vk::ImageAspectFlags::DEPTH;
    const FINAL_LAYOUT: vk::ImageLayout = vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
    fn usage() -> vk::ImageUsageFlags {
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
    }
}

pub struct Image<T> {
    image: vk::Image,
    allocation: Option<alloc::Allocation>,
    pub view: vk::ImageView,
    _p: std::marker::PhantomData<T>,
}

impl<T: Props> Image<T> {
    pub fn create(ctx: &mut Context, name: &str, info: &vk::ImageCreateInfo) -> Self {
        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            initial_layout: vk::ImageLayout::UNDEFINED,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            format: T::FORMAT,
            usage: T::usage(),
            ..*info
        };

        let image = unsafe {
            ctx.create_image(&image_info, None)
                .expect("Failed to create image")
        };

        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(T::FORMAT)
            .subresource_range(Self::subresource_range());

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

        let view = unsafe {
            ctx.create_image_view(&view_info, None)
                .expect("Failed to create image view")
        };

        Self {
            image,
            allocation: Some(allocation),
            view,
            _p: std::marker::PhantomData,
        }
    }

    const fn subresource_range() -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: T::ASPECT_FLAGS,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }

    fn transition_layout(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        layout_transition: [vk::ImageLayout; 2],
        stage_transition: [vk::PipelineStageFlags; 2],
        access_transition: [vk::AccessFlags; 2],
    ) {
        let barrier = vk::ImageMemoryBarrier::builder()
            .image(self.image)
            .old_layout(layout_transition[0])
            .new_layout(layout_transition[1])
            .src_access_mask(access_transition[0])
            .dst_access_mask(access_transition[1])
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(Self::subresource_range())
            .build();

        let image_barriers = [barrier];

        unsafe {
            ctx.cmd_pipeline_barrier(
                command_buffer,
                stage_transition[0],
                stage_transition[1],
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &image_barriers,
            );
        }
    }
}

impl HdrImage {
    pub fn transition_layout_ready_to_write(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
    ) {
        self.transition_layout(
            ctx,
            command_buffer,
            [
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ],
            [
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ],
            [
                vk::AccessFlags::empty(),
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ],
        );
    }

    pub fn transition_layout_ready_to_read(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
    ) {
        self.transition_layout(
            ctx,
            command_buffer,
            [
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ],
            [
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ],
            [vk::AccessFlags::empty(), vk::AccessFlags::SHADER_READ],
        );
    }
}

impl ColorImage {
    pub fn create_from_image(
        ctx: &mut Context,
        scope: &mut Scope,
        name: &str,
        img: &image::RgbaImage,
    ) -> Self {
        let data_sources = [img.as_raw().as_slice()];
        let staging = {
            let info = vk::BufferCreateInfo::builder().usage(vk::BufferUsageFlags::TRANSFER_SRC);
            Buffer::create_with_data(ctx, &format!("{name} [STAGING]"), *info, &data_sources)
        };

        let extent = vk::Extent3D {
            width: img.width(),
            height: img.height(),
            depth: 1,
        };

        let info = vk::ImageCreateInfo::builder().extent(extent);
        let mut image = Self::create(ctx, name, &info);

        image.transition_layout_for_transfer(ctx, scope.commands.buffer);
        image.record_copy_from(ctx, scope.commands.buffer, &staging, extent);
        image.transition_layout_ready_to_read(ctx, scope.commands.buffer);

        scope.add_resource(staging);

        image
    }

    pub fn record_copy_from(
        &mut self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        src: &Buffer,
        extent: vk::Extent3D,
    ) {
        let copy_info = [vk::BufferImageCopy::builder()
            .image_extent(extent)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build()];

        unsafe {
            ctx.cmd_copy_buffer_to_image(
                command_buffer,
                **src,
                **self,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &copy_info,
            );
        }
    }

    fn transition_layout_for_transfer(&self, ctx: &Context, command_buffer: vk::CommandBuffer) {
        self.transition_layout(
            ctx,
            command_buffer,
            [
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ],
            [
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ],
            [vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE],
        );
    }

    fn transition_layout_ready_to_read(&self, ctx: &Context, command_buffer: vk::CommandBuffer) {
        self.transition_layout(
            ctx,
            command_buffer,
            [vk::ImageLayout::TRANSFER_DST_OPTIMAL, Color::FINAL_LAYOUT],
            [
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ],
            [
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
            ],
        );
    }
}

impl DepthImage {
    pub fn init(ctx: &mut Context, scope: &mut Scope, name: &str) -> Self {
        let info = vk::ImageCreateInfo::builder().extent(ctx.surface.config.extent.into());
        let depth_image = Self::create(ctx, name, &info);
        depth_image.transition_layout_ready_for_use(ctx, scope.commands.buffer);
        depth_image
    }

    fn transition_layout_ready_for_use(&self, ctx: &Context, command_buffer: vk::CommandBuffer) {
        self.transition_layout(
            ctx,
            command_buffer,
            [vk::ImageLayout::UNDEFINED, Depth::FINAL_LAYOUT],
            [
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ],
            [
                vk::AccessFlags::empty(),
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ],
        );
    }
}

impl<T> Destroy<Context> for Image<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_image_view(self.view, None);
        ctx.destroy_image(self.image, None);
        if let Some(allocation) = self.allocation.take() {
            ctx.allocator
                .free(allocation)
                .expect("Failed to free allocated memory");
        }
    }
}

impl<T> Deref for Image<T> {
    type Target = vk::Image;
    fn deref(&self) -> &Self::Target {
        &self.image
    }
}
