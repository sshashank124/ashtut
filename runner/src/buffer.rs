use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::{
    context::{gpu_alloc, Context},
    util::Destroy,
};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<gpu_alloc::Allocation>,
}

impl Buffer {
    pub fn create_with<T>(
        ctx: &mut Context,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
        data: &[T],
    ) -> Self {
        let mut buffer = Self::create(ctx, name, create_info, location);
        buffer.fill(data);
        buffer
    }

    pub fn create(
        ctx: &mut Context,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer = unsafe {
            ctx.device
                .create_buffer(create_info, None)
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

    pub fn fill<T>(&mut self, data: &[T]) {
        if let Some(allocation) = &mut self.allocation {
            unsafe {
                let mapped_ptr = allocation
                    .mapped_ptr()
                    .expect("Failed to get mapped pointer")
                    .as_ptr()
                    .cast::<T>();
                std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr, data.len());
            }
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
