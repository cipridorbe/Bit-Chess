use crate::{repr::bitboard::BB, test_assert};

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
    /// Converts the given u8 to a Square
    pub fn from_u8(square: u8) -> Self {
        test_assert!(square < 64);
        unsafe { std::mem::transmute(square) }
    }

    pub fn from_rank_file(rank: u8, file: u8) -> Self {
        test_assert!(rank < 8 && file < 8);
        Square::from_u8(rank * 8 + file)
    }

    /// Returns a bitboard with `self` set
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

    pub fn to_fen(self) -> String {
        let files = ["a", "b", "c", "d", "e", "f", "g", "h"];
        let ranks = ["1", "2", "3", "4", "5", "6", "7", "8"];
        let file = self.file();
        let rank = self.rank();
        format!("{}{}", files[file as usize], ranks[rank as usize])
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        if fen == "-" {
            return None;
        }
        let chars: Vec<char> = fen.chars().collect();
        if chars.len() != 2 {
            panic!("Invalid fen for square: {}", fen);
        }
        let file = chars[0] as u8 - 'a' as u8;
        let rank = chars[1] as u8 - '1' as u8;
        Some(Square::from_rank_file(rank, file))
    }
}