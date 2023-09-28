use std::ffi::c_char;

use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Win32Surface},
};

pub const REQUIRED_EXTENSION_NAMES: &[*const c_char] = &[
    Surface::name().as_ptr(),
    DebugUtils::name().as_ptr(),
    Win32Surface::name().as_ptr(),
];