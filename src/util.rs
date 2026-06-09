/// Asserts a condition when the `assertions` feature is enabled; compiles to nothing otherwise.
/// Enable with: cargo build --features assertions
#[macro_export]
macro_rules! test_assert {
    ($cond:expr) => {
        #[cfg(feature = "assertions")]
        assert!($cond);
    };
    ($cond:expr, $($arg:tt)+) => {
        #[cfg(feature = "assertions")]
        assert!($cond, $($arg)+);
    };
}