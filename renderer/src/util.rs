pub const SHADER_ENTRY_POINT: &std::ffi::CStr =
    unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

pub fn bytes_to_string(string: *const std::ffi::c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub const fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
