//! Constructor functions and types.

use std::pin::Pin;

use crate::{
    component::{InjectTo, RequestFrom},
    result::Result,
};

/// [`Constructor`] is a trait to describe how to construct components.
///
/// Int most cases, you don't need to implement this trait manually, as
/// [`Constructor`] is automatically implemented on the following types:
///
/// - Types that implement [`InjectTo`].
/// - Functions that take [`RequestFrom`] parameters and return type
///   that implements [`InjectTo`].
/// - Async functions that take [`RequestFrom`] parameters and return future
///   that resolves to a type that implements [`InjectTo`].
pub trait Constructor<T, R>: Clone + Sized + Send + Sync + 'static {
    /// Calls the constructor on a resolver.
    fn call(self, resolver: &R) -> impl Future<Output = Result<()>> + Send;
}

impl<R, O> Constructor<(), R> for O
where
    R: Send + Sync + 'static,
    O: InjectTo<R>,
{
    async fn call(self, resolver: &R) -> Result<()> {
        self.inject_to(resolver).await
    }
}

macro_rules! impl_constructor_fn {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, R, $($ty,)* O> Constructor<((O,), $($ty,)*), R> for F
        where
            R: Send + Sync + 'static,
            F: FnOnce($($ty,)*) -> O + Clone + Send + Sync + 'static,
            $( $ty: RequestFrom<R>, )*
            O: InjectTo<R>,
        {
            async fn call(self, resolver: &R) -> Result<()> {
                let ($($ty,)*) = <($($ty,)*)>::request_from(resolver).await?;
                let res = self($($ty,)*);
                res.inject_to(resolver).await
            }
        }
    };
}

apply_tuples!(impl_constructor_fn);

macro_rules! impl_constructor_async_fn {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, Fut, R, $($ty,)* O> Constructor<(Pin<Fut>, $($ty,)*), R> for F
        where
            R: Send + Sync + 'static,
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = O> + Send,
            $( $ty: RequestFrom<R>, )*
            O: InjectTo<R>,
        {
            async fn call(self, resolver: &R) -> Result<()> {
                let ($($ty,)*) = <($($ty,)*)>::request_from(resolver).await?;
                let res = self($($ty,)*).await;
                res.inject_to(resolver).await
            }
        }
    };
}

apply_tuples!(impl_constructor_async_fn);

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    //! This modules tests if a type implements [`Constructor`] at compile time.

    use std::sync::Arc;

    use super::*;

    struct Foo(pub String);

    struct Bar {
        x: String,
        y: i32,
    }

    struct Baz {
        foo: Arc<Foo>,
        bar: Arc<Bar>,
    }

    struct Qux {
        x: u32,
    }

    struct Quz {
        x: String,
        y: u32,
    }

    fn is_constructor<C, T>(_: C)
    where
        C: Constructor<T, ()>,
    {
    }

    fn test_is_constructor() {
        is_constructor(Arc::new(Bar {
            x: String::default(),
            y: 0,
        }));
        is_constructor(
            |_: Result<Arc<Foo>>, _: Option<Arc<Bar>>| -> Result<Arc<Baz>> { unimplemented!() },
        );
        is_constructor(async |_: Arc<String>| -> Arc<Foo> { unimplemented!() });
        is_constructor(async |_: Arc<String>| -> Result<Arc<Foo>> { unimplemented!() });
        is_constructor(
            async |_: Result<Arc<Foo>>, _: Option<Arc<Bar>>| -> Result<Arc<Baz>> {
                unimplemented!()
            },
        );
        is_constructor(
            async |_: Result<Arc<Foo>>, _: (Option<Arc<Bar>>, Arc<Qux>)| -> (Arc<Baz>, Arc<Quz>) {
                unimplemented!()
            },
        );
    }
}
