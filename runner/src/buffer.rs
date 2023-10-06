use std::ops::{Deref, DerefMut};

use ash::vk;

use shared::bytemuck;

use crate::{
    context::{gpu_alloc, Context},
    util::{self, Destroy},
};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<gpu_alloc::Allocation>,
}

impl Buffer {
    pub fn create(
        ctx: &mut Context,
        name: &str,
        create_info: vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer = unsafe {
            ctx.device
                .create_buffer(&create_info, None)
                .expect("Failed to create buffer")
        };

        let requirements = unsafe { ctx.device.get_buffer_memory_requirements(buffer) };
        let allocation_create_info = gpu_alloc::AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: gpu_alloc::AllocationScheme::GpuAllocatorManaged,
        };

        let allocation = ctx
            .device
            .allocator
            .allocate(&allocation_create_info)
            .expect("Failed to allocate memory");

        unsafe {
            ctx.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("Failed to bind memory");
        }

        Self {
            buffer,
            allocation: Some(allocation),
        }
    }

    pub fn create_with_data(
        ctx: &mut Context,
        name: &str,
        mut info: vk::BufferCreateInfo,
        data_sources: &[&[u8]],
    ) -> Self {
        info.size = util::total_size(data_sources) as u64;
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::CpuToGpu);
        buffer.fill_from_sources(data_sources);
        buffer
    }

    pub fn create_with_staged_data(
        ctx: &mut Context,
        name: &str,
        mut info: vk::BufferCreateInfo,
        data_sources: &[&[u8]],
    ) -> Self {
        let mut staging = Self::create_with_data(
            ctx,
            &format!("{name} [STAGING]"),
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                ..info
            },
            data_sources,
        );

        info.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        info.size = util::total_size(data_sources) as u64;
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::GpuOnly);
        buffer.copy_from(ctx, &staging, info.size);

        unsafe {
            staging.destroy_with(ctx);
        }

        buffer
    }

    pub fn fill_with<T: bytemuck::Pod>(&mut self, data: &T) {
        self.fill_from_source(bytemuck::bytes_of(data));
    }

    pub fn fill_from_source(&mut self, source: &[u8]) {
        let sources = [source];
        self.fill_from_sources(&sources);
    }

    pub fn fill_from_sources(&mut self, data_sources: &[&[u8]]) {
        if let Some(allocation) = &mut self.allocation {
            let mut mapped_slice = allocation
                .mapped_slice_mut()
                .expect("Failed to get mapped slice");

            for &data_source in data_sources {
                let source_size = data_source.len();
                mapped_slice[..source_size].copy_from_slice(data_source);
                mapped_slice = &mut mapped_slice[source_size..];
            }
        }
    }

    pub fn copy_from(&mut self, ctx: &Context, src: &Self, size: u64) {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(ctx.device.queues.transient_transfer_pool())
            .command_buffer_count(1);

        let command_buffers = unsafe {
            ctx.device
                .allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffer")
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        let copy_info = [vk::BufferCopy::builder().size(size).build()];

        let submit_info = [vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build()];

        unsafe {
            ctx.device
                .begin_command_buffer(command_buffers[0], &begin_info)
                .expect("Failed to begin recording command buffer");

            ctx.device
                .cmd_copy_buffer(command_buffers[0], **src, **self, &copy_info);

            ctx.device
                .end_command_buffer(command_buffers[0])
                .expect("Failed to end recording command buffer");

            ctx.device
                .queue_submit(*ctx.device.queues.transfer, &submit_info, vk::Fence::null())
                .expect("Failed to submit command to `transfer` queue");

            ctx.device
                .queue_wait_idle(*ctx.device.queues.transfer)
                .expect("Failed to wait for transfer queue to idle");

            ctx.device.free_command_buffers(
                ctx.device.queues.transient_transfer_pool(),
                &command_buffers,
            );
        }
    }
}

impl<'a> Destroy<&'a mut Context> for Buffer {
    unsafe fn destroy_with(&mut self, ctx: &'a mut Context) {
        if let Some(allocation) = self.allocation.take() {
            ctx.device
                .allocator
                .free(allocation)
                .expect("Failed to free allocated memory");
        }
        ctx.device.destroy_buffer(self.buffer, None);
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
