use std::any::Any;

/// [`CloneBoxed`] is a trait to clone a reference to an `?Sized` type into a [`Box`].
///
/// This trait is used to work around [`Sized`] bound on [`Clone`].
pub trait BoxedClone: Any + Send + Sync {
    /// Returns the boxed clone of `self`.
    fn boxed_clone(&self) -> Box<dyn BoxedClone>;
}

impl<T> BoxedClone for T
where
    T: Any + Clone + Send + Sync,
{
    fn boxed_clone(&self) -> Box<dyn BoxedClone> {
        Box::new(self.clone())
    }
}
