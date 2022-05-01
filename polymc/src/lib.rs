#![deny(unsafe_op_in_unsafe_fn)]
pub mod auth;
pub mod config;
pub mod error;
pub mod instance;
pub mod java_wrapper;
pub mod launcher;
pub mod meta;
pub mod util;

pub use error::{Error, Result};
use std::os::raw::c_char;

/// Helper for C code to free a CString
#[cfg(feature = "ctypes")]
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn free_str(s: *mut c_char) {
    let _ = unsafe { std::ffi::CString::from_raw(s) };
}
