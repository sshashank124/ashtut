use std::{ops::Deref, slice};

use ash::vk;

use super::{alloc, context::Context, scope::OneshotScope, Destroy};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<alloc::Allocation>,
}

impl Buffer {
    pub fn create(
        ctx: &mut Context,
        name: impl AsRef<str>,
        create_info: vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let name = String::from(name.as_ref()) + " - Buffer";
        let buffer = unsafe {
            ctx.create_buffer(&create_info, None)
                .expect("Failed to create buffer")
        };
        ctx.set_debug_name(buffer, &name);

        let requirements = unsafe { ctx.get_buffer_memory_requirements(buffer) };
        let allocation_name = name + " - Allocation";
        let allocation_create_info = alloc::AllocationCreateDesc {
            name: &allocation_name,
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
        name: impl AsRef<str>,
        mut info: vk::BufferCreateInfo,
        data: &[u8],
    ) -> Self {
        info.size = std::mem::size_of_val(data) as _;
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::CpuToGpu);
        buffer.fill_from(data);
        buffer
    }

    pub fn create_with_staged_data(
        ctx: &mut Context,
        scope: &mut OneshotScope,
        name: impl AsRef<str>,
        mut info: vk::BufferCreateInfo,
        data: &[u8],
    ) -> Self {
        let staging = Self::create_with_data(
            ctx,
            String::from(name.as_ref()) + " - Staging",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                ..info
            },
            data,
        );

        info.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        info.size = std::mem::size_of_val(data) as _;
        let mut buffer = Self::create(ctx, name, info, gpu_allocator::MemoryLocation::GpuOnly);
        buffer.record_copy_from(ctx, scope.commands.buffer, &staging, info.size);

        scope.add_resource(staging);

        buffer
    }

    pub fn fill_with<T: bytemuck::Pod>(&mut self, data: &T) {
        self.fill_from(bytemuck::bytes_of(data));
    }

    pub fn fill_from(&mut self, data: &[u8]) {
        if let Some(allocation) = &mut self.allocation {
            allocation
                .mapped_slice_mut()
                .expect("Failed to get mapped slice")[..data.len()]
                .copy_from_slice(data);
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
