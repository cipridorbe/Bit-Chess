use crate::repr::bitboard::BB;

/// Asserts a condition when the `assertions` feature is enabled; compiles to nothing otherwise.
/// Enable with: cargo build --features assertions

pub fn populate_files(bb: BB) -> BB {
    populate_files_up(bb) | populate_files_down(bb)
}

pub fn populate_files_up(mut bb: BB) -> BB {
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;
    bb
}

pub fn populate_files_down(mut bb: BB) -> BB {
    bb |= bb >> 8;
    bb |= bb >> 16;
    bb |= bb >> 32;
    bb
}

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