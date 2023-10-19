use std::{ffi::CString, ops::RangeInclusive};

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub fn cstring(s: &str) -> CString {
    CString::new(s).expect("Input string should not contain NUL byte")
}

pub const fn solo_range(i: usize) -> RangeInclusive<usize> {
    i..=i
}

pub fn total_size<T>(slices: &[&[T]]) -> usize {
    slices
        .iter()
        .map(|&slice| std::mem::size_of_val(slice))
        .sum()
}

pub const fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

pub fn load_image_from_file(filename: &str) -> image::RgbaImage {
    image::io::Reader::open(filename)
        .expect("Failed to open image file")
        .decode()
        .expect("Failed to read image from file")
        .into_rgba8()
}
