use std::{
    any::{self, TypeId},
    error::Error,
    sync::Arc,
};

/// [`ResolutionError`] is an error that can be raised by functions and methods from this library.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResolutionError {
    NotDefined(TypeId, &'static str),
    Other(Arc<dyn Error + Send + Sync + 'static>),
}

impl ResolutionError {
    pub fn not_defined<T>() -> Self
    where
        T: 'static,
    {
        Self::NotDefined(TypeId::of::<T>(), any::type_name::<T>())
    }

    pub fn other<E>(err: E) -> Self
    where
        E: Into<Box<dyn Error + Send + Sync>>,
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

impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDefined(_, type_name) => {
                write!(f, "type `{type_name}` is not defined")
            }
            Self::Other(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ResolutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Other(error) => Some(error),
            _ => None,
        }
    }
}

/// [`Result`] is an alias to [`core::result::Result`] with [`ResolutionError`] as the
/// default error type.
pub type Result<T, E = ResolutionError> = core::result::Result<T, E>;
