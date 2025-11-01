//! [`Injector`] trait and common implementations.

use std::any::type_name;
use std::pin::Pin;
use std::sync::Arc;

use crate::result::Result;

pub mod state;

mod watch;
pub use watch::Watch;

mod state_map;
pub use state_map::StateMap;

/// A base trait for container to inject to and retrieve value from.
pub trait Injector {
    type Watch<T: Send + 'static>: Watch<Ty = T>;

    /// Tells the injector that a type might be injected to it.
    ///
    /// Depending on the implementation, Trying to retrieve value (e.g. by calling
    /// [`wait`](Watch::wait)) prior to calling this method for its type may panic, wait forever,
    /// or return [`ResolutionError::NotDefined`](crate::result::ResolutionError::NotDefined).
    /// Calling this method ensures that retrieving value of this type will wait until a value
    /// is available.
    fn define<T>(&self)
    where
        T: Clone + Send + Sync + 'static;

    /// Inject a value of a given type into the injector.
    fn inject<T>(&self, value: Result<T>)
    where
        T: Clone + Send + Sync + 'static;

    /// Watches for values of a given type in the injector.
    fn watch<T>(&self) -> Self::Watch<T>
    where
        T: Clone + Send + Sync + 'static;
}

impl<I> Injector for Arc<I>
where
    I: Injector,
{
    type Watch<T: Send + 'static> = I::Watch<T>;

    #[inline]
    fn define<T>(&self)
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).define::<T>();
    }

    #[inline]
    fn inject<T>(&self, value: Result<T>)
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).inject(value);
    }

    #[inline]
    fn watch<T>(&self) -> Self::Watch<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).watch()
    }
}

impl<I> Injector for Box<I>
where
    I: Injector,
{
    type Watch<T: Send + 'static> = I::Watch<T>;

    #[inline]
    fn define<T>(&self)
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).define::<T>();
    }

    #[inline]
    fn inject<T>(&self, value: Result<T>)
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).inject(value);
    }

    #[inline]
    fn watch<T>(&self) -> Self::Watch<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        (**self).watch()
    }
}

/// A task operating around an injector.
pub trait InjectorTask<I> {
    type Future: Future<Output = Result<()>> + Send;

    /// Run the task with the given injector.
    ///
    /// # Errors
    ///
    /// The semantics of the error returned by this method may vary between implementations, but
    /// in the most common case, this method will return an error if the underlying task encounters
    /// an unexpected error, panicks, or is otherwise unable to continue in any way.
    fn run(self, injector: &I) -> Self::Future;
}

impl<I, F, Fut> InjectorTask<I> for F
where
    F: FnOnce(&I) -> Fut,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    type Future = Fut;

    #[inline]
    fn run(self, injector: &I) -> Self::Future {
        self(injector)
    }
}

/// A dispatchable [`InjectorTask`] trait object.
///
/// Use this instead of `Box<dyn InjectorTask>` to dynamically dispatch [`InjectorTask::run`].
pub struct InjectorTaskObject<I> {
    // We don't store `InjectorTask` trait object here, as `run` method is non-dispatchable,
    // using boxed `FnOnce` allow us to consume the boxed value when calling `run`.
    #[allow(clippy::type_complexity)]
    boxed: Box<dyn FnOnce(&I) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send>,
    concrete_type: &'static str,
}

impl<I> InjectorTaskObject<I> {
    /// Creates a new `BoxedInjectorTask` from a concrete task.
    pub fn new<T>(task: T) -> Self
    where
        T: InjectorTask<I> + Send + 'static,
        T::Future: Send + 'static,
    {
        let wrapped_fn = |injector: &I| {
            Box::pin(task.run(injector)) as Pin<Box<dyn Future<Output = Result<()>> + Send>>
        };

        Self {
            boxed: Box::new(wrapped_fn),
            concrete_type: type_name::<T>(),
        }
    }

    /// Creates a new `BoxedInjectorTask` from a concrete task that returns future that implements.
    /// `Into<Pin<Box<dyn Future<Output = Result<()>> + Send>>>`.
    ///
    /// This is useful to avoid extra allocation and indirection.
    pub fn from_boxed_future<T>(task: T) -> Self
    where
        T: InjectorTask<I> + Send + 'static,
        T::Future: Into<Pin<Box<dyn Future<Output = Result<()>> + Send>>>,
    {
        let wrapped_fn = |injector: &I| task.run(injector).into();

        Self {
            boxed: Box::new(wrapped_fn),
            concrete_type: type_name::<T>(),
        }
    }
}

impl<I> std::fmt::Debug for InjectorTaskObject<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedInjectorTask")
            .field("concrete_type", &self.concrete_type)
            .finish_non_exhaustive()
    }
}

impl<I> InjectorTask<I> for InjectorTaskObject<I> {
    type Future = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

    #[inline]
    fn run(self, injector: &I) -> Self::Future {
        self.boxed.run(injector)
    }
}
