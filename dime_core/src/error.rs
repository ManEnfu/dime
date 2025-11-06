//! Error types.

use std::any::{TypeId, type_name};
use std::error::Error as StdError;
use std::sync::Arc;

/// [`Error`] is an error that can be raised by functions and methods from this library.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    NotDefined(TypeId, &'static str),
    Other(Arc<dyn StdError + Send + Sync + 'static>),
}

impl Error {
    pub fn not_defined<T>() -> Self
    where
        T: 'static,
    {
        Self::NotDefined(TypeId::of::<T>(), type_name::<T>())
    }

    pub fn other<E>(err: E) -> Self
    where
        E: Into<Box<dyn StdError + Send + Sync>>,
    {
        Self::Other(Arc::from(err.into()))
    }

    pub const fn is_not_defined(&self) -> bool {
        matches!(self, Self::NotDefined(_, _))
    }

    pub fn is_not_defined_for<T>(&self) -> bool
    where
        T: 'static,
    {
        matches!(self, Self::NotDefined(id, _) if *id == TypeId::of::<T>())
    }

    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other(_))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDefined(_, type_name) => {
                write!(f, "type `{type_name}` is not defined")
            }
            Self::Other(error) => error.fmt(f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Other(error) => Some(error),
            _ => None,
        }
    }
}

/// [`Result`] is an alias to [`core::result::Result`] with [`Error`] as the
/// default error type.
pub type Result<T, E = Error> = core::result::Result<T, E>;
