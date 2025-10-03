use std::{any::TypeId, sync::Arc};

use crate::result::{ResolutionError, Result};

pub trait Resolver: Send + Sync + 'static {}

/// [`RequestFrom`] is a trait for components or groups of components which values can be requested
/// from a resolver.
///
/// In most cases, you don't need to implement this trait manually. You can wrap any type inside
/// an [`Arc`] and it will implement [`RequestFrom`].
pub trait RequestFrom<R>: Clone + Sized + Send + Sync + 'static
where
    R: Send + Sync + 'static,
{
    /// Requests a value of the specified type from a resolver.
    fn request_from(resolver: &R) -> impl Future<Output = Result<Self>> + Send;
}

impl<R, T> RequestFrom<R> for Arc<T>
where
    R: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn request_from(_: &R) -> Result<Self> {
        todo!()
    }
}

impl<R, T> RequestFrom<R> for Option<T>
where
    R: Send + Sync + 'static,
    T: RequestFrom<R>,
{
    async fn request_from(resolver: &R) -> Result<Self> {
        match T::request_from(resolver).await {
            Ok(v) => Ok(Some(v)),
            Err(ResolutionError::NotDefined(type_id, _)) if type_id == TypeId::of::<T>() => {
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }
}

impl<R, T> RequestFrom<R> for Result<T>
where
    R: Send + Sync + 'static,
    T: RequestFrom<R>,
{
    async fn request_from(resolver: &R) -> Result<Self> {
        Ok(T::request_from(resolver).await)
    }
}

macro_rules! impl_request_tuple {
    ($($ty:ident),*) => {
        impl<R, $($ty,)*> RequestFrom<R> for ($($ty,)*)
        where
            R: Send + Sync + 'static,
            $($ty: RequestFrom<R>,)*
        {
            async fn request_from(resolver: &R) -> Result<Self>
            {
                Ok((
                    $( $ty::request_from(resolver).await?, )*
                ))
            }
        }
    };
}

apply_tuples!(impl_request_tuple);

/// [`InjectTo`] is a trait for components or group of components which can be injected
/// into a resolver.
///
/// In most cases, you don't need to implement this trait manually. You can wrap any type inside
/// an [`Arc`] and it will implement [`InjectTo`].
pub trait InjectTo<R>: Clone + Sized + Send + Sync + 'static
where
    R: Send + Sync + 'static,
{
    /// Injects `self` into a resolver.
    fn inject_to(self, resolver: &R) -> impl Future<Output = Result<()>> + Send;
}

impl<R, T> InjectTo<R> for Arc<T>
where
    R: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn inject_to(self, _: &R) -> Result<()> {
        todo!()
    }
}

impl<R, T> InjectTo<R> for Option<T>
where
    R: Send + Sync + 'static,
    T: InjectTo<R>,
{
    async fn inject_to(self, resolver: &R) -> Result<()> {
        if let Some(v) = self {
            v.inject_to(resolver).await
        } else {
            Err(ResolutionError::not_defined::<T>())
        }
    }
}

impl<R, T> InjectTo<R> for Result<T>
where
    R: Send + Sync + 'static,
    T: InjectTo<R>,
{
    async fn inject_to(self, resolver: &R) -> Result<()> {
        match self {
            Ok(v) => v.inject_to(resolver).await,
            Err(e) => Err(e),
        }
    }
}

macro_rules! impl_inject_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<R, $($ty,)*> InjectTo<R> for ($($ty,)*)
        where
            R: Send + Sync + 'static,
            $($ty: InjectTo<R>,)*
        {
            async fn inject_to(self, resolver: &R) -> Result<()>
            {
                let ($($ty,)*) = self;
                $( $ty.inject_to(resolver).await?; )*
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

    impl<R> RequestFrom<R> for Foo
    where
        R: Send + Sync + 'static,
    {
        async fn request_from(_: &R) -> Result<Self> {
            unimplemented!()
        }
    }

    impl<R> InjectTo<R> for Foo
    where
        R: Send + Sync + 'static,
    {
        async fn inject_to(self, _: &R) -> Result<()> {
            unimplemented!()
        }
    }

    fn is_request<T>()
    where
        T: RequestFrom<()>,
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
        T: InjectTo<()>,
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
