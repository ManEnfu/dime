//! Core types and traits for `dime` library.
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::must_use_candidate)]

#[macro_use]
pub(crate) mod macros;

pub mod erased;
pub mod error;
pub mod injector;
pub mod runtime;

pub use erased::Erased;
pub use error::{Error, Result};
pub use injector::Injector;
pub use runtime::Runtime;
