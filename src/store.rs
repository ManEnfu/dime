//! Collection type for heterogenous types.

use std::{any::TypeId, collections::HashMap};

use crate::erased::Erased;

/// [`Store`] is a collection of values of arbitrary type.
///
/// Each value is identified by its type. Therefore, a [`Store`] can only contains at most one
/// value for each unique concrete type. If you need to store multiple values with the same type,
/// you can use newtype pattern.
///
/// The values stored in this store must implement [`Clone`], [`Send`], and [`Sync`],
/// and must be `'static`.
#[derive(Debug, Clone)]
pub struct Store(HashMap<TypeId, Erased>);

impl Store {
    /// Creates a new [`Store`].
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Inserts a value of the specified type into the store.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.insert_erased(Erased::new(value)).1.map(|v| {
            #[expect(
                clippy::missing_panics_doc,
                reason = "it is guaranteed that v.type_id() == TypeId::of::<T>()"
            )]
            v.downcast()
                .expect("`the returned value should be of type `T`")
        })
    }

    /// Inserts an [`Erased`] value into the store.
    #[inline]
    pub fn insert_erased(&mut self, value: Erased) -> (TypeId, Option<Erased>) {
        let type_id = value.as_any().type_id();
        (type_id, self.0.insert(type_id, value))
    }

    /// Returns a reference to the value of the specified type.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_by_id(TypeId::of::<T>()).map(|v| {
            #[expect(
                clippy::missing_panics_doc,
                reason = "it is guaranteed that v.type_id() == TypeId::of::<T>()"
            )]
            v.as_any()
                .downcast_ref()
                .expect("`the returned value should be of type `T`")
        })
    }

    /// Returns a reference to the [`Erased`] value corresponding to `type_id`.
    #[inline]
    pub fn get_by_id(&self, type_id: TypeId) -> Option<&Erased> {
        self.0.get(&type_id)
    }

    /// Returns a mutable reference to the value of the specified type.
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_mut_by_id(TypeId::of::<T>()).map(|v| {
            #[expect(
                clippy::missing_panics_doc,
                reason = "it is guaranteed that v.type_id() == TypeId::of::<T>()"
            )]
            v.as_mut_any()
                .downcast_mut()
                .expect("`the returned value should be of type `T`")
        })
    }

    /// Returns a mutable reference to the [`Erased`] value corresponding to `type_id`.
    #[inline]
    pub fn get_mut_by_id(&mut self, type_id: TypeId) -> Option<&mut Erased> {
        self.0.get_mut(&type_id)
    }

    /// Removes a value of the specified type from the store and returns it, if one exists.
    pub fn remove<T>(&mut self) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.remove_by_id(TypeId::of::<T>()).map(|v| {
            #[expect(
                clippy::missing_panics_doc,
                reason = "it is guaranteed that v.type_id() == TypeId::of::<T>()"
            )]
            v.downcast()
                .expect("`the returned value should be of type `T`")
        })
    }

    /// Removes a value corresponding to `type_id` and returns the [`Erased`] version of it,
    /// if one exists.
    #[inline]
    pub fn remove_by_id(&mut self, type_id: TypeId) -> Option<Erased> {
        self.0.remove(&type_id)
    }

    /// Returns `true` if the store contains a value of the specified type.
    #[inline]
    pub fn contains<T>(&self) -> bool
    where
        T: Clone + Send + Sync + 'static,
    {
        self.contains_id(TypeId::of::<T>())
    }

    /// Returns `true` if the store contains a value corresponding to `type_id`
    #[inline]
    pub fn contains_id(&self, type_id: TypeId) -> bool {
        self.0.contains_key(&type_id)
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut store = Store::new();
        assert!(store.insert("owned".to_string()).is_none());
        assert!(store.insert("borrowed").is_none());
        let got: &String = store.get().unwrap();
        assert_eq!(got, "owned");
        assert!(store.get::<i32>().is_none());
    }

    #[test]
    fn test_insert_and_replace() {
        let mut store = Store::new();
        assert!(store.insert("owned".to_string()).is_none());
        let got = store.insert("owned2".to_string()).unwrap();
        assert_eq!(got, "owned");
        let got: &String = store.get().unwrap();
        assert_eq!(got, "owned2");
    }

    #[test]
    fn test_remove() {
        let mut store = Store::new();
        assert!(store.insert("owned".to_string()).is_none());
        let got: String = store.remove().unwrap();
        assert_eq!(got, "owned");
        assert!(store.get::<String>().is_none());
    }
}
