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

#[cfg(any(feature = "tokio", test))]
pub use rt_tokio::TokioRuntime;

#[cfg(any(feature = "tokio", test))]
mod rt_tokio {
    use super::{Runtime, Task};

    /// A [`tokio`] runtime.
    #[derive(Clone, Default, Debug)]
    pub struct TokioRuntime {}

    /// A wrapper to task spawned by [`TokioRuntime`].
    #[derive(Debug)]
    pub struct TokioTask<T> {
        handle: tokio::task::JoinHandle<T>,
    }

    impl TokioRuntime {
        /// Creates a runtime.
        #[inline]
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl Runtime for TokioRuntime {
        type Task<T>
            = TokioTask<T>
        where
            T: Send + 'static;

        #[inline]
        fn spawn<F>(&self, fut: F) -> Self::Task<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            TokioTask {
                handle: tokio::task::spawn(fut),
            }
        }
    }

    impl<T> Task for TokioTask<T> {
        type Output = T;

        type Error = tokio::task::JoinError;

        type Join = tokio::task::JoinHandle<T>;

        #[inline]
        fn abort(&self) {
            self.handle.abort();
        }

        #[inline]
        fn join(self) -> Self::Join {
            self.handle
        }
    }
}

pub(crate) struct AbortOnDrop<T: Task>(T);

impl<T: Task> AbortOnDrop<T> {
    pub const fn new(task: T) -> Self {
        Self(task)
    }
}

impl<T: Task> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}
