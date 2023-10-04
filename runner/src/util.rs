use std::ops::RangeInclusive;

pub trait Descriptors {
    type BindingType;
    const NUM_BINDINGS: usize;
    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS];

    type AttributeType;
    const NUM_ATTRIBUTES: usize;
    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES];
}

pub trait Destroy<Input> {
    unsafe fn destroy_with(&mut self, input: Input);
}

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub const fn solo_range(i: usize) -> RangeInclusive<usize> {
    i..=i
}
