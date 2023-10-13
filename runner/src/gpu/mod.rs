pub mod buffer;
pub mod commands;
pub mod context;
pub mod descriptors;
pub mod framebuffer;
pub mod image;
pub mod pipeline;
pub mod render_pass;
pub mod sampled_image;
pub mod sampler;
pub mod scope;
pub mod uniforms;
pub mod vertex;

pub use gpu_allocator::vulkan as alloc;

pub trait Destroy<Input> {
    unsafe fn destroy_with(&mut self, input: &mut Input);
}

impl<T: Destroy<Input>, Input> Destroy<Input> for Vec<T> {
    unsafe fn destroy_with(&mut self, input: &mut Input) {
        self.iter_mut().for_each(|e| e.destroy_with(input));
    }
}

pub trait Descriptions {
    type BindingType;
    const NUM_BINDINGS: usize;
    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS];

    type AttributeType;
    const NUM_ATTRIBUTES: usize;
    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES];
}
