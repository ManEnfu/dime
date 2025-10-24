//! Asynchronous dependency injection library.
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::must_use_candidate)]

pub use boxed_clone::BoxedClone;

#[macro_use]
pub(crate) mod macros;

mod boxed_clone;
pub mod erased;
pub mod store;

pub mod result;

pub mod component;
pub mod constructor;
