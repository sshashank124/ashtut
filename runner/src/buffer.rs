use std::ops::{Deref, DerefMut};

use ash::vk;

use crate::{
    device::{gpu_alloc, Device},
    util::Destroy,
};

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<gpu_alloc::Allocation>,
}

impl Buffer {
    pub fn create_with<T>(
        device: &mut Device,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
        data: &[T],
    ) -> Self {
        let mut buffer = Self::create(device, name, create_info, location);
        buffer.fill(data);
        buffer
    }

    pub fn create(
        device: &mut Device,
        name: &str,
        create_info: &vk::BufferCreateInfo,
        location: gpu_allocator::MemoryLocation,
    ) -> Self {
        let buffer = unsafe {
            device
                .create_buffer(create_info, None)
                .expect("Failed to create buffer")
        };

        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let allocation_create_info = gpu_alloc::AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: gpu_alloc::AllocationScheme::GpuAllocatorManaged,
        };

        let allocation = device
            .allocator
            .allocate(&allocation_create_info)
            .expect("Failed to allocate memory");

        unsafe {
            device
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
                    .as_ptr() as *mut T;
                std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr, data.len());
            }
        }
    }
}

impl<'a> Destroy<&'a mut Device> for Buffer {
    unsafe fn destroy_with(&mut self, device: &'a mut Device) {
        if let Some(allocation) = self.allocation.take() {
            device
                .allocator
                .free(allocation)
                .expect("Failed to free allocated memory");
        }
        device.destroy_buffer(self.buffer, None);
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
