use std::{
    any::{self, TypeId},
    error::Error,
    sync::Arc,
};

/// [`ResolutionError`] is an error that can be raised by functions and methods from this library.
#[derive(Debug, Clone)]
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
}

/// [`Result`] is an alias to [`core::result::Result`] with [`ResolutionError`] as the
/// default error type.
pub type Result<T, E = ResolutionError> = core::result::Result<T, E>;
