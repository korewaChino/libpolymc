#![deny(unsafe_op_in_unsafe_fn)]
pub mod auth;
pub mod error;
pub mod instance;
pub mod java_wrapper;

pub use error::{Error, Result};
