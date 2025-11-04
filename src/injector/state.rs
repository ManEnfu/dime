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
    fn define(&mut self) -> bool {
        if matches!(self, Self::Undefined) {
            *self = Self::Pending;
            true
        } else {
            false
        }
    }

    fn is_ready_and<F>(&self, f: F) -> bool
    where
        F: FnOnce(&Result<Erased>) -> bool,
    {
        match self {
            Self::Ready(result) => f(result),
            _ => false,
        }
    }
}

/// A state of a given type in [`Injector`](crate::injector::Injector).
///
/// This is a *raw* version of the state, which works with [`Erased`] values.
/// To work with values of concrete types, consider using [`State`].
#[derive(Debug, Clone)]
pub(crate) struct RawState {
    inner: watch::Sender<Inner>,
    type_id: TypeId,
    type_name: &'static str,
}

/// Watches for type-erased values of a given type in [`Injector`](crate::injector::Injector).
///
/// This is a *raw* version of the watch, which works with [`Erased`] values.
/// To work with values of concrete types, consider using [`Watch`].
#[derive(Debug, Clone)]
pub(crate) struct RawWatch {
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
    pub(crate) fn new(type_id: TypeId, type_name: &'static str) -> Self {
        Self::new_inner(Inner::Undefined, type_id, type_name)
    }

    /// Tells the state a type might be injected to it.
    pub(crate) fn define(&self) {
        self.inner.send_if_modified(Inner::define);
    }

    /// Injects a value into the state.
    ///
    /// # Panics
    ///
    /// See [`Injector::inject_by_type_id`](crate::injector::Injector::inject_by_type_id).
    pub(crate) fn inject(&self, value: Result<Erased>) {
        self.inner.send_replace(Inner::Ready(value));
    }

    /// Returns a watch for this state.
    pub(crate) fn watch(&self) -> RawWatch {
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

    pub(crate) fn current(&self) -> Result<Erased> {
        match &*self.inner.borrow() {
            Inner::Undefined | Inner::Pending => {
                Err(ResolutionError::NotDefined(self.type_id, self.type_name))
            }
            Inner::Ready(erased) => erased.clone(),
        }
    }

    pub(crate) fn current_optional(&self) -> Result<Option<Erased>> {
        match &*self.inner.borrow() {
            Inner::Undefined | Inner::Pending => Ok(None),
            Inner::Ready(erased) => erased.clone().map(Some),
        }
    }

    pub(crate) async fn wait(&mut self) -> Result<Erased> {
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

    pub(crate) async fn wait_optional(&mut self) -> Result<Option<Erased>> {
        self.inner
            .wait_for(|state| !matches!(state, Inner::Pending))
            .await
            .map_err(ResolutionError::other)
            .and_then(|state| match &*state {
                Inner::Undefined => Ok(None),
                Inner::Pending => unreachable!(),
                Inner::Ready(result) => result.clone().map(Some),
            })
    }

    pub(crate) async fn wait_always(&mut self) -> Result<Erased> {
        self.inner
            .wait_for(|state| {
                state.is_ready_and(|result| !matches!(result, Err(err) if err.is_not_defined()))
            })
            .await
            .map_err(ResolutionError::other)
            .and_then(|state| match &*state {
                Inner::Ready(result) => result.clone(),
                _ => unreachable!(),
            })
    }

    pub(crate) async fn wait_ok(&mut self) -> Result<Erased> {
        self.inner
            .wait_for(|state| state.is_ready_and(Result::is_ok))
            .await
            .map_err(ResolutionError::other)
            .and_then(|state| match &*state {
                Inner::Ready(Ok(value)) => Ok(value.clone()),
                _ => unreachable!(),
            })
    }

    pub(crate) async fn changed(&mut self) -> Result<()> {
        self.inner.changed().await.map_err(ResolutionError::other)?;

        Ok(())
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
    pub(crate) fn from_raw(raw: RawState) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Tells the state a type might be injected to it.
    #[inline]
    pub fn define(&self) {
        trace!("type" = type_name::<T>(), "define");
        self.raw.define();
    }

    /// Injects a value into the state.
    #[inline]
    pub fn inject(&self, value: Result<T>) {
        trace!(
            "type" = type_name::<T>(),
            error = value.as_ref().err().map(tracing::field::debug),
            "inject"
        );
        self.raw.inject(value.map(Erased::new));
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
    pub(crate) fn from_raw(raw: &'a RawState) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Tells the state a type might be injected to it.
    #[inline]
    pub fn define(&self) {
        trace!("type" = type_name::<T>(), "define");
        self.raw.define();
    }

    /// Injects a value into the state.
    #[inline]
    pub fn inject(&self, value: Result<T>) {
        trace!(
            "type" = type_name::<T>(),
            error = value.as_ref().err().map(tracing::field::debug),
            "inject"
        );
        self.raw.inject(value.map(Erased::new));
    }

    /// Returns a watch for this state.
    #[inline]
    pub fn watch(&self) -> Watch<T> {
        Watch::from_raw(self.raw.watch())
    }
}

impl<T> Watch<T>
where
    T: 'static,
{
    /// Creates a watch from [`RawWatch`].
    ///
    /// # Panics
    ///
    /// Panic may occur if `T` and the underlying type of the values observed by [`RawWatch`] does
    /// not match.
    pub(crate) fn from_raw(raw: RawWatch) -> Self {
        debug_assert_eq!(TypeId::of::<T>(), raw.type_id);

        Self {
            raw,
            _marker: PhantomData,
        }
    }
}

impl<T> super::watch::Watch for Watch<T>
where
    T: 'static + Send,
{
    type Ty = T;

    fn current(&self) -> Result<T> {
        self.raw
            .current()
            .map(|value| value.downcast::<T>().unwrap())
    }

    fn current_optional(&self) -> Result<Option<T>> {
        self.raw
            .current_optional()
            .map(|value| value.map(|value| value.downcast::<T>().unwrap()))
    }

    async fn wait(&mut self) -> Result<T> {
        trace!("type" = type_name::<T>(), "wait");
        self.raw
            .wait()
            .await
            .map(|value| value.downcast::<T>().unwrap())
    }

    async fn wait_optional(&mut self) -> Result<Option<T>> {
        trace!("type" = type_name::<T>(), "wait_optional");
        self.raw
            .wait_optional()
            .await
            .map(|value| value.map(|value| value.downcast::<T>().unwrap()))
    }

    async fn wait_always(&mut self) -> Result<T> {
        trace!("type" = type_name::<T>(), "wait_always");
        self.raw
            .wait_always()
            .await
            .map(|value| value.downcast::<T>().unwrap())
    }

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        trace!("type" = type_name::<T>(), "wait_ok");
        self.raw
            .wait_ok()
            .await
            .map(|value| value.downcast::<T>().unwrap())
    }

    async fn changed(&mut self) -> Result<()> {
        trace!("type" = type_name::<T>(), "wait_changed");
        self.raw.changed().await
    }
}
