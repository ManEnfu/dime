//! Type value states.

use std::any::{TypeId, type_name};
use std::marker::PhantomData;

use tokio::sync::watch;

use crate::{
    erased::Erased,
    result::{ResolutionError, Result},
};

#[derive(Clone, Debug, Default)]
enum Inner {
    #[default]
    Undefined,
    Pending,
    Ready(Result<Erased>),
}

impl Inner {
    const fn is_defined(&self) -> bool {
        !matches!(self, Self::Undefined)
    }

    const fn is_available(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    fn define(&mut self) -> bool {
        if matches!(self, Self::Undefined) {
            *self = Self::Pending;
            true
        } else {
            false
        }
    }
}

/// A state of a given type in [`Injector`](crate::injector::Injector).
///
/// This is a *raw* version of the state, which works with [`Erased`] values.
/// To work with values of concrete types, consider using [`State`].
#[derive(Debug, Clone)]
pub struct RawState {
    inner: watch::Sender<Inner>,
    type_id: TypeId,
    type_name: &'static str,
}

/// Watches for type-erased values of a given type in [`Injector`](crate::injector::Injector).
///
/// This is a *raw* version of the watch, which works with [`Erased`] values.
/// To work with values of concrete types, consider using [`Watch`].
#[derive(Debug, Clone)]
pub struct RawWatch {
    inner: watch::Receiver<Inner>,
    type_id: TypeId,
    type_name: &'static str,
}

/// A state of a given type in [`Injector`](crate::injector::Injector).
#[derive(Debug, Clone)]
pub struct State<T> {
    raw: RawState,
    _marker: PhantomData<T>,
}

/// A reference to a state of a given type in [`Injector`](crate::injector::Injector).
#[derive(Debug, Clone)]
pub struct StateRef<'a, T> {
    raw: &'a RawState,
    _marker: PhantomData<T>,
}

/// Watches for values of a given type in [`Injector`](crate::injector::Injector).
#[derive(Debug, Clone)]
pub struct Watch<T> {
    raw: RawWatch,
    _marker: PhantomData<T>,
}

impl RawState {
    fn new_inner(inner: Inner, type_id: TypeId, type_name: &'static str) -> Self {
        let (tx, _) = watch::channel(inner);

        Self {
            inner: tx,
            type_id,
            type_name,
        }
    }

    /// Creates a new, undefined state.
    pub fn new(type_id: TypeId, type_name: &'static str) -> Self {
        Self::new_inner(Inner::Undefined, type_id, type_name)
    }

    /// Tells the state a type might be injected to it.
    pub fn define(&self) {
        self.inner.send_if_modified(Inner::define);
    }

    /// Injects a value into the state.
    ///
    /// # Panics
    ///
    /// See [`Injector::inject_by_type_id`](crate::injector::Injector::inject_by_type_id).
    pub fn inject(&self, value: Result<Erased>) {
        self.inner.send_replace(Inner::Ready(value));
    }

    /// Returns true if the type has been defined for the state.
    pub fn is_defined(&self) -> bool {
        self.inner.borrow().is_defined()
    }

    /// Returns true if the type has been available.
    pub fn is_available(&self) -> bool {
        self.inner.borrow().is_available()
    }

    /// Returns a watch for this state.
    pub fn watch(&self) -> RawWatch {
        let rx = self.inner.subscribe();
        RawWatch::new(rx, self.type_id, self.type_name)
    }
}

impl RawWatch {
    const fn new(inner: watch::Receiver<Inner>, type_id: TypeId, type_name: &'static str) -> Self {
        Self {
            inner,
            type_id,
            type_name,
        }
    }

    /// Waits until a value (or an error that occurs while trying to create such value) of type
    /// associated with this watch is available.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the type is not yet defined in the
    /// injector, an error occured during the construction of the value, or the injector has been
    /// dropped.
    pub async fn available(&mut self) -> Result<Erased> {
        self.inner
            .wait_for(|state| !matches!(state, Inner::Pending))
            .await
            .map_err(ResolutionError::other)
            .and_then(|state| match &*state {
                Inner::Undefined => Err(ResolutionError::NotDefined(self.type_id, self.type_name)),
                Inner::Pending => unreachable!(),
                Inner::Ready(result) => result.clone(),
            })
    }

    /// Waits until the type associated with this watch is defined.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    pub async fn defined(&mut self) -> Result<()> {
        self.inner
            .wait_for(|state| !matches!(state, Inner::Undefined))
            .await
            .map_err(ResolutionError::other)?;

        Ok(())
    }

    /// Waits until the type associated with this watch is defined and a value is available.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if an error occured during the
    /// construction of the value, or the injector has been dropped.
    pub async fn defined_and_available(&mut self) -> Result<Erased> {
        self.inner
            .wait_for(|state| matches!(state, Inner::Ready(_)))
            .await
            .map_err(ResolutionError::other)
            .and_then(|state| match &*state {
                Inner::Ready(result) => result.clone(),
                _ => unreachable!(),
            })
    }

    /// Waits until the state of the type associated with this watch is changed.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    pub async fn changed(&mut self) -> Result<()> {
        self.inner.changed().await.map_err(ResolutionError::other)
    }

    /// Returns true if the state of the type associated with this watch has been changed.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    pub fn has_changed(&self) -> Result<bool> {
        self.inner.has_changed().map_err(ResolutionError::other)
    }

    /// Returns true if the type has been defined for the injector.
    pub fn is_defined(&self) -> bool {
        self.inner.borrow().is_defined()
    }

    /// Returns true if the type has been available.
    pub fn is_available(&self) -> bool {
        self.inner.borrow().is_available()
    }
}

