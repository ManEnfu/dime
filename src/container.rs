use std::sync::Arc;

use crate::component::{
    AsyncConstructor, AsyncConstructorTask, Component, Composite, Constructor, ConstructorTask,
};
use crate::injector::{Injector, InjectorTask, InjectorTaskObject, StateMap};
use crate::runtime::Runtime;

pub struct SimpleContainer<R, I = Arc<StateMap>> {
    #[expect(dead_code)]
    rt: R,
    injector: I,
}

pub struct SimpleContainerBuilder<R, I = Arc<StateMap>> {
    rt: R,
    injector: I,
    tasks: Vec<InjectorTaskObject<I>>,
}

impl<R> SimpleContainer<R> {
    #[must_use]
    pub fn builder(rt: R) -> SimpleContainerBuilder<R> {
        SimpleContainerBuilder {
            rt,
            injector: Arc::default(),
            tasks: Vec::new(),
        }
    }
}

impl<R, I> SimpleContainerBuilder<R, I>
where
    R: Runtime,
    I: Injector + Clone + Send + 'static,
{
    #[must_use]
    pub fn with_task<T>(mut self, task: T) -> Self
    where
        T: InjectorTask<I> + Send + 'static,
    {
        self.tasks.push(InjectorTaskObject::new(task));
        self
    }

    #[must_use]
    pub fn with_component<T>(self, component: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
        I::Watch<T>: Send,
    {
        self.with_constructor(|| Component(component))
    }

    #[must_use]
    pub fn with_constructor<C, T>(mut self, constructor: C) -> Self
    where
        T: Composite<I> + Send + 'static,
        T::Watch: Send + 'static,
        C: Constructor<T> + Clone + Send + Sync + 'static,
        C::Constructed: Composite<I>,
    {
        let task = ConstructorTask::new(constructor);
        self.tasks.push(InjectorTaskObject::from_boxed_future(task));
        self
    }

    #[must_use]
    pub fn with_async_constructor<C, T>(mut self, constructor: C) -> Self
    where
        T: Composite<I> + Send + 'static,
        T::Watch: Send + 'static,
        C: AsyncConstructor<T> + Clone + Send + Sync + 'static,
        C::Constructed: Composite<I>,
        C::Future: Send,
    {
        let task = AsyncConstructorTask::new(constructor);
        self.tasks.push(InjectorTaskObject::from_boxed_future(task));
        self
    }

    #[must_use]
    pub fn build(self) -> SimpleContainer<R, I> {
        let Self {
            rt,
            injector,
            tasks,
        } = self;

        for task in tasks {
            let cloned = injector.clone();
            rt.spawn(async move { task.run(cloned).await });
        }

        SimpleContainer { rt, injector }
    }
}

impl<R, I> SimpleContainer<R, I>
where
    R: Runtime,
    I: Injector,
{
    pub fn watch<T>(&self) -> I::Watch<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.injector.watch()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use tokio::time::timeout;

    use crate::component::{Component, Current};
    use crate::runtime::TokioRuntime;

    use crate::injector::Watch;

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
    async fn test_db_constructor() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Address>(2);

        let container = SimpleContainer::builder(TokioRuntime::new())
            .with_task(async move |injector: Arc<StateMap>| {
                injector.define::<Address>();
                loop {
                    if let Some(address) = rx.recv().await {
                        injector.inject(Ok(address));
                    }
                }
            })
            .with_constructor(
                |Component(address): Component<Address>,
                 Current(old_db): Current<Option<Component<Database>>>| {
                    if let Some(Component(db)) = old_db {
                        db.disconnect();
                    }

                    Component(Database::connect(address))
                },
            )
            .build();

        let mut watch_db = container.watch::<Database>();

        tx.send(Address("foo")).await.unwrap();
        let db1 = timeout(TIMEOUT, async { watch_db.wait_always().await.unwrap() })
            .await
            .unwrap();
        assert_eq!(db1.address(), &Address("foo"));
        assert!(db1.is_connected());

        tx.send(Address("bar")).await.unwrap();
        let db2 = timeout(TIMEOUT, async {
            watch_db.changed().await.unwrap();
            watch_db.wait_always().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(db2.address(), &Address("bar"));
        assert!(!db1.is_connected());
    }
}
