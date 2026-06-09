use crate::repr::bitboard::BB;

#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Square {
    a1, b1, c1, d1, e1, f1, g1, h1,
    a2, b2, c2, d2, e2, f2, g2, h2,
    a3, b3, c3, d3, e3, f3, g3, h3,
    a4, b4, c4, d4, e4, f4, g4, h4,
    a5, b5, c5, d5, e5, f5, g5, h5,
    a6, b6, c6, d6, e6, f6, g6, h6,
    a7, b7, c7, d7, e7, f7, g7, h7,
    a8, b8, c8, d8, e8, f8, g8, h8,
}

impl Square {
    pub fn bb(self) -> BB {
        BB::new(1 << self as u8)
    }

    /// Returns the 0-indexed rank
    pub fn rank(self) -> u8 {
        self as u8 / 8
    }

    /// Returns the 0-indexed file, where file A is the 0'th file
    pub fn file(self) -> u8 {
        self as u8 % 8
    }
}