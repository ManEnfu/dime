//! Asynchronous dependency injection library.
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::must_use_candidate)]

pub use dyn_clone::DynClone;

#[macro_use]
pub(crate) mod macros;

mod dyn_clone;
pub mod erased;
pub mod store;

pub mod result;

pub mod runtime;

pub mod injector;

pub mod component;
pub mod constructor;
