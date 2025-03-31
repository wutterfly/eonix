/// A macro to unwrap or unwrap_unchecked, based on compile flags.
///
/// Can be used to optimized "trivial" runtime checks, that *should* always be true.
macro_rules! unwrap {
    ($expression:expr) => {{
        if cfg!(feature = "runtime-checks") {
            $expression.unwrap()
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                $expression.unwrap_unchecked()
            }
        }
    }};
}

pub(crate) use unwrap;
