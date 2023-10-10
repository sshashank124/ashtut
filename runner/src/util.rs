use std::ops::RangeInclusive;

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
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

pub fn load_image_from_file(filename: &str) -> image::RgbaImage {
    image::io::Reader::open(filename)
        .expect("Failed to open image file")
        .decode()
        .expect("Failed to read image from file")
        .into_rgba8()
}
