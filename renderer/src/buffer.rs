use std::{ops::Deref, slice};

use ash::vk;
use vk_mem::Alloc;

use crate::{context::Context, memory, scope::Scope, Destroy};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
}

impl Buffer {
    pub fn create(
        ctx: &Context,
        name: String,
        create_info: vk::BufferCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
    ) -> Self {
        let (buffer, allocation) = unsafe {
            ctx.allocator
                .create_buffer(&create_info, alloc_info)
                .expect("Failed to create buffer with allocated memory")
        };
        ctx.set_debug_name(buffer, &(name + " - Buffer"));

        Self { buffer, allocation }
    }

    pub fn create_with_data(
        ctx: &Context,
        name: String,
        mut info: vk::BufferCreateInfo,
        data: &[u8],
    ) -> Self {
        info = info.size(std::mem::size_of_val(data) as _);
        let buffer = Self::create(ctx, name, info, &memory::purpose::staging());
        buffer.fill_from(ctx, data);
        buffer
    }

    pub fn create_with_staged_data(
        ctx: &Context,
        scope: &mut Scope,
        name: String,
        mut info: vk::BufferCreateInfo,
        data: &[u8],
        memory_priority: memory::Priority,
    ) -> Self {
        let staging = Self::create_with_data(
            ctx,
            name.clone() + " - Staging",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                ..info
            },
            data,
        );

        info.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        info.size = std::mem::size_of_val(data) as _;
        let buffer = Self::create(
            ctx,
            name,
            info,
            &memory::purpose::device_local(memory_priority),
        );
        buffer.cmd_copy_from(ctx, scope.commands.buffer, &staging, info.size);

        scope.add_resource(staging);

        buffer
    }

    pub fn fill_with<T: bytemuck::Pod>(&self, ctx: &Context, data: &T) {
        self.fill_from(ctx, bytemuck::bytes_of(data));
    }

    pub fn fill_from(&self, ctx: &Context, data: &[u8]) {
        let mapped_ptr = ctx
            .allocator
            .get_allocation_info(&self.allocation)
            .mapped_data;

        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr.cast(), data.len());
        }
    }

    pub fn cmd_copy_from(
        &self,
        ctx: &Context,
        command_buffer: vk::CommandBuffer,
        src: &Self,
        size: u64,
    ) {
        let copy_info = vk::BufferCopy::default().size(size);

        unsafe {
            ctx.cmd_copy_buffer(command_buffer, **src, **self, slice::from_ref(&copy_info));
        }
    }

    pub fn get_device_address(&self, ctx: &Context) -> vk::DeviceAddress {
        unsafe {
            ctx.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(self.buffer),
            )
        }
    }
}

impl Destroy<Context> for Buffer {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        ctx.allocator
            .destroy_buffer(self.buffer, &mut self.allocation);
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
