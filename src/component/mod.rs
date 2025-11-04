//! Type-safe component system.

use std::sync::Arc;

use crate::injector::{Injector, Watch};
use crate::result::{ResolutionError, Result};

mod constructor;
pub use constructor::{AsyncConstructor, AsyncConstructorTask, Constructor, ConstructorTask};

/// A component or aggregate of components that can be watched for its values from an injector.
pub trait WatchFrom<I>: Sized {
    /// The watch returned by [`watch_from`](Self::watch_from) method.
    type Watch: Watch<Ty = Self>;

    /// Watches for values of components that make up this types from the injector.
    fn watch_from(injector: &I) -> Self::Watch;
}

/// A component or aggregate of components that can be injected into an injector.
pub trait InjectTo<I>: Sized {
    /// Tells the injector that the components that make up this type might be injected to it.
    fn promise_to(injector: &I);

    /// Injects the components that make up this type to the injector.
    fn inject_to(result: Result<Self>, injector: &I);
}

impl<I, T> WatchFrom<I> for Arc<T>
where
    I: Injector,
    T: ?Sized + Send + Sync + 'static,
{
    type Watch = I::Watch<Self>;

    fn watch_from(injector: &I) -> Self::Watch {
        injector.watch()
    }
}

impl<I, T> InjectTo<I> for Arc<T>
where
    I: Injector,
    T: ?Sized + Send + Sync + 'static,
{
    fn promise_to(injector: &I) {
        injector.define::<Self>();
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        injector.inject(result);
    }
}

// We can assume that injectors always have `()` unit component, so injecting `()` into any
// injector is no-op and watching for `()` always immediately return `Ok(())`.
impl<I> WatchFrom<I> for () {
    type Watch = ();

    fn watch_from(_injector: &I) -> Self::Watch {}
}

impl<I> InjectTo<I> for () {
    fn promise_to(_injector: &I) {}

    fn inject_to(_result: Result<Self>, _injector: &I) {}
}

/// A wrapper around a single component type.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Component<T>(pub T);

impl<I, T> WatchFrom<I> for Component<T>
where
    I: Injector,
    T: Clone + Send + Sync + 'static,
    I::Watch<T>: Send,
{
    type Watch = ComponentWatch<I::Watch<T>>;

    fn watch_from(injector: &I) -> Self::Watch {
        ComponentWatch::new(injector.watch())
    }
}

impl<I, T> InjectTo<I> for Component<T>
where
    I: Injector,
    T: Clone + Send + Sync + 'static,
    I::Watch<T>: Send,
{
    fn promise_to(injector: &I) {
        injector.define::<T>();
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        injector.inject(result.map(|v| v.0));
    }
}

impl<I, T> WatchFrom<I> for Option<T>
where
    T: WatchFrom<I> + Clone + Send + Sync + 'static,
    T::Watch: Send,
{
    type Watch = OptionalWatch<T::Watch>;

    fn watch_from(injector: &I) -> Self::Watch {
        OptionalWatch::new(T::watch_from(injector))
    }
}

impl<I, T> InjectTo<I> for Option<T>
where
    T: InjectTo<I> + Clone + Send + Sync + 'static,
{
    fn promise_to(injector: &I) {
        T::promise_to(injector);
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        T::inject_to(
            result.and_then(|v| v.ok_or_else(|| ResolutionError::not_defined::<T>())),
            injector,
        );
    }
}

impl<I, T> WatchFrom<I> for Result<T>
where
    T: WatchFrom<I> + Clone + Send + Sync + 'static,
    T::Watch: Send,
{
    type Watch = ResultWatch<T::Watch>;

    fn watch_from(injector: &I) -> Self::Watch {
        ResultWatch::new(T::watch_from(injector))
    }
}

impl<I, T> InjectTo<I> for Result<T>
where
    T: InjectTo<I> + Clone + Send + Sync + 'static,
{
    fn promise_to(injector: &I) {
        T::promise_to(injector);
    }

    fn inject_to(result: Result<Self>, injector: &I) {
        T::inject_to(result.flatten(), injector);
    }
}

/// Ignores waiting on a value of the wrapped component.
///
/// Calling any method from the associated watch always returns immediately with the current value
/// stored in the injector state.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Current<T>(pub T);

impl<I, T> WatchFrom<I> for Current<T>
where
    T: WatchFrom<I>,
    T::Watch: Send,
{
    type Watch = CurrentWatch<T::Watch>;

    fn watch_from(injector: &I) -> Self::Watch {
        CurrentWatch::new(T::watch_from(injector))
    }
}

