use std::{ops::Deref, slice};

use ash::vk;

use shared::bytemuck;

use super::{alloc, context::Context, scope::OneshotScope, Destroy};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<alloc::Allocation>,
}

impl Buffer {
    pub fn create(
        ctx: &mut Context,
        name: &str,
        create_info: vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer = unsafe {
            ctx.create_buffer(&create_info, None)
                .expect("Failed to create buffer")
        };

        let requirements = unsafe { ctx.get_buffer_memory_requirements(buffer) };
        let allocation_create_info = alloc::AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: alloc::AllocationScheme::GpuAllocatorManaged,
        };

        let allocation = ctx
            .device
            .allocator
            .allocate(&allocation_create_info)
            .expect("Failed to allocate memory");

        unsafe {
            ctx.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
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
        info.size = Self::total_size_of_sources(data_sources);
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::CpuToGpu);
        buffer.fill_from_sources(data_sources);
        buffer
    }

    pub fn create_with_staged_data(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        name: &str,
        mut info: vk::BufferCreateInfo,
        data_sources: &[&[u8]],
    ) -> Self {
        let staging = Self::create_with_data(
            ctx,
            &format!("{name} [STAGING]"),
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                ..info
            },
            data_sources,
        );

        info.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        info.size = Self::total_size_of_sources(data_sources);
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::GpuOnly);
        buffer.record_copy_from(ctx, scope.commands.buffer, &staging, info.size);

        scope.add_resource(staging);

        buffer
    }

    pub fn fill_with<T: bytemuck::Pod>(&mut self, data: &T) {
        self.fill_from_sources(slice::from_ref(&bytemuck::bytes_of(data)));
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

    pub fn record_copy_from(
        &mut self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        src: &Self,
        size: u64,
    ) {
        let copy_info = vk::BufferCopy::builder().size(size);

        unsafe {
            ctx.cmd_copy_buffer(command_buffer, **src, **self, slice::from_ref(&copy_info));
        }
    }

    pub fn get_device_address(&self, ctx: &Context) -> vk::DeviceAddress {
        unsafe {
            ctx.get_buffer_device_address(&vk::BufferDeviceAddressInfo {
                buffer: self.buffer,
                ..Default::default()
            })
        }
    }

    fn total_size_of_sources<T>(slices: &[&[T]]) -> u64 {
        slices
            .iter()
            .map(|&slice| std::mem::size_of_val(slice) as u64)
            .sum()
    }
}

impl Destroy<Context> for Buffer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        if let Some(allocation) = self.allocation.take() {
            ctx.allocator
                .free(allocation)
                .expect("Failed to free allocated memory");
        }
        ctx.destroy_buffer(self.buffer, None);
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
