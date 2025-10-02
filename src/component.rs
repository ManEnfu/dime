use std::{any::TypeId, sync::Arc};

use crate::result::{ResolutionError, Result};

pub trait Resolver: Send + Sync + 'static {}

/// [`Request`] is a trait for components or groups of components which values can be requested
/// from a resolver.
pub trait Request: Clone + Sized + Send + Sync + 'static {
    fn request<R>(r: &R) -> impl Future<Output = Result<Self>> + Send
    where
        R: Resolver;
}

impl<T> Request for Arc<T>
where
    T: Send + Sync + 'static,
{
    async fn request<R>(_: &R) -> Result<Self>
    where
        R: Resolver,
    {
        todo!()
    }
}

impl<T> Request for Option<T>
where
    T: Request,
{
    async fn request<R>(r: &R) -> Result<Self>
    where
        R: Resolver,
    {
        match T::request(r).await {
            Ok(v) => Ok(Some(v)),
            Err(ResolutionError::NotDefined(type_id, _)) if type_id == TypeId::of::<T>() => {
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }
}

impl<T> Request for Result<T>
where
    T: Request,
{
    async fn request<R>(r: &R) -> Result<Self>
    where
        R: Resolver,
    {
        Ok(T::request(r).await)
    }
}

/// [`Inject`] is a trait for components or group of components which can be injected
/// into a resolver.
pub trait Inject: Clone + Sized + Send + Sync + 'static {
    fn inject<R>(self, r: &R) -> impl Future<Output = Result<()>> + Send
    where
        R: Resolver;
}

impl<T> Inject for Arc<T>
where
    T: Send + Sync + 'static,
{
    async fn inject<R>(self, _: &R) -> Result<()>
    where
        R: Resolver,
    {
        todo!()
    }
}

impl<T> Inject for Option<T>
where
    T: Inject,
{
    async fn inject<R>(self, r: &R) -> Result<()>
    where
        R: Resolver,
    {
        if let Some(v) = self {
            v.inject(r).await
        } else {
            Err(ResolutionError::not_defined::<T>())
        }
    }
}

impl<T> Inject for Result<T>
where
    T: Inject,
{
    async fn inject<R>(self, r: &R) -> Result<()>
    where
        R: Resolver,
    {
        match self {
            Ok(v) => v.inject(r).await,
            Err(e) => Err(e),
        }
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod comp_tests {
    use super::*;

    fn is_request<T>(_: T)
    where
        T: Request,
    {
    }

    fn test_is_request() {
        is_request(Arc::new("hello".to_string()));
        is_request(Some(Arc::new(42)));
        is_request(Ok(Arc::new(50)));
    }

    fn is_inject<T>(_: T)
    where
        T: Inject,
    {
    }

    fn test_is_inject() {
        is_inject(Arc::new("hello".to_string()));
        is_inject(Some(Arc::new(42)));
        is_inject(Ok(Arc::new(50)));
    }
}