/// Waits until the result of this component's evaluation is available.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaitAlways<T>(pub T);

impl<I, T> WatchFrom<I> for WaitAlways<T>
where
    T: WatchFrom<I>,
    T::Watch: Send,
{
    type Watch = WaitAlwaysWatch<T::Watch>;

    fn watch_from(injector: &I) -> Self::Watch {
        WaitAlwaysWatch::new(T::watch_from(injector))
    }
}

/// Waits until the `Ok` value of this component's is available.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaitOk<T>(pub T);

impl<I, T> WatchFrom<I> for WaitOk<T>
where
    T: WatchFrom<I>,
    T::Watch: Send,
{
    type Watch = WaitOkWatch<T::Watch>;

    fn watch_from(injector: &I) -> Self::Watch {
        WaitOkWatch::new(T::watch_from(injector))
    }
}

macro_rules! impl_composite_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        #[allow(clippy::redundant_clone)]
        impl<I, $($ty,)*> WatchFrom<I> for ($($ty,)*)
        where
            I: Injector,
            $($ty: WatchFrom<I> + Clone + Send + Sync + 'static,)*
            $($ty::Watch: Send,)*
        {
            type Watch = ($($ty::Watch,)*);

            fn watch_from(injector: &I) -> Self::Watch {
                ($($ty::watch_from(injector),)*)
            }
        }

        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        #[allow(clippy::redundant_clone)]
        impl<I, $($ty,)*> InjectTo<I> for ($($ty,)*)
        where
            I: Injector,
            $($ty: InjectTo<I> + Clone + Send + Sync + 'static,)*
        {
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
        }
    }
}

apply_tuples!(impl_composite_tuple);

/// Watches over values wrapped in [`Component`].
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct ComponentWatch<W>(W);

impl<W> ComponentWatch<W> {
    /// Wraps a watch in a new `OptionalWatch`
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
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

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        self.0.wait_ok().await.map(Component)
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
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
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

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        Ok(Some(self.0.wait_ok().await?))
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
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
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

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        Ok(self.0.wait_ok().await)
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}

/// Watches over [`Current`] values.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct CurrentWatch<W>(W);

impl<W> CurrentWatch<W> {
    /// Wraps a watch in a new `CurrentWatch`
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
    }
}

impl<W> Watch for CurrentWatch<W>
where
    W: Watch + Send,
{
    type Ty = Current<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        self.0.current().map(Current)
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        let value = self.0.current_optional()?;
        Ok(value.map(Current))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        self.0.current().map(Current)
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        let value = self.0.current_optional()?;
        Ok(value.map(Current))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        self.0.current().map(Current)
    }

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        self.0.current().map(Current)
    }

    fn changed(&mut self) -> impl Future<Output = Result<()>> + Send {
        std::future::pending()
    }
}

/// Watches over [`WaitAlways`] values.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct WaitAlwaysWatch<W>(W);

impl<W> WaitAlwaysWatch<W> {
    /// Wraps a watch in a new `WaitAlwaysWatch`
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
    }
}

impl<W> Watch for WaitAlwaysWatch<W>
where
    W: Watch + Send,
{
    type Ty = WaitAlways<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        self.0.current().map(WaitAlways)
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        let value = self.0.current_optional()?;
        Ok(value.map(WaitAlways))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        self.0.wait_always().await.map(WaitAlways)
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        let value = self.0.wait_always().await?;
        Ok(Some(WaitAlways(value)))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        self.0.wait_always().await.map(WaitAlways)
    }

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        self.0.wait_ok().await.map(WaitAlways)
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}

/// Watches over [`WaitOk`] values.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct WaitOkWatch<W>(W);

impl<W> WaitOkWatch<W> {
    /// Wraps a watch in a new `WaitOkWatch`
    pub(crate) const fn new(watch: W) -> Self {
        Self(watch)
    }
}

impl<W> Watch for WaitOkWatch<W>
where
    W: Watch + Send,
{
    type Ty = WaitOk<W::Ty>;

    fn current(&self) -> Result<Self::Ty> {
        self.0.current().map(WaitOk)
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        let value = self.0.current_optional()?;
        Ok(value.map(WaitOk))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        self.0.wait_always().await.map(WaitOk)
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        let value = self.0.wait_always().await?;
        Ok(Some(WaitOk(value)))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        self.0.wait_always().await.map(WaitOk)
    }

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        self.0.wait_ok().await.map(WaitOk)
    }

    async fn changed(&mut self) -> Result<()> {
        self.0.changed().await
    }
}
