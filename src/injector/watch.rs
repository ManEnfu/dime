#![allow(clippy::missing_errors_doc)]

use crate::result::Result;

/// Watches for values of a given type in [`Injector`](crate::injector::Injector).
pub trait Watch {
    /// The type of the value watched by the watch.
    type Ty;

    /// Immediately retrieves the current value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn current(&self) -> Result<Self::Ty>;

    /// Immediately retrieves the current value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn current_optional(&self) -> Result<Option<Self::Ty>>;

    /// Waits until a value of type `T` is available if the injector is promised such value.
    ///
    /// # Errors
    ///
    /// This method returns
    /// [`ResolutionError::NotDefined`](crate::result::ResolutionError::NotDefined)
    /// if no value of type `T` is promised to the injector.
    /// Otherwise, this method returns [`ResolutionError`](crate::result::ResolutionError) if the
    /// evaluation of the value returned an error.
    fn wait(&mut self) -> impl Future<Output = Result<Self::Ty>> + Send;

    /// Waits until a value of type `T` is available if the injector is promised such value,
    /// returning `None` otherwise.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn wait_optional(&mut self) -> impl Future<Output = Result<Option<Self::Ty>>> + Send;

    /// Waits until a value of type `T` is available regardless if the injector is promised such
    /// value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn wait_always(&mut self) -> impl Future<Output = Result<Self::Ty>> + Send;

    /// Waits until the value of type `T` changes.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn changed(&mut self) -> impl Future<Output = Result<()>> + Send;
}

macro_rules! impl_watch_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        impl<$($ty,)*> Watch for ($($ty,)*)
        where
            $($ty: Watch + Send,)*
            $($ty::Ty: Send,)*
        {
            type Ty = ($($ty::Ty,)*);

            fn current(&self) -> Result<Self::Ty> {
                let ($($ty,)*) = self;
                let ($($ty,)*) = ($($ty.current()?,)*);
                Ok(($($ty,)*))
            }

            fn current_optional(&self) -> Result<Option<Self::Ty>> {
                fn unwrap_option_tuple<$($ty,)*>($($ty: Option<$ty>,)*) -> Option<($($ty,)*)> {
                    Some(($($ty?,)*))
                }

                let ($($ty,)*) = self;
                let ($($ty,)*) = ($($ty.current_optional()?,)*);
                Ok(unwrap_option_tuple($($ty,)*))
            }

            async fn wait(&mut self) -> Result<Self::Ty> {
                let ($($ty,)*) = self;
                tokio::try_join!($($ty.wait(),)*)
            }

            async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
                fn unwrap_option_tuple<$($ty,)*>($($ty: Option<$ty>,)*) -> Option<($($ty,)*)> {
                    Some(($($ty?,)*))
                }

                let ($($ty,)*) = self;
                let ($($ty,)*) = tokio::try_join!($($ty.wait_optional(),)*)?;
                Ok(unwrap_option_tuple($($ty,)*))
            }

            async fn wait_always(&mut self) -> Result<Self::Ty> {
                let ($($ty,)*) = self;
                tokio::try_join!($($ty.wait_always(),)*)
            }

            async fn changed(&mut self) -> Result<()> {
                use std::pin::pin;
                use std::task::Poll;

                let ($($ty,)*) = self;
                let ($($ty,)*) = ($($ty.changed(),)*);
                let ($(mut $ty,)*) = ($(pin!($ty),)*);

                std::future::poll_fn(|cx| {
                    $(
                        if let Poll::Ready(res) = $ty.as_mut().poll(cx) {
                            return Poll::Ready(res);
                        }
                    )*
                    Poll::Pending
                }).await
            }
        }
    };
}

impl_watch_tuple!(T1);
impl_watch_tuple!(T1, T2);
impl_watch_tuple!(T1, T2, T3);
impl_watch_tuple!(T1, T2, T3, T4);
impl_watch_tuple!(T1, T2, T3, T4, T5);
impl_watch_tuple!(T1, T2, T3, T4, T5, T6);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
// impl_watch_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
// impl_watch_tuple!(
//     T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
// );
// impl_watch_tuple!(
//     T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
// );
