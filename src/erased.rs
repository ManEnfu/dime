//! Utilites around opaque values with erased type informations.

use std::any::Any;

/// [`CloneBoxed`] is a trait to clone a reference to an `?Sized` type into a [`Box`].
///
/// This trait is used to work around [`Sized`] bound on [`Clone`].
trait CloneBoxed: Any + Send + Sync {
    /// Returns the boxed clone of `self`.
    fn clone_boxed(&self) -> Box<dyn CloneBoxed>;
}

impl<T> CloneBoxed for T
where
    T: Any + Clone + Send + Sync,
{
    fn clone_boxed(&self) -> Box<dyn CloneBoxed> {
        Box::new(self.clone())
    }
}

/// [`Erased`] is a container for value of an arbitrary type, as long as it
/// implements [`Clone`], [`Send`], and [`Sync`] and is `'static`.
pub struct Erased(Box<dyn CloneBoxed + Send + Sync>);

impl Erased {
    /// Creates a new `Erased` with the provided `value` of type `T`.
    ///
    /// `T` must be `'static` implement [`Clone`], [`Send`], and [`Sync`].
    #[must_use]
    pub fn new<T>(value: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        Self(Box::new(value) as Box<dyn CloneBoxed + Send + Sync>)
    }

    /// Tries to downcast `self` into type `T`.
    ///
    /// # Errors
    ///
    /// If the underlying value is not of type `T`, this method will return
    /// itself as error.
    pub fn downcast<T>(self) -> Result<T, Self>
    where
        T: Clone + Send + Sync + 'static,
    {
        if (&*self.0 as &dyn Any).is::<T>() {
            #[expect(clippy::missing_panics_doc, reason = "already checked")]
            let concrete = (self.0 as Box<dyn Any + Send + Sync>)
                .downcast::<T>()
                .expect("the concrete type of this box should be `T` as it was checked before downcasting.");
            Ok(*concrete)
        } else {
            Err(self)
        }
    }
}

impl std::ops::Deref for Erased {
    type Target = dyn Any + Send + Sync;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for Erased {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Clone for Erased {
    fn clone(&self) -> Self {
        Self(self.0.clone_boxed())
    }
}

impl std::fmt::Debug for Erased {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Erased").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, sync::Arc};

    use super::Erased;

    #[allow(dead_code)]
    fn test_implements_send_and_sync() -> impl Send + Sync {
        Erased::new("Hello".to_string())
    }

    #[test]
    fn test_downcast() {
        let erased = Erased::new("Hello".to_string());
        let got = erased.downcast::<String>().unwrap();
        assert_eq!(got, "Hello");
    }

    #[test]
    fn test_downcast_err() {
        let erased = Erased::new("Hello".to_string());
        let err = erased.downcast::<i32>().unwrap_err();

        let got = err.downcast::<String>().unwrap();
        assert_eq!(got, "Hello");
    }

    #[test]
    fn test_downcast_ref() {
        let erased = Erased::new("Hello".to_string());
        let got = erased.downcast_ref::<String>().unwrap();
        assert_eq!(got, "Hello");
    }

    #[test]
    fn test_downcast_ref_err() {
        let erased = Erased::new("Hello".to_string());
        assert!(erased.downcast_ref::<i32>().is_none());
    }

    #[test]
    fn test_downcast_mut() {
        let mut erased = Erased::new("Hello".to_string());
        erased
            .downcast_mut::<String>()
            .unwrap()
            .push_str(", world!");
        let got = erased.downcast_ref::<String>().unwrap();
        assert_eq!(got, "Hello, world!");
    }

    #[test]
    fn test_downcast_mut_err() {
        let mut erased = Erased::new("Hello".to_string());
        assert!(erased.downcast_mut::<i32>().is_none());
    }

    #[test]
    fn test_type_id() {
        let erased = Erased::new("Hello".to_string());
        assert_eq!(erased.type_id(), TypeId::of::<String>());
    }

    #[test]
    fn test_clone() {
        let a = Arc::new(100);
        let erased = Erased::new(Arc::clone(&a));
        assert_eq!(Arc::strong_count(&a), 2);

        let cloned = erased.clone();
        assert_eq!(Arc::strong_count(&a), 3);

        drop(cloned);
        drop(erased);
    }

    #[test]
    fn test_drop() {
        let a = Arc::new(100);
        let erased = Erased::new(Arc::clone(&a));
        assert_eq!(Arc::strong_count(&a), 2);

        drop(erased);
        assert_eq!(Arc::strong_count(&a), 1);
    }
}
