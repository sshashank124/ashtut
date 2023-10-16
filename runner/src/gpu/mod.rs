pub mod acceleration_structure;
pub mod buffer;
pub mod commands;
pub mod context;
pub mod descriptors;
pub mod framebuffers;
pub mod image;
pub mod model;
pub mod pipeline;
pub mod query_pool;
pub mod sampler;
pub mod scope;
pub mod swapchain;
pub mod sync_info;
pub mod texture;
pub mod uniforms;
pub mod vertex;

pub use gpu_allocator::vulkan as alloc;

pub trait Destroy<C> {
    unsafe fn destroy_with(&mut self, ctx: &mut C);
}

impl<T: Destroy<C>, C> Destroy<C> for Vec<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut C) {
        self.iter_mut().for_each(|e| e.destroy_with(ctx));
    }
}

pub trait Descriptions: Sized {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    type BindingType;
    const NUM_BINDINGS: usize;
    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS];

    type AttributeType;
    const NUM_ATTRIBUTES: usize;
    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES];
}
