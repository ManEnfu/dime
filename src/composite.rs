//! Composite components.

use std::sync::Arc;

use crate::injector::{Injector, Watch};
use crate::result::{ResolutionError, Result};

/// A trait for types that may consists of multiple component.
pub trait Composite<I>: Sized {
    type Watch: Watch<Ty = Self>;

    /// Tells the injector that the components that make up this type might be injected to it.
    fn promise_to(injector: &I);

    /// Injects the components that make up this type to the injector.
    fn inject_to(result: Result<Self>, injector: &I);

    /// Watches for values of components that make up this types from the injector.
    fn watch_from(injector: &I) -> Self::Watch;
}

impl<I, T> Composite<I> for Arc<T>
where
    I: Injector,
    T: ?Sized + Send + Sync + 'static,
{
    type Watch = I::Watch<Self>;

    fn promise_to(injector: &I) {
        injector.define::<Self>();
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        injector.inject(result);
    }

    fn watch_from(injector: &I) -> Self::Watch {
        injector.watch()
    }
}

/// A wrapper to make any type implement [`Composite`].
pub struct Component<T>(pub T);

// We can assume that injectors always have `()` unit component, so injecting `()` into any
// injector is no-op and watching for `()` always immediately return `Ok(())`.
impl<I> Composite<I> for () {
    type Watch = ();

    fn promise_to(_injector: &I) {}

    fn inject_to(_result: Result<Self>, _injector: &I) {}

    fn watch_from(_injector: &I) -> Self::Watch {}
}

impl<I, T> Composite<I> for Component<T>
where
    I: Injector,
    T: Clone + Send + Sync + 'static,
    I::Watch<T>: Send,
{
    type Watch = ComponentWatch<I::Watch<T>>;

    fn promise_to(injector: &I) {
        injector.define::<T>();
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        injector.inject(result.map(|v| v.0));
    }

    fn watch_from(injector: &I) -> Self::Watch {
        ComponentWatch::new(injector.watch())
    }
}

impl<I, T> Composite<I> for Option<T>
where
    I: Injector,
    T: Composite<I> + Clone + Send + Sync + 'static,
    T::Watch: Send,
{
    type Watch = OptionalWatch<T::Watch>;

    fn promise_to(injector: &I) {
        T::promise_to(injector);
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        T::inject_to(
            result.and_then(|v| v.ok_or_else(|| ResolutionError::not_defined::<T>())),
            injector,
        );
    }

    fn watch_from(injector: &I) -> Self::Watch {
        OptionalWatch::new(T::watch_from(injector))
    }
}

impl<I, T> Composite<I> for Result<T>
where
    I: Injector,
    T: Composite<I> + Clone + Send + Sync + 'static,
    T::Watch: Send,
{
    type Watch = ResultWatch<T::Watch>;

    fn promise_to(injector: &I) {
        injector.define::<T>();
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        T::inject_to(result.flatten(), injector);
    }

    fn watch_from(injector: &I) -> Self::Watch {
        ResultWatch::new(T::watch_from(injector))
    }
}

macro_rules! impl_composite_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        #[allow(clippy::redundant_clone)]
        impl<I, $($ty,)*> Composite<I> for ($($ty,)*)
        where
            I: Injector,
            $($ty: Composite<I> + Clone + Send + Sync + 'static,)*
            $($ty::Watch: Send,)*
        {
            type Watch = ($($ty::Watch,)*);

            fn promise_to(injector: &I) {
                $($ty::promise_to(injector);)*
            }

            fn inject_to(result: Result<Self>, injector: &I) {
                match result {
                    Ok(($($ty,)*)) => {
                        $($ty::inject_to(Ok($ty), injector);)*
                    },
                    Err(err) => {
                        $($ty::inject_to(Err(err.clone()), injector);)*
                    }
                }
            }

            fn watch_from(injector: &I) -> Self::Watch {
                ($($ty::watch_from(injector),)*)
            }
        }
    }
}

impl_composite_tuple!(T1);
impl_composite_tuple!(T1, T2);
impl_composite_tuple!(T1, T2, T3);
impl_composite_tuple!(T1, T2, T3, T4);
impl_composite_tuple!(T1, T2, T3, T4, T5);
impl_composite_tuple!(T1, T2, T3, T4, T5, T6);
// impl_composite_tuple!(T1, T2, T3, T4, T5, T6, T7);
// impl_composite_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15);
// impl_composite_tuple!(T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16);

/// Watches over values wrapped in [`Component`].
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct ComponentWatch<W>(W);

impl<W> ComponentWatch<W> {
    /// Wraps a watch in a new `OptionalWatch`
    pub const fn new(watch: W) -> Self {
        Self(watch)
    }

    /// Extract the underlying watch from `self`.
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W> Watch for ComponentWatch<W>
where
    W: Watch + Send,
{
    type Ty = Component<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        self.0.current().map(Component)
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        let value = self.0.current_optional()?;
        Ok(value.map(Component))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        self.0.wait().await.map(Component)
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        let value = self.0.wait_optional().await?;
        Ok(value.map(Component))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        self.0.wait_always().await.map(Component)
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}

/// Watches over optional value.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct OptionalWatch<W>(W);

impl<W> OptionalWatch<W> {
    /// Wraps a watch in a new `OptionalWatch`
    pub const fn new(watch: W) -> Self {
        Self(watch)
    }

    /// Extract the underlying watch from `self`.
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W> Watch for OptionalWatch<W>
where
    W: Watch + Send,
{
    type Ty = Option<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        self.0.current_optional()
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        Ok(Some(self.0.current_optional()?))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        self.0.wait_optional().await
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        Ok(Some(self.0.wait_optional().await?))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        Ok(Some(self.0.wait_always().await?))
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}

/// Watches over [`Result`] values.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct ResultWatch<W>(W);

impl<W> ResultWatch<W> {
    /// Wraps a watch in a new `ResultWatch`
    pub const fn new(watch: W) -> Self {
        Self(watch)
    }

    /// Extract the underlying watch from `self`.
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W> Watch for ResultWatch<W>
where
    W: Watch + Send,
{
    type Ty = Result<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        Ok(self.0.current())
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        Ok(Some(self.0.current()))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        Ok(self.0.wait().await)
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        Ok(Some(self.0.wait().await))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        Ok(self.0.wait_always().await)
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}
