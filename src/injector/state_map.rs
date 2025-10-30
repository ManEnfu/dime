use std::any::{TypeId, type_name};
use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::erased::Erased;
use crate::injector::Injector;
use crate::injector::state::{RawState, StateRef, Watch};
use crate::result::Result;

use super::state::RawWatch;

/// A Simple injector backed by [`BTreeMap`].
///
/// # Examples
///
/// The following example demonstrates how to connect to a database with an address as its
/// dependency:
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// # use std::time::Duration;
/// #
/// # use tokio::time::timeout;
///
/// use dime::injector::{StateMap, InjectorExt};
/// use dime::result::ResolutionError;
///
/// # const TIMEOUT: Duration = Duration::from_millis(500);
/// #
/// # #[derive(Clone, Debug, Default, PartialEq, Eq)]
/// # struct Address(&'static str);
/// #
/// # #[derive(Clone, Debug)]
/// # struct Database(Arc<DatabaseInner>);
/// #
/// # #[derive(Debug)]
/// # struct DatabaseInner {
/// #     address: Address,
/// #     connected: AtomicBool,
/// # }
/// #
/// # impl Database {
/// #     fn connect(address: Address) -> Self {
/// #         Self(Arc::new(DatabaseInner {
/// #             address,
/// #             connected: AtomicBool::new(true),
/// #         }))
/// #     }
/// #
/// #     fn address(&self) -> &Address {
/// #         &self.0.address
/// #     }
/// #
/// #     fn disconnect(&self) {
/// #         self.0.connected.store(false, Ordering::Relaxed);
/// #     }
/// #
/// #     fn is_connected(&self) -> bool {
/// #         self.0.connected.load(Ordering::Relaxed)
/// #     }
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let injector = Arc::new(StateMap::new());
///
/// let mut watch_db = injector.watch::<Database>();
///
/// // If we try to request a database value, it will return an error!
/// # timeout(TIMEOUT, async {
/// let res = watch_db.available().await;
/// assert!(res.is_err_and(|err| err.is_not_defined_for::<Database>()));
/// # })
/// # .await?;
///
/// // Spawn an async task that will connect to our database from the injected address.
/// let cloned = injector.clone();
/// tokio::spawn(async move {
///     let injector = cloned;
///
///     injector.define::<Database>();
///     let mut watch_address = injector.watch::<Address>();
///     let mut current_db: Option<Database> = None;
///
///     loop {
///         match watch_address.available().await {
///             Ok(address) => {
///                 // Connect to a new database.
///                 let db = Database::connect(address);
///
///                 // Disconnect old database.
///                 if let Some(db) = current_db.take() {
///                     db.disconnect();
///                 }
///                 current_db = Some(db.clone());
///
///                 injector.inject(Ok(db));
///             }
///             Err(err) => injector.inject::<Database>(Err(err)),
///         }
///
///         watch_address.changed().await?;
///     }
///
///     Ok::<(), ResolutionError>(())
/// });
///
/// // Inject a "foo" database address. The injector will return a database connected to "foo".
/// injector.inject(Ok(Address("foo")));
/// # let db1 = timeout(TIMEOUT, async {
/// watch_db.changed().await?;
/// let db1 = watch_db.available().await?;
/// # Ok::<Database, ResolutionError>(db1)
/// # })
/// # .await??;
/// assert_eq!(db1.address(), &Address("foo"));
/// assert!(db1.is_connected());
///
/// // Inject a "bar" database address. The injector will return a database connected to "bar",
/// // and the first database will be disconnected.
/// injector.inject(Ok(Address("bar")));
/// # let db2 = timeout(TIMEOUT, async {
/// watch_db.changed().await?;
/// let db2 = watch_db.available().await?;
/// # Ok::<Database, ResolutionError>(db2)
/// # })
/// # .await??;
/// assert_eq!(db2.address(), &Address("bar"));
/// assert!(!db1.is_connected());
/// #
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct StateMap {
    states: RwLock<BTreeMap<TypeId, RawState>>,
}

impl Default for StateMap {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMap {
    /// Creates a new `StateMap`.
    pub const fn new() -> Self {
        Self {
            states: RwLock::new(BTreeMap::new()),
        }
    }

