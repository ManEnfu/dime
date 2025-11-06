use std::marker::PhantomData;
use std::pin::Pin;

#[cfg(feature = "tracing")]
use tracing::{Instrument, field};

use crate::Result;
use crate::component::{InjectTo, WatchFrom};
use crate::injector::{Injector, InjectorTask, Watch};

/// Constructs a component from smaller components.
pub trait Constructor<T> {
    /// The type of the constructed component.
    type Constructed;

    /// Calls the constructor.
    fn construct(self, param: T) -> Self::Constructed;
}

impl<F, O> Constructor<()> for F
where
    F: FnOnce() -> O,
{
    type Constructed = O;

    fn construct(self, _param: ()) -> Self::Constructed {
        self()
    }
}

macro_rules! impl_constructor_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, O, $($ty,)*> Constructor<($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> O,
        {
            type Constructed = O;

            fn construct(self, param: ($($ty,)*)) -> Self::Constructed {
                let ($($ty,)*) = param;
                self($($ty,)*)
            }
        }
    };
}

apply_tuples!(impl_constructor_tuple);

/// Asynchronously constructs a component from smaller components.
pub trait AsyncConstructor<T> {
    /// The type of the constructed component.
    type Constructed;

    /// The future returned by [`construct`](Self::construct) method.
    type Future: Future<Output = Self::Constructed> + Send;

    /// Calls the constructor.
    fn construct(self, param: T) -> Self::Future;
}

impl<F, Fut> AsyncConstructor<()> for F
where
    F: FnOnce() -> Fut,
    Fut: Future + Send,
{
    type Constructed = Fut::Output;

    type Future = Fut;

    fn construct(self, _param: ()) -> Self::Future {
        self()
    }
}

macro_rules! impl_async_constructor_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($ty,)*> AsyncConstructor<($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> Fut,
            Fut: Future + Send,
        {
            type Constructed = Fut::Output;

            type Future = Fut;

            fn construct(self, param: ($($ty,)*)) -> Self::Future {
                let ($($ty,)*) = param;
                self($($ty,)*)
            }
        }
    };
}

apply_tuples!(impl_async_constructor_tuple);

/// A adapter for [`Constructor`] types so that it implements [`InjectorTask`].
pub struct ConstructorTask<C, T> {
    constructor: C,
    _marker: PhantomData<fn() -> T>,
}

impl<C, T> ConstructorTask<C, T>
where
    C: Constructor<T>,
{
    /// Creates a new [`ConstructorTask`] from a [`Constructor`].
    pub fn new(constructor: C) -> Self {
        Self {
            constructor,
            _marker: PhantomData,
        }
    }
}

impl<I, C, T> InjectorTask<I> for ConstructorTask<C, T>
where
    I: Injector + Clone + Send + 'static,
    T: WatchFrom<I> + Send,
    T::Watch: Send + 'static,
    C: Constructor<T> + Clone + Send + Sync + 'static,
    C::Constructed: InjectTo<I>,
{
    type Future = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

    fn run(self, injector: I) -> Self::Future {
        C::Constructed::promise_to(&injector);

        let fut = async move {
            let mut watch = T::watch_from(&injector);
            trace!("start task");

            loop {
                let input: Result<T> = watch.wait().await;
                trace!(error = input.as_ref().err().map(field::display), "waited");

                {
                    let output: Result<C::Constructed> = match input {
                        Ok(input) => Ok(self.constructor.clone().construct(input)),
                        Err(err) => Err(err),
                    };
                    trace!(
                        error = output.as_ref().err().map(tracing::field::display),
                        "constructed"
                    );

                    C::Constructed::inject_to(output, &injector);
                }

                #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
                watch
                    .changed()
                    .await
                    .inspect_err(|error| error!(%error, "error while waiting for change"))?;
                trace!("changed");
            }
        };

        #[cfg(feature = "tracing")]
        let fut = fut.instrument(tracing::debug_span!(
            "constructor_task",
            dependency = std::any::type_name::<T>(),
            constructed = std::any::type_name::<C::Constructed>(),
        ));

        Box::pin(fut)
    }
}

/// A adapter for [`AsyncConstructor`] types so that it implements [`InjectorTask`].
pub struct AsyncConstructorTask<C, T> {
    constructor: C,
    _marker: PhantomData<fn() -> T>,
}

impl<C, T> AsyncConstructorTask<C, T>
where
    C: AsyncConstructor<T>,
{
    /// Creates a new [`AsyncConstructorTask`] from a [`AsyncConstructor`].
    pub fn new(constructor: C) -> Self {
        Self {
            constructor,
            _marker: PhantomData,
        }
    }
}

