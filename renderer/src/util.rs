use std::ffi;

#[macro_export]
macro_rules! cstr {
    ($str:expr) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($str, "\0").as_bytes()) }
    };
}

pub const SHADER_ENTRY_POINT: &ffi::CStr = cstr!("main");

pub fn bytes_to_string(string: *const ffi::c_char) -> String {
    unsafe { ffi::CStr::from_ptr(string) }
        .to_str()
        .expect("Failed to parse raw string")
        .to_owned()
}

pub const fn align_to(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}
