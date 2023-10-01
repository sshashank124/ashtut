use std::ops::Range;

pub mod info;
pub mod platform;

pub trait Destroy<Input> {
    fn destroy_with(&self, input: Input);
}

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub fn solo_range(i: usize) -> Range<usize> {
    i..i + 1
}
