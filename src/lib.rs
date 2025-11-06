//! Type-system-powered asynchronous dependency injection library.
//!
//! # Example
//! With the following type definitions (note that you don't need to use `dime` types, functions,
//! or macros here):
//!
//! ```no_run
//! use std::error::Error;
//! use std::fmt::Debug;
//! use std::sync::Arc;
//! # use std::sync::atomic::{AtomicBool, Ordering};
//!
//! use tokio::sync::mpsc;
//!
//! #[derive(Clone, Debug, Default, PartialEq, Eq)]
//! struct Address(&'static str);
//!
//! #[derive(Clone, Debug)]
//! struct Item {
//!     // ...
//! #     pub id: u32,
//! #     pub name: String,
//! }
//!
//! trait Database: Debug + Send + Sync + 'static {
//!     fn get(&self, id: u32) -> Result<Item, Box<dyn Error + Send + Sync>>;
//! }
//!
//! #[derive(Debug)]
//! struct DatabaseImpl {
//!     // ...
//! #     _address: Address,
//! #     connected: AtomicBool,
//! #     items: Vec<Item>,
//! }
//!
//! impl DatabaseImpl {
//! #     #[expect(clippy::unused_async)]
//!     async fn connect(address: Address) -> Self {
//!         // ...
//! #        Self {
//! #             _address: address,
//! #             connected: AtomicBool::new(true),
//! #             items: vec![
//! #                 Item {
//! #                     id: 0,
//! #                     name: "Item 1".to_string(),
//! #                 },
//! #                 Item {
//! #                     id: 1,
//! #                     name: "Item 2".to_string(),
//! #                 },
//! #                 Item {
//! #                     id: 2,
//! #                     name: "Item 3".to_string(),
//! #                 },
//! #             ],
//! #         }
//!     }
//!
//!     fn is_connected(&self) -> bool {
//!         // ...
//! #         self.connected.load(Ordering::Relaxed)
//!     }
//! }
//!
//! #[derive(Clone, Debug)]
//! struct Logger(mpsc::UnboundedSender<String>);
//!
//! impl Logger {
//!     fn log(&self, s: String) {
//!         // ...
//! #         let _ = self.0.send(s);
//!     }
//! }
//!
//! impl Database for DatabaseImpl {
//!     fn get(&self, id: u32) -> Result<Item, Box<dyn Error + Send + Sync>> {
//!         // ...
//! #         if self.is_connected() {
//! #             self.items
//! #                 .get(id as usize)
//! #                 .cloned()
//! #                 .ok_or_else(|| "item not found".into())
//! #         } else {
//! #             Err("database disconnected".into())
//! #         }
//!     }
//! }
//!
//! #[derive(Clone, Debug)]
//! struct Service {
//!     // ...
//! #     database: Arc<dyn Database>,
//! #     logger: Logger,
//! }
//!
//! impl Service {
//!     fn new(database: Arc<dyn Database>, logger: Logger) -> Self {
//!         // ...
//! #         Self { database, logger }
//!     }
//!
//! #     #[expect(clippy::unused_async)]
//!     async fn call(&self, id: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
//!         // ...
//! #         let item = self.database.get(id)?;
//! #         self.logger
//! #             .log(format!("got item id {}, name `{}`", item.id, &item.name));
//! #         Ok(())
//!     }
//! }
//!
//! #[derive(Clone, Debug)]
//! struct Application {
//!     // ...
//! #     service: Service,
//! #     logger: Logger,
//! }
//!
//! impl Application {
//!     fn new(service: Service, logger: Logger) -> Self {
//!        // ...
//! #         Self { service, logger }
//!     }
//!
//!     async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
//!        // ...
//! #        self.logger.log("starting application".to_string());
//! #
//! #        for id in 0..3 {
//! #            self.service.call(id).await?;
//! #        }
//! #
//! #        Ok(())
//!    }
//! }
//! ```
//!
//! You can structure your application around [`SimpleContainer`](crate::container::SimpleContainer) like this:
//!
//! ```
//! # use std::error::Error;
//! # use std::fmt::Debug;
//! use std::sync::Arc;
//! # use std::sync::atomic::{AtomicBool, Ordering};
//!
//! use tokio::sync::mpsc;
//!
//! use dime::component::{Component as C, WaitAlways};
//! use dime::container::SimpleContainer;
//! use dime::injector::StateMap;
//! use dime::tokio::TokioRuntime;
//!
//! # #[derive(Clone, Debug, Default, PartialEq, Eq)]
//! # struct Address(&'static str);
//! #
//! # #[derive(Clone, Debug)]
//! # struct Item {
//! #     // ...
//! #     pub id: u32,
//! #     pub name: String,
//! # }
//! #
//! # trait Database: Debug + Send + Sync + 'static {
//! #     fn get(&self, id: u32) -> Result<Item, Box<dyn Error + Send + Sync>>;
//! # }
//! #
//! # #[derive(Debug)]
//! # struct DatabaseImpl {
//! #     // ...
//! #     _address: Address,
//! #     connected: AtomicBool,
//! #     items: Vec<Item>,
//! # }
//! #
//! # impl DatabaseImpl {
//! #     #[expect(clippy::unused_async)]
//! #     async fn connect(address: Address) -> Self {
//! #         // ...
//! #         Self {
//! #             _address: address,
//! #             connected: AtomicBool::new(true),
//! #             items: vec![
//! #                 Item {
//! #                     id: 0,
//! #                     name: "Item 1".to_string(),
//! #                 },
//! #                 Item {
//! #                     id: 1,
//! #                     name: "Item 2".to_string(),
//! #                 },
//! #                 Item {
//! #                     id: 2,
//! #                     name: "Item 3".to_string(),
//! #                 },
//! #             ],
//! #         }
//! #     }
//! #
//! #     fn is_connected(&self) -> bool {
//! #         // ...
//! #         self.connected.load(Ordering::Relaxed)
//! #     }
//! # }
//! #
//! # #[derive(Clone, Debug)]
//! # struct Logger(mpsc::UnboundedSender<String>);
//! #
//! # impl Logger {
//! #     fn log(&self, s: String) {
//! #         // ...
//! #         let _ = self.0.send(s);
//! #     }
//! # }
//! #
//! # impl Database for DatabaseImpl {
//! #     fn get(&self, id: u32) -> Result<Item, Box<dyn Error + Send + Sync>> {
//! #         // ...
//! #         if self.is_connected() {
//! #             self.items
//! #                 .get(id as usize)
//! #                 .cloned()
//! #                 .ok_or_else(|| "item not found".into())
//! #         } else {
//! #             Err("database disconnected".into())
//! #         }
//! #     }
//! # }
//! #
//! # #[derive(Clone, Debug)]
//! # struct Service {
//! #     // ...
//! #     database: Arc<dyn Database>,
//! #     logger: Logger,
//! # }
//! #
//! # impl Service {
//! #     fn new(database: Arc<dyn Database>, logger: Logger) -> Self {
//! #         // ...
//! #         Self { database, logger }
//! #     }
//! #
//! #     #[expect(clippy::unused_async)]
//! #     async fn call(&self, id: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
//! #         // ...
//! #         let item = self.database.get(id)?;
//! #         self.logger
//! #             .log(format!("got item id {}, name `{}`", item.id, &item.name));
//! #         Ok(())
//! #     }
//! # }
//! #
//! # #[derive(Clone, Debug)]
//! # struct Application {
//! #     // ...
//! #     service: Service,
//! #     logger: Logger,
//! # }
//! #
//! # impl Application {
//! #     fn new(service: Service, logger: Logger) -> Self {
//! #        // ...
//! #         Self { service, logger }
//! #     }
//! #
//! #     async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
//! #        // ...
//! #        self.logger.log("starting application".to_string());
//! #
//! #        for id in 0..3 {
//! #            self.service.call(id).await?;
//! #        }
//! #
//! #        Ok(())
//! #    }
//! # }
//! #
//! # #[tokio::main]
//! # async fn main() {
//! # async fn inner() -> Result<(), Box<dyn Error + Send + Sync>> {
//! let (tx, mut rx) = mpsc::unbounded_channel::<String>();
//!
//! let container = SimpleContainer::builder(TokioRuntime::new())
//!     // You can provide the container with already-made components...
//!     .with_component(Logger(tx))
//!     .with_component(Address("foo"))
//!     // ... or a constructor function that takes components and produces another
//!     // components... (a component can be an `Arc` or something wrapped in `Component`)
//!     .with_constructor(|db: Arc<dyn Database>, C(logger): C<Logger>| {
//!         C(Service::new(db, logger))
//!     })
//!     // ... or an async constructor function...
//!     .with_async_constructor(async |C(address): C<Address>| {
//!         Arc::new(DatabaseImpl::connect(address).await) as Arc<dyn Database>
//!     })
//!     // ... or you can write custom code around the inner injector using `InjectorTask`!
//!     .with_task(async |injector: Arc<StateMap>| {
//!         use dime::injector::{Injector, Watch};
//!
//!         injector.define::<Application>();
//!         let mut watch_service = injector.watch::<Service>();
//!         let mut watch_logger = injector.watch::<Logger>();
//!
//!         loop {
//!             let app = tokio::try_join!(watch_service.wait(), watch_logger.wait())
//!                 .map(|(service, logger)| Application::new(service, logger));
//!             injector.inject(app);
//!
//!             tokio::select! {
//!                 res = watch_service.changed() => res,
//!                 res = watch_logger.changed() => res,
//!             }?;
//!         }
//!     })
//!     .build();
//!
//! // Call a function with a `Application` as argument, and the injector shall provide the
//! // `Application` created by our constructors.
//! container
//!     .call_async(async |WaitAlways(C(app)): WaitAlways<C<Application>>| app.run().await)
//!     .await??;
//! #
//! # let mut buf = Vec::<String>::with_capacity(4);
//! # assert_eq!(rx.recv_many(&mut buf, 4).await, 4);
//! # assert_eq!(
//! #     buf,
//! #     vec![
//! #         "starting application",
//! #         "got item id 0, name `Item 1`",
//! #         "got item id 1, name `Item 2`",
//! #         "got item id 2, name `Item 3`",
//! #     ]
//! # );
//! #
//! # Ok(())
//! # }
//! #
//! # tokio::time::timeout(std::time::Duration::from_millis(1000), inner())
//! #     .await
//! #     .unwrap()
//! #     .unwrap();
//! # }
//! ```
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::must_use_candidate)]

#[macro_use]
pub(crate) mod macros;

#[doc(inline)]
pub use dime_core::{Erased, Error, Injector, Result, Runtime, erased, error, runtime};

pub mod component;
pub mod container;
pub mod injector;

#[cfg(any(feature = "tokio", test))]
pub mod tokio;
