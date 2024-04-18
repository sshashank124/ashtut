#[macro_export]
macro_rules! cstr {
    ($str:expr) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($str, "\0").as_bytes()) }
    };
}