impl<T> Default for State<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> State<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a new, undefined state
    pub fn new() -> Self {
        let raw = RawState::new(TypeId::of::<T>(), type_name::<T>());
        Self::from_raw(raw)
    }

    /// Creates a state from [`RawState`].
    ///
    /// # Panics
    ///
    /// Panic may occur if `T` and the underlying type of the value stored in [`RawState`] does
    /// not match.
    pub fn from_raw(raw: RawState) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Tells the state a type might be injected to it.
    #[inline]
    pub fn define(&self) {
        self.raw.define();
    }

    /// Injects a value into the state.
    #[inline]
    pub fn inject(&self, value: Result<Erased>) {
        self.raw.inject(value.map(Erased::new));
    }

    /// Returns true if the type has been defined for the state.
    #[inline]
    pub fn is_defined(&self) -> bool {
        self.raw.is_defined()
    }

    /// Returns true if the type has been available.
    #[inline]
    pub fn is_available(&self) -> bool {
        self.raw.is_available()
    }

    /// Returns a watch for this state.
    #[inline]
    pub fn watch(&self) -> Watch<T> {
        Watch::from_raw(self.raw.watch())
    }

    /// Returns a reference to this state.
    #[inline]
    pub fn as_ref(&self) -> StateRef<'_, T> {
        StateRef::from_raw(&self.raw)
    }

    /// Returns a reference to the raw state.
    #[inline]
    pub const fn as_raw(&self) -> &RawState {
        &self.raw
    }

    /// Takes the state and returns the raw version of this state.
    #[inline]
    pub fn into_raw(self) -> RawState {
        self.raw
    }
}

impl<'a, T> StateRef<'a, T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a state from [`RawState`].
    ///
    /// # Panics
    ///
    /// Panic may occur if `T` and the underlying type of the value stored in [`RawState`] does
    /// not match.
    pub fn from_raw(raw: &'a RawState) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Tells the state a type might be injected to it.
    #[inline]
    pub fn define(&self) {
        self.raw.define();
    }

    /// Injects a value into the state.
    #[inline]
    pub fn inject(&self, value: Result<Erased>) {
        self.raw.inject(value.map(Erased::new));
    }

    /// Returns true if the type has been defined for the state.
    #[inline]
    pub fn is_defined(&self) -> bool {
        self.raw.is_defined()
    }

    /// Returns true if the type has been available.
    #[inline]
    pub fn is_available(&self) -> bool {
        self.raw.is_available()
    }

    /// Returns a watch for this state.
    #[inline]
    pub fn watch(&self) -> Watch<T> {
        Watch::from_raw(self.raw.watch())
    }

    /// Returns the owned variant of this state.
    #[inline]
    pub fn to_owned(&self) -> State<T> {
        State::from_raw(self.raw.clone())
    }

    /// Returns a reference to the raw state.
    #[inline]
    pub const fn as_raw(&self) -> &RawState {
        self.raw
    }
}

impl<T> Watch<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a watch from [`RawWatch`].
    ///
    /// # Panics
    ///
    /// Panic may occur if `T` and the underlying type of the values observed by [`RawWatch`] does
    /// not match.
    pub fn from_raw(raw: RawWatch) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Waits until a value (or an error that occurs while trying to create such value) of type
    /// associated with this watch is available.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the type is not yet defined in the
    /// injector, an error occured during the construction of the value, or the injector has been
    /// dropped.
    pub async fn available(&mut self) -> Result<T> {
        self.raw.available().await.map(|value| {
            #[expect(clippy::missing_panics_doc)]
            value.downcast::<T>().unwrap()
        })
    }

    /// Waits until the type associated with this watch is defined.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    #[inline]
    pub async fn defined(&mut self) -> Result<()> {
        self.raw.defined().await
    }

    /// Waits until the type associated with this watch is defined and a value is available.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if an error occured during the
    /// construction of the value, or the injector has been dropped.
    #[inline]
    pub async fn defined_and_available(&mut self) -> Result<T> {
        self.raw.defined_and_available().await.map(|value| {
            #[expect(clippy::missing_panics_doc)]
            value.downcast::<T>().unwrap()
        })
    }

    /// Waits until the state of the type associated with this watch is changed.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    #[inline]
    pub async fn changed(&mut self) -> Result<()> {
        self.raw.changed().await
    }

    /// Returns true if the state of the type associated with this watch has been changed.
    ///
    /// # Errors
    ///
    /// This method returns returns [`ResolutionError`] if the injector has been dropped.
    #[inline]
    pub fn has_changed(&mut self) -> Result<bool> {
        self.raw.has_changed()
    }

    /// Returns true if the type has been defined for the injector.
    #[inline]
    pub fn is_defined(&self) -> bool {
        self.raw.is_defined()
    }

    /// Returns true if the type has been available.
    #[inline]
    pub fn is_available(&self) -> bool {
        self.raw.is_available()
    }

    /// Returns a reference to the raw watch.
    #[inline]
    pub const fn as_raw(&self) -> &RawWatch {
        &self.raw
    }

    /// Takes the watch and returns the raw version of this watch.
    #[inline]
    pub fn into_raw(self) -> RawWatch {
        self.raw
    }
}