impl<I, C, T> InjectorTask<I> for AsyncConstructorTask<C, T>
where
    I: Injector + Clone + Send + 'static,
    T: WatchFrom<I> + Send,
    T::Watch: Send + 'static,
    C: AsyncConstructor<T> + Clone + Send + Sync + 'static,
    C::Constructed: InjectTo<I>,
    C::Future: Send,
{
    type Future = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

    fn run(self, injector: I) -> Self::Future {
        C::Constructed::promise_to(&injector);

        let fut = async move {
            let mut watch = T::watch_from(&injector);
            trace!("start task");

            loop {
                let input: Result<T> = watch.wait().await;
                trace!(error = input.as_ref().err().map(field::display), "waited");

                {
                    let output: Result<C::Constructed> = match input {
                        Ok(input) => Ok(self.constructor.clone().construct(input).await),
                        Err(err) => Err(err),
                    };
                    trace!(
                        error = output.as_ref().err().map(tracing::field::display),
                        "constructed"
                    );

                    C::Constructed::inject_to(output, &injector);
                }

                #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
                watch
                    .changed()
                    .await
                    .inspect_err(|error| error!(%error, "error while waiting for change"))?;
                trace!("changed");
            }
        };

        #[cfg(feature = "tracing")]
        let fut = fut.instrument(tracing::debug_span!(
            "async_constructor_task",
            dependency = std::any::type_name::<T>(),
            constructed = std::any::type_name::<C::Constructed>(),
        ));

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::component::Component;
    use crate::injector::StateMap;

    use super::*;

    struct Foo;
    struct Bar;
    struct Baz;

    trait Qux: Send + Sync + 'static {}
    trait Quz: Send + Sync + 'static {}

    fn assert_constructor<F, T>(_f: F)
    where
        F: Constructor<T>,
    {
    }

    fn assert_async_constructor<F, T>(_f: F)
    where
        F: AsyncConstructor<T>,
    {
    }

    fn assert_injector_task<T>(_task: T)
    where
        T: InjectorTask<Arc<StateMap>>,
    {
    }

    #[test]
    fn test_constructor_bound() {
        assert_constructor(|| -> Component<i32> { unimplemented!() });
        assert_constructor(|| -> (Component<i32>,) { unimplemented!() });
        assert_constructor(|_: Component<bool>| ());
        assert_constructor(|_: Component<bool>| -> Component<i32> { unimplemented!() });
        assert_constructor(|_: Arc<Foo>, _: Arc<Bar>| -> (Arc<Baz>, Arc<String>) {
            unimplemented!()
        });
        assert_constructor(
            |_: Option<Arc<Foo>>, _: Result<Arc<dyn Quz>>| -> Option<Arc<dyn Qux>> {
                unimplemented!()
            },
        );
    }

    #[test]
    fn test_async_constructor_bound() {
        assert_async_constructor(async || -> Component<i32> { unimplemented!() });
        assert_async_constructor(async || -> (Component<i32>,) { unimplemented!() });
        assert_async_constructor(async |_: Component<bool>| ());
        assert_async_constructor(async |_: Component<bool>| -> Component<i32> { unimplemented!() });
        assert_async_constructor(
            async |_: Arc<Foo>, _: Arc<Bar>| -> (Arc<Baz>, Arc<String>) { unimplemented!() },
        );
        assert_async_constructor(
            async |_: Option<Arc<Foo>>, _: Result<Arc<dyn Quz>>| -> Option<Arc<dyn Qux>> {
                unimplemented!()
            },
        );
    }

    #[test]
    fn test_constructor_task_bound() {
        assert_injector_task(ConstructorTask::new(|| -> Component<i32> {
            unimplemented!()
        }));
        assert_injector_task(ConstructorTask::new(|| -> (Component<i32>,) {
            unimplemented!()
        }));
        assert_injector_task(ConstructorTask::new(|_: Component<bool>| ()));
        assert_injector_task(ConstructorTask::new(
            |_: Component<bool>| -> Component<i32> { unimplemented!() },
        ));
        assert_injector_task(ConstructorTask::new(
            |_: Arc<Foo>, _: Arc<Bar>| -> (Arc<Baz>, Arc<String>) { unimplemented!() },
        ));
        assert_injector_task(ConstructorTask::new(
            |_: Option<Arc<Foo>>, _: Result<Arc<dyn Quz>>| -> Option<Arc<dyn Qux>> {
                unimplemented!()
            },
        ));
    }

    #[test]
    fn test_async_constructor_task_bound() {
        assert_injector_task(AsyncConstructorTask::new(async || -> Component<i32> {
            unimplemented!()
        }));
        assert_injector_task(AsyncConstructorTask::new(async || -> (Component<i32>,) {
            unimplemented!()
        }));
        assert_injector_task(AsyncConstructorTask::new(async |_: Component<bool>| ()));
        assert_injector_task(AsyncConstructorTask::new(
            async |_: Component<bool>| -> Component<i32> { unimplemented!() },
        ));
        assert_injector_task(AsyncConstructorTask::new(
            async |_: Arc<Foo>, _: Arc<Bar>| -> (Arc<Baz>, Arc<String>) { unimplemented!() },
        ));
        assert_injector_task(AsyncConstructorTask::new(
            async |_: Option<Arc<Foo>>, _: Result<Arc<dyn Quz>>| -> Option<Arc<dyn Qux>> {
                unimplemented!()
            },
        ));
        assert_injector_task(AsyncConstructorTask::new(
            async |_: Option<Arc<Foo>>,
                   _: (Result<Arc<dyn Quz>>, Component<i32>)|
                   -> Option<Arc<dyn Qux>> { unimplemented!() },
        ));
    }
}
