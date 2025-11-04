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

    /// Waits until a result value of type `T` is available regardless if the injector is promised
    /// such value.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn wait_always(&mut self) -> impl Future<Output = Result<Self::Ty>> + Send;

    /// Waits until a value of type `T` is successfully created (e.g. by injecting `Ok(value)`
    /// to the injector), regardless if the injector is promised such value.
    ///
    /// # Errors
    ///
    /// While this method ensures that the value stored in the injector is of `Ok` variant, it is
    /// still necessary to account for another unexpected error caused by the internals of the
    /// injector itself.
    fn wait_ok(&mut self) -> impl Future<Output = Result<Self::Ty>> + Send;

    /// Waits until the value of type `T` changes.
    ///
    /// # Errors
    ///
    /// This method returns [`ResolutionError`](crate::result::ResolutionError) if the evaluation
    /// of the value returned an error.
    fn changed(&mut self) -> impl Future<Output = Result<()>> + Send;
}

// We can produce `()` out of thin air.
impl Watch for () {
    type Ty = ();

    fn current(&self) -> Result<Self::Ty> {
        Ok(())
    }

    fn current_optional(&self) -> Result<Option<Self::Ty>> {
        Ok(Some(()))
    }

    async fn wait(&mut self) -> Result<Self::Ty> {
        Ok(())
    }

    async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
        Ok(Some(()))
    }

    async fn wait_always(&mut self) -> Result<Self::Ty> {
        Ok(())
    }

    async fn wait_ok(&mut self) -> Result<Self::Ty> {
        Ok(())
    }

    fn changed(&mut self) -> impl Future<Output = Result<()>> + Send {
        std::future::pending()
    }
}

macro_rules! impl_watch_tuple {
    ($($ty:ident),*) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        const _: () = {
            fn unwrap_option_tuple<$($ty,)*>($($ty: Option<$ty>,)*) -> Option<($($ty,)*)> {
                Some(($($ty?,)*))
            }

            def_try_join_ty_fn!($($ty),*);

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
                    let ($($ty,)*) = self;
                    let ($($ty,)*) = ($($ty.current_optional()?,)*);
                    Ok(unwrap_option_tuple($($ty,)*))
                }

                async fn wait(&mut self) -> Result<Self::Ty> {
                    let ($($ty,)*) = self;
                    let ($($ty,)*) = ($($ty.wait(),)*);
                    try_join_ty($($ty),*).await
                }

                async fn wait_optional(&mut self) -> Result<Option<Self::Ty>> {
                    let ($($ty,)*) = self;
                    let ($($ty,)*) = ($($ty.wait_optional(),)*);
                    let ($($ty,)*) = try_join_ty($($ty),*).await?;
                    Ok(unwrap_option_tuple($($ty,)*))
                }

                async fn wait_always(&mut self) -> Result<Self::Ty> {
                    let ($($ty,)*) = self;
                    let ($($ty,)*) = ($($ty.wait_always(),)*);
                    try_join_ty($($ty),*).await
                }

                async fn wait_ok(&mut self) -> Result<Self::Ty> {
                    let ($($ty,)*) = self;
                    let ($($ty,)*) = ($($ty.wait_ok(),)*);
                    try_join_ty($($ty),*).await
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
    };
}

macro_rules! def_try_join_ty_fn {
    ($($ty:ident),*) => {
        async fn try_join_ty<$($ty,)* E>($($ty: $ty,)*) -> Result<($($ty::Ok,)*), E>
        where
            $($ty: $crate::macros::TryFuture<Err = E>,)*
        {
            use std::pin::pin;
            use std::task::Poll;
            use $crate::macros::TryFuture;

            let ($($ty,)*) = ($($crate::macros::TryMaybeDone::new($ty),)*);
            let ($(mut $ty,)*) = ($(pin!($ty),)*);

            std::future::poll_fn(|cx| {
                let mut done = true;

                $(
                    match $ty.as_mut().try_poll(cx) {
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                        poll => done &= poll.is_ready(),
                    }
                )*

                if done {
                    Poll::Ready(Ok((
                        $({
                            $ty.as_mut()
                                .take_output()
                                .expect("expected completed future")
                        },)*
                    )))
                } else {
                    Poll::Pending
                }
            }).await
        }
    };
}

apply_tuples!(impl_watch_tuple);
