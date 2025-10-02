use std::{any::TypeId, sync::Arc};

use crate::result::{ResolutionError, Result};

pub trait Resolver: Send + Sync + 'static {}

/// [`Request`] is a trait for components or groups of components which values can be requested
/// from a resolver.
///
/// In most cases, you don't need to implement this trait manually. You can wrap any type inside
/// an [`Arc`] and it will implement [`Request`].
pub trait Request: Clone + Sized + Send + Sync + 'static {
    /// Requests a value of the specified type from a resolver.
    fn request<R>(resolver: &R) -> impl Future<Output = Result<Self>> + Send
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
    async fn request<R>(resolver: &R) -> Result<Self>
    where
        R: Resolver,
    {
        match T::request(resolver).await {
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
    async fn request<R>(resolver: &R) -> Result<Self>
    where
        R: Resolver,
    {
        Ok(T::request(resolver).await)
    }
}

macro_rules! impl_request_tuple {
    ($($ty:ident),*) => {
        impl<$($ty,)*> Request for ($($ty,)*)
        where
            $($ty: Request,)*
        {
            async fn request<R>(resolver: &R) -> Result<Self>
            where
                R: Resolver,
            {
                Ok((
                    $( $ty::request(resolver).await?, )*
                ))
            }
        }
    };
}

apply_tuples!(impl_request_tuple);

/// [`Inject`] is a trait for components or group of components which can be injected
/// into a resolver.
///
/// In most cases, you don't need to implement this trait manually. You can wrap any type inside
/// an [`Arc`] and it will implement [`Inject`].
pub trait Inject: Clone + Sized + Send + Sync + 'static {
    /// Injects `self` into a resolver.
    fn inject<R>(self, resolver: &R) -> impl Future<Output = Result<()>> + Send
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
    async fn inject<R>(self, resolver: &R) -> Result<()>
    where
        R: Resolver,
    {
        if let Some(v) = self {
            v.inject(resolver).await
        } else {
            Err(ResolutionError::not_defined::<T>())
        }
    }
}

impl<T> Inject for Result<T>
where
    T: Inject,
{
    async fn inject<R>(self, resolver: &R) -> Result<()>
    where
        R: Resolver,
    {
        match self {
            Ok(v) => v.inject(resolver).await,
            Err(e) => Err(e),
        }
    }
}

macro_rules! impl_inject_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($ty,)*> Inject for ($($ty,)*)
        where
            $($ty: Inject,)*
        {
            async fn inject<R>(self, resolver: &R) -> Result<()>
            where
                R: Resolver,
            {
                let ($($ty,)*) = self;
                $( $ty.inject(resolver).await?; )*
                Ok(())
            }
        }
    };
}

apply_tuples!(impl_inject_tuple);

#[allow(dead_code)]
#[cfg(test)]
mod comp_tests {
    //! This modules tests if a type implements [`Request`] and [`Inject`] at compile time.

    use super::*;

    #[derive(Clone)]
    struct Foo {}

    impl Request for Foo {
        async fn request<R>(_: &R) -> Result<Self>
        where
            R: Resolver,
        {
            unimplemented!()
        }
    }

    impl Inject for Foo {
        async fn inject<R>(self, _: &R) -> Result<()>
        where
            R: Resolver,
        {
            unimplemented!()
        }
    }

    fn is_request<T>()
    where
        T: Request,
    {
    }

    fn test_is_request() {
        is_request::<Arc<String>>();
        is_request::<Option<Arc<&'static str>>>();
        is_request::<Result<Arc<u32>>>();
        is_request::<(Arc<String>,)>();
        is_request::<(
            Option<Arc<String>>,
            Result<Foo>,
            Arc<std::sync::Mutex<(i128, f64)>>,
        )>();
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
        is_inject((Arc::new("world"),));
        is_inject((
            Some(Arc::new("hello".to_string())),
            Ok(Foo {}),
            Result::<Arc<i32>>::Err(ResolutionError::not_defined::<Arc<i32>>()),
        ));
    }
}
