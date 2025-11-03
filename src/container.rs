use std::sync::Arc;

use crate::component::{
    AsyncConstructor, AsyncConstructorTask, Composite, Constructor, ConstructorTask,
};
use crate::injector::{Injector, InjectorTask, InjectorTaskObject, StateMap};
use crate::runtime::Runtime;

pub struct SimpleContainer<R, I = Arc<StateMap>> {
    #[expect(dead_code)]
    rt: R,
    injector: I,
}

pub struct SimpleContainerBuilder<R, I> {
    rt: R,
    injector: I,
    tasks: Vec<InjectorTaskObject<I>>,
}

impl<R, I> SimpleContainer<R, I>
where
    I: Default,
{
    #[must_use]
    pub fn builder(rt: R) -> SimpleContainerBuilder<R, I> {
        SimpleContainerBuilder {
            rt,
            injector: I::default(),
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
    pub fn register_task<T>(mut self, task: T) -> Self
    where
        T: InjectorTask<I> + Send + 'static,
    {
        self.tasks.push(InjectorTaskObject::new(task));
        self
    }

    #[must_use]
    pub fn register_constructor<C, T>(mut self, constructor: C) -> Self
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
    pub fn register_async_constructor<C, T>(mut self, constructor: C) -> Self
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
            rt.spawn(async move { task.run(&cloned).await });
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

    use crate::component::Component;
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
    async fn test_inject_db() {
        fn connect_db(address: Component<Address>) -> Component<Database> {
            Component(Database::connect(address.0))
        }

        timeout(TIMEOUT, async {
            let container: SimpleContainer<TokioRuntime, Arc<StateMap>> =
                SimpleContainer::builder(TokioRuntime::new())
                    .register_constructor(|| Component(Address("foo")))
                    .register_constructor(connect_db)
                    .build();

            let mut watch_db = container.watch::<Database>();
            let db = watch_db.wait_always().await.unwrap();
            assert_eq!(db.address(), &Address("foo"));
        })
        .await
        .unwrap();
    }
}
