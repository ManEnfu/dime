//! [`Injector`] trait and common implementations.

use std::any::{TypeId, type_name};
use std::sync::Arc;

use crate::erased::Erased;
use crate::result::Result;

pub mod state;
use state::{RawWatch, Watch};

mod state_map;
pub use state_map::StateMap;

/// A base trait for container to inject to and retrieve value from.
pub trait Injector: Send + Sync {
    /// Tells the injector that a type might be injected to it.
    ///
    /// Depending on the implementation, Trying to retrieve value (e.g. by calling
    /// [`Watch::available`]) prior to calling this method for its type may panic, wait forever,
    /// or return [`ResolutionError::NotDefined`](crate::result::ResolutionError::NotDefined).
    /// Calling this method ensures that retrieving value of this type will wait until a value
    /// is available.
    fn define_by_type_id(&self, type_id: TypeId, type_name: &'static str);

    /// Inject a value of a given type into the injector.
    ///
    /// # Panics
    ///
    /// The caller must ensure that the type of `value` and the type identified by `type_id`
    /// match. Breaking this contract may cause panic or other sorts of problems.
    /// However as, this method is safe, the implementor must ensure that calls with incorrect
    /// arguments should *not* cause undefined behavior.
    fn inject_by_type_id(&self, type_id: TypeId, type_name: &'static str, value: Result<Erased>);

    /// Watches for values of a given type in the injector.
    ///
    /// # Panics
    ///
    /// The implementation must ensure that the type of values received by the returned
    /// [`RawWatch`] matches with the type identified by `type_id`. Otherwise, breaking this
    /// contract may cause panic or other sorts of problems, but should *not* cause undefined
    /// behavior.
    fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch;
}

impl<I> Injector for Arc<I>
where
    I: Injector,
{
    fn define_by_type_id(&self, type_id: TypeId, type_name: &'static str) {
        (**self).define_by_type_id(type_id, type_name);
    }

    fn inject_by_type_id(&self, type_id: TypeId, type_name: &'static str, value: Result<Erased>) {
        (**self).inject_by_type_id(type_id, type_name, value);
    }

    fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch {
        (**self).raw_watch_by_type_id(type_id, type_name)
    }
}

impl<I> Injector for Box<I>
where
    I: Injector,
{
    fn define_by_type_id(&self, type_id: TypeId, type_name: &'static str) {
        (**self).define_by_type_id(type_id, type_name);
    }

    fn inject_by_type_id(&self, type_id: TypeId, type_name: &'static str, value: Result<Erased>) {
        (**self).inject_by_type_id(type_id, type_name, value);
    }

    fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch {
        (**self).raw_watch_by_type_id(type_id, type_name)
    }
}

/// Type-safe methods for [`Injector`].
pub trait InjectorExt: Injector {
    /// Tells the injector that a type might be injected to it.
    ///
    /// Depending on the implementation, Trying to retrieve value (e.g. by calling
    /// [`Watch::available`]) prior to calling this method for its type may panic, wait forever,
    /// or return [`ResolutionError::NotDefined`](crate::result::ResolutionError::NotDefined).
    /// Calling this method ensures that retrieving value of this type will wait until a value
    /// is available.
    ///
    /// This method is a type-safe variant of [`Injector::define_by_type_id`].
    fn define<T>(&self)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.define_by_type_id(TypeId::of::<T>(), type_name::<T>());
    }

    /// Inject a value of a given type into the injector.
    ///
    /// This method is a type-safe variant of [`Injector::inject_by_type_id`].
    fn inject<T>(&self, value: Result<T>)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.inject_by_type_id(TypeId::of::<T>(), type_name::<T>(), value.map(Erased::new));
    }

    /// Watches for values of a given type in the injector.
    ///
    /// This method is a type-safe variant of [`Injector::raw_watch_by_type_id`].
    fn watch<T>(&self) -> Watch<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        let raw = self.raw_watch_by_type_id(TypeId::of::<T>(), type_name::<T>());

        Watch::from_raw(raw)
    }
}

impl<I> InjectorExt for I where I: ?Sized + Injector {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dyn_compatible() {
        pub struct FakeInjector;

        impl Injector for FakeInjector {
            fn define_by_type_id(&self, _type_id: TypeId, _type_name: &'static str) {}

            fn inject_by_type_id(
                &self,
                _type_id: TypeId,
                _type_name: &'static str,
                _value: Result<Erased>,
            ) {
            }

            fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch {
                state::RawState::new_undefined(type_id, type_name).watch()
            }
        }

        fn check_dyn_compatible(injector: &dyn Injector) {
            injector.define::<i32>();
            injector.inject(Ok("hello"));
            let _ = injector.watch::<bool>();
        }

        check_dyn_compatible(&FakeInjector);
    }
}
