use std::any::Any;

/// Clones a trait object into a [`Box`].
///
/// As [`Clone`] requires its implementor to be [`Sized`], it is not
/// dyn-compatible. This trait allows safe cloning on DSTs and trait objects
/// at the cost of heap allocation.
pub trait DynClone: Any + Send + Sync {
    /// Clones `self` and wraps it inside a [`Box`].
    fn dyn_clone(&self) -> Box<dyn DynClone>;
}

impl<T> DynClone for T
where
    T: Any + Clone + Send + Sync,
{
    fn dyn_clone(&self) -> Box<dyn DynClone> {
        Box::new(self.clone())
    }
}
