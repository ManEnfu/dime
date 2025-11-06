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

macro_rules! error {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            {
                ::tracing::error!($($tt)*)
            }
        }
    };
}

macro_rules! warn {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            {
                ::tracing::warn!($($tt)*)
            }
        }
    };
}

macro_rules! info {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            {
                ::tracing::info!($($tt)*)
            }
        }
    };
}

macro_rules! debug {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            {
                ::tracing::debug!($($tt)*)
            }
        }
    };
}

macro_rules! trace {
    ($($tt:tt)*) => {
        {
            #[cfg(feature = "tracing")]
            {
                ::tracing::trace!($($tt)*)
            }
        }
    };
}