    /// Calls a closure on a state of the given type, creating a new state if one does not yet
    /// exists.
    pub fn with_state_by_type_id<F>(&self, type_id: TypeId, type_name: &'static str, f: F)
    where
        F: FnOnce(&RawState),
    {
        {
            // TODO: use non-poisoning alternative
            #[expect(clippy::missing_panics_doc)]
            let states = self.states.read().unwrap();
            if let Some(state) = states.get(&type_id) {
                f(state);
                return;
            }
        }

        // TODO: use non-poisoning alternative
        #[expect(clippy::missing_panics_doc)]
        let mut states = self.states.write().unwrap();
        // Some other thread might insert a state between the time read lock is released and the
        // write lock is acquired. If that's the case, use the existing state.
        if let Some(state) = states.get(&type_id) {
            f(state);
            return;
        }

        let state = RawState::new(type_id, type_name);
        f(&state);
        states.insert(type_id, state);
    }

    /// Calls a closure on a state of the given type, creating a new state if one does not yet
    /// exists.
    ///
    /// This method is the type-safe variant of
    /// [`with_state_by_type_id`](Self::with_state_by_type_id).
    pub fn with_state<T, F>(&self, f: F)
    where
        T: Clone + Send + Sync + 'static,
        F: FnOnce(StateRef<'_, T>),
    {
        self.with_state_by_type_id(TypeId::of::<T>(), type_name::<T>(), |raw| {
            f(StateRef::from_raw(raw));
        });
    }

    /// Calls a closure on a state of the given type and returns the watch to it, creating a new
    /// state if one does not yet exists.
    pub fn with_state_and_watch_by_type_id<F>(
        &self,
        type_id: TypeId,
        type_name: &'static str,
        f: F,
    ) -> RawWatch
    where
        F: FnOnce(&RawState),
    {
        {
            // TODO: use non-poisoning alternative
            #[expect(clippy::missing_panics_doc)]
            let states = self.states.read().unwrap();
            if let Some(state) = states.get(&type_id) {
                f(state);
                return state.watch();
            }
        }

        // TODO: use non-poisoning alternative
        #[expect(clippy::missing_panics_doc)]
        let mut states = self.states.write().unwrap();
        // Some other thread might insert a state between the time read lock is released and the
        // write lock is acquired. If that's the case, use the existing state.
        if let Some(state) = states.get(&type_id) {
            f(state);
            return state.watch();
        }

        let state = RawState::new(type_id, type_name);
        f(&state);
        let watch = state.watch();
        states.insert(type_id, state);
        watch
    }

    /// Calls a closure on a state of the given type and returns the watch to it, creating a new
    /// state if one does not yet exists.
    ///
    /// This method is the type-safe variant of
    /// [`with_state_and_watch_by_type_id`](Self::with_state_and_watch_by_type_id).
    pub fn with_state_and_watch<T, F>(&self, f: F) -> Watch<T>
    where
        T: Clone + Send + Sync + 'static,
        F: FnOnce(StateRef<'_, T>),
    {
        let raw =
            self.with_state_and_watch_by_type_id(TypeId::of::<T>(), type_name::<T>(), |raw| {
                f(StateRef::from_raw(raw));
            });

        Watch::from_raw(raw)
    }
}

impl Injector for StateMap {
    fn define_by_type_id(&self, type_id: TypeId, type_name: &'static str) {
        self.with_state_by_type_id(type_id, type_name, RawState::define);
    }

    fn inject_by_type_id(&self, type_id: TypeId, type_name: &'static str, value: Result<Erased>) {
        self.with_state_by_type_id(type_id, type_name, |state| state.inject(value));
    }

    fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch {
        self.with_state_and_watch_by_type_id(type_id, type_name, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use tokio::time::timeout;

    use crate::injector::InjectorExt;
    use crate::result::ResolutionError;

    use super::*;

    const TIMEOUT: Duration = Duration::from_millis(500);

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct Address(&'static str);

    #[derive(Clone, Debug)]
    struct Database(Arc<DatabaseInner>);

    #[derive(Debug)]
    struct DatabaseInner {
        address: Address,
        connected: AtomicBool,
    }

    impl Database {
        fn connect(address: Address) -> Self {
            Self(Arc::new(DatabaseInner {
                address,
                connected: AtomicBool::new(true),
            }))
        }

        fn address(&self) -> &Address {
            &self.0.address
        }

        fn disconnect(&self) {
            self.0.connected.store(false, Ordering::Relaxed);
        }

        fn is_connected(&self) -> bool {
            self.0.connected.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn test_inject_available() {
        let injector = Arc::new(StateMap::new());

        let mut watch_db = injector.watch::<Database>();

        timeout(TIMEOUT, async {
            let err = watch_db.available().await.unwrap_err();
            assert!(err.is_not_defined_for::<Database>());
        })
        .await
        .unwrap();

        let cloned = injector.clone();
        tokio::spawn(async move {
            let injector = cloned;

            injector.define::<Database>();
            let mut watch_address = injector.watch::<Address>();
            let mut current_db: Option<Database> = None;

            loop {
                match watch_address.available().await {
                    Ok(address) => {
                        let db = Database::connect(address);

                        if let Some(db) = current_db.take() {
                            db.disconnect();
                        }
                        current_db = Some(db.clone());

                        injector.inject(Ok(db));
                    }
                    Err(err) => injector.inject::<Database>(Err(err)),
                }

                watch_address.changed().await.unwrap();
            }
        });

        injector.inject(Ok(Address("foo")));
        let db1 = timeout(TIMEOUT, async {
            watch_db.changed().await.unwrap();
            watch_db.available().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(db1.address(), &Address("foo"));
        assert!(db1.is_connected());

        injector.inject(Ok(Address("bar")));
        let db2 = timeout(TIMEOUT, async {
            watch_db.changed().await.unwrap();
            watch_db.available().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(db2.address(), &Address("bar"));
        assert!(!db1.is_connected());

        injector.inject::<Address>(Err(ResolutionError::other("something went wrong")));
        let err = timeout(TIMEOUT, async {
            watch_db.changed().await.unwrap();
            watch_db.available().await.unwrap_err()
        })
        .await
        .unwrap();
        assert!(err.is_other());
    }
}
