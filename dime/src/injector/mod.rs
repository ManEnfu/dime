//! [`Injector`] trait and common implementations.

use std::any::type_name;
use std::pin::Pin;

#[doc(inline)]
pub use dime_core::injector::{Injector, InjectorTask, Watch};

use crate::Result;

pub mod state;

mod state_map;
pub use state_map::StateMap;

/// A dispatchable [`InjectorTask`] trait object.
///
/// Use this instead of `Box<dyn InjectorTask>` to dynamically dispatch [`InjectorTask::run`].
pub struct InjectorTaskObject<I> {
    // We don't store `InjectorTask` trait object here, as `run` method is non-dispatchable,
    // using boxed `FnOnce` allow us to consume the boxed value when calling `run`.
    #[allow(clippy::type_complexity)]
    boxed: Box<dyn FnOnce(I) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send>,
    concrete_type: &'static str,
}

impl<I> InjectorTaskObject<I> {
    /// Creates a new `BoxedInjectorTask` from a concrete task.
    pub fn new<T>(task: T) -> Self
    where
        T: InjectorTask<I> + Send + 'static,
        T::Future: Send + 'static,
    {
        let wrapped_fn = |injector: I| {
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
        let wrapped_fn = |injector: I| task.run(injector).into();

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
    fn run(self, injector: I) -> Self::Future {
        self.boxed.run(injector)
    }
}
