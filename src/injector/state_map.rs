use std::any::TypeId;
use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::erased::Erased;
use crate::injector::Injector;
use crate::injector::state::RawState;
use crate::result::Result;

use super::state::RawWatch;

/// A Simple injector backed by [`BTreeMap`].
#[derive(Debug, Default)]
pub struct StateMap {
    states: RwLock<BTreeMap<TypeId, RawState>>,
}

impl StateMap {
    /// Creates a new `StateMap`.
    pub const fn new() -> Self {
        Self {
            states: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn with_state<F>(&self, type_id: TypeId, type_name: &'static str, f: F)
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

        let state = RawState::new_undefined(type_id, type_name);
        f(&state);
        states.insert(type_id, state);
    }

    pub fn with_state_and_watch<F>(
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

        let state = RawState::new_undefined(type_id, type_name);
        f(&state);
        let watch = state.watch();
        states.insert(type_id, state);
        watch
    }
}

impl Injector for StateMap {
    fn define_by_type_id(&self, type_id: TypeId, type_name: &'static str) {
        self.with_state(type_id, type_name, RawState::define);
    }

    fn inject_by_type_id(&self, type_id: TypeId, type_name: &'static str, value: Result<Erased>) {
        self.with_state(type_id, type_name, |state| state.inject(value));
    }

    fn raw_watch_by_type_id(&self, type_id: TypeId, type_name: &'static str) -> RawWatch {
        self.with_state_and_watch(type_id, type_name, |_| {})
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{injector::InjectorExt, result::ResolutionError};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct Double(pub i32);

    impl Double {
        fn new(num: i32) -> Self {
            Self(num * 2)
        }
    }

    #[tokio::test]
    async fn test_inject() {
        let injector = Arc::new(StateMap::new());
        let cloned_injector = injector.clone();

        tokio::spawn(async move {
            let injector = cloned_injector;

            injector.define::<Double>();
            let mut watch_i32 = injector.watch::<i32>();

            loop {
                let double = watch_i32.available().await.map(Double::new);
                injector.inject(double);

                watch_i32.changed().await.unwrap();
            }
        });

        let mut watch_double = injector.watch::<Double>();

        injector.inject(Ok(32_i32));
        watch_double.changed().await.unwrap();
        assert_eq!(watch_double.available().await.unwrap(), Double(64));

        injector.inject::<i32>(Err(ResolutionError::not_defined::<i32>()));
        watch_double.changed().await.unwrap();
        assert!(
            watch_double
                .available()
                .await
                .unwrap_err()
                .is_not_defined_for::<i32>()
        );

        injector.inject(Ok(90_i32));
        watch_double.changed().await.unwrap();
        assert_eq!(watch_double.available().await.unwrap(), Double(180));

        injector.inject::<i32>(Err(ResolutionError::other(std::fmt::Error)));
        watch_double.changed().await.unwrap();
        assert!(watch_double.available().await.unwrap_err().is_other());
    }
}
