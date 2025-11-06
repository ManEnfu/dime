use crate::runtime::{Runtime, Task};

/// A `tokio` runtime.
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
