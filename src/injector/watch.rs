#![allow(clippy::missing_errors_doc)]

use crate::result::Result;

/// Watches for values of a given type in [`Injector`](crate::injector::Injector).
pub trait Watch<T> {
    /// Immediately retrieves the current value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn current(&self) -> Result<T>;

    /// Immediately retrieves the current value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn current_optional(&self) -> Result<Option<T>>;

    /// Waits until a value of type `T` is available if the injector is promised such value.
    ///
    /// # Errors
    ///
    /// This method returns
    /// [`ResolutionError::NotDefined`](crate::result::ResolutionError::NotDefined)
    /// if no value of type `T` is promised to the injector.
    /// Otherwise, this method returns [`ResolutionError`](crate::result::ResolutionError) if the
    /// evaluation of the value returned an error.
    fn wait(&mut self) -> impl Future<Output = Result<T>> + Send;

    /// Waits until a value of type `T` is available if the injector is promised such value,
    /// returning `None` otherwise.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn wait_optional(&mut self) -> impl Future<Output = Result<Option<T>>> + Send;

    /// Waits until a value of type `T` is available regardless if the injector is promised such
    /// value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn wait_always(&mut self) -> impl Future<Output = Result<T>> + Send;

    /// Waits until the value of type `T` changes.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn changed(&mut self) -> impl Future<Output = Result<()>> + Send;
}
