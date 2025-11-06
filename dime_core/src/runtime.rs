//! Traits for async runtime.

/// An async runtime to spawn asynchronous tasks.
pub trait Runtime: Clone + Send + Sync + 'static {
    /// A handle to a running task.
    type Task<T>: Task<Output = T, Error: Send + 'static, Join: Send + 'static> + Send
    where
        T: Send + 'static;

    /// Spawns an asynchronous task.
    fn spawn<F>(&self, fut: F) -> Self::Task<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
}

/// A handle to a running task.
pub trait Task {
    /// The output type returned by the task.
    type Output;

    /// The error type that may occur during spawning a task.
    type Error;

    /// The future returned by [`join`](Self::join) method.
    type Join: Future<Output = Result<Self::Output, Self::Error>>;

    /// Aborts the task associated with the handle.
    fn abort(&self);

    /// Takes the handle and yields until the task is completed.
    fn join(self) -> Self::Join;
}

/// Aborts the wrapped task when it is dropped.
pub struct AbortOnDrop<T: Task>(Option<T>);

impl<T: Task> AbortOnDrop<T> {
    /// Wraps a task in a new `AbortOnDrop`.
    pub const fn new(task: T) -> Self {
        Self(Some(task))
    }

    /// Unwraps `self` and returns the original task.
    #[expect(clippy::missing_panics_doc)]
    pub fn into_inner(mut self) -> T {
        self.0
            .take()
            .expect("expected a valid `AbortOnDrop` to contain `Some(task)`")
    }
}

impl<T: Task> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(task) = &mut self.0 {
            task.abort();
        }
    }
}
