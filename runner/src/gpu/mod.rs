pub mod buffer;
pub mod command_builder;
pub mod command_pool;
pub mod context;
pub mod image;
pub mod sampled_image;
pub mod sampler;
pub mod vertex;

pub trait Destroy<Input> {
    unsafe fn destroy_with(&mut self, input: &mut Input);
}

pub trait Descriptions {
    type BindingType;
    const NUM_BINDINGS: usize;
    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS];

    type AttributeType;
    const NUM_ATTRIBUTES: usize;
    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES];
}
