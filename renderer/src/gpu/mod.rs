pub mod acceleration_structure;
pub mod buffer;
pub mod commands;
pub mod context;
pub mod descriptors;
pub mod framebuffers;
pub mod image;
pub mod pipeline;
pub mod query_pool;
pub mod sampler;
pub mod scene;
pub mod scope;
pub mod shader_binding_table;
pub mod swapchain;
pub mod sync_info;
pub mod texture;
pub mod uniforms;

use std::ops::DerefMut;

pub use gpu_allocator::vulkan as alloc;

pub trait Destroy<C> {
    unsafe fn destroy_with(&mut self, ctx: &mut C);
}

impl<T: Destroy<C>, C> Destroy<C> for Vec<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut C) {
        self.iter_mut().for_each(|e| e.destroy_with(ctx));
    }
}

impl<T: Destroy<C> + ?Sized, C> Destroy<C> for Box<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut C) {
        self.deref_mut().destroy_with(ctx);
    }
}
