//! Asynchronous dependency injection library.
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::must_use_candidate)]

#[macro_use]
pub(crate) mod macros;

pub mod erased;
pub mod store;

pub mod result;

pub mod component;
