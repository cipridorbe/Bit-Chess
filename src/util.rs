use std::ops::{Index, IndexMut};

use crate::repr::{bitboard::BB, colour::Colour, piece::{Piece, PieceType}, square::Square};

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

impl<T: Copy> Index<Square> for [T; 64] {
    type Output = T;
    fn index(&self, index: Square) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T: Copy> IndexMut<Square> for [T; 64] {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl<T: Copy> Index<Piece> for [T; 12] {
    type Output = T;
    fn index(&self, index: Piece) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T: Copy> IndexMut<Piece> for [T; 12] {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl<T: Copy> Index<Colour> for [T; 2] {
    type Output = T;
    fn index(&self, index: Colour) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T: Copy> IndexMut<Colour> for [T; 2] {
    fn index_mut(&mut self, index: Colour) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl<T: Copy> Index<PieceType> for [T; 2] {
    type Output = T;
    fn index(&self, index: PieceType) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T: Copy> IndexMut<PieceType> for [T; 2] {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        &mut self[index as usize]
    }
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