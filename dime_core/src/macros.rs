//! Internal macros.

#![allow(unused_macros)]

#[rustfmt::skip]
macro_rules! apply_tuples {
    ($name:ident) => {
        $name!(T1);
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
    };
}

pub trait TryFuture: Future {
    type Ok;

    type Err;

    fn try_poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Self::Ok, Self::Err>>;
}

impl<T, E, F: Future<Output = Result<T, E>>> TryFuture for F {
    type Ok = T;

    type Err = E;

    #[inline]
    fn try_poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Self::Ok, Self::Err>> {
        self.poll(cx)
    }
}

pin_project_lite::pin_project! {
    #[project = TryMaybeDoneProj]
    #[project_replace = TryMaybeDoneReplaceProj]
    pub enum TryMaybeDone<F: TryFuture> {
        Future{ #[pin] future: F },
        Done{ output: F::Ok },
        Gone,
    }
}

impl<F: TryFuture> TryMaybeDone<F> {
    pub const fn new(future: F) -> Self {
        Self::Future { future }
    }

    pub fn take_output(self: std::pin::Pin<&mut Self>) -> Option<F::Ok> {
        match *self {
            Self::Done { .. } => {}
            _ => return None,
        }

        if let TryMaybeDoneReplaceProj::Done { output } = self.project_replace(Self::Gone) {
            Some(output)
        } else {
            unreachable!()
        }
    }
}

impl<F: TryFuture> Future for TryMaybeDone<F> {
    type Output = Result<(), F::Err>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().project() {
            TryMaybeDoneProj::Future { future } => match std::task::ready!(future.try_poll(cx)) {
                Ok(output) => self.set(Self::Done { output }),
                Err(err) => return std::task::Poll::Ready(Err(err)),
            },
            TryMaybeDoneProj::Done { .. } => {}
            TryMaybeDoneProj::Gone => unreachable!(),
        }

        std::task::Poll::Ready(Ok(()))
    }
}
