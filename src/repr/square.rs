use once_cell::sync::Lazy;

use crate::{repr::bitboard::{BB, BBIter}, test_assert};

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
    pub const DARK_SQUARES: BB = const {
        let mut out = 1;
        out |= out << 2;
        out |= out << 4;
        out |= out << 9;
        out |= out << 16;
        out |= out << 32;
        BB::new(out)
    };

    pub const LIGHT_SQUARES: BB = BB::new(!Square::DARK_SQUARES.0);

    /// Iterator over all squares
    pub fn all() -> BBIter {
        BBIter(!BB::new(0)).into_iter()
    }

    /// Converts the given u8 to a Square
    pub const fn from_u8(square: u8) -> Self {
        test_assert!(square < 64);
        unsafe { std::mem::transmute(square) }
    }

    pub const fn from_rank_file(rank: u8, file: u8) -> Self {
        test_assert!(rank < 8 && file < 8);
        Square::from_u8(rank * 8 + file)
    }

    /// Returns a bitboard with `self` set
    pub fn bb(self) -> BB {
        BB::new(1 << self as u8)
    }

    /// Returns the 0-indexed rank
    pub const fn rank(self) -> u8 {
        self as u8 / 8
    }

    /// Returns the 0-indexed file, where file A is the 0'th file
    pub const fn file(self) -> u8 {
        self as u8 % 8
    }

    /// Returns the rank and file, 0-indexed
    pub const fn rank_file(self) -> (u8, u8) {
        (self.rank(), self.file())
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

/// [Included][Excluded]
pub static SEGMENT: Lazy<[[BB; 64]; 64]> = Lazy::new(|| {
    let mut table = [[BB::new(0); 64]; 64];
    for sq1 in Square::all() {
        for sq2 in Square::all() {
            let (r1, f1) = sq1.rank_file();
            let (r2, f2) = sq2.rank_file();
            let dr = r1 as i8 - r2 as i8;
            let df = f1 as i8 - f2 as i8;
            if !(dr == 0 || df == 0 || dr.abs() == df.abs()) {
                continue;
            }
            let mut segment = BB::new(0);
            let (mut r, mut f) = (r2 as i8, f2 as i8);
            while !(r as u8 == r1 && f as u8 == f1) {
                let sq = Square::from_rank_file(r as u8, f as u8);
                segment |= sq;
                r += dr.signum();
                f += df.signum();
            }
            segment |= sq1.bb();
            segment &= !sq2.bb();
            table[sq1 as usize][sq2 as usize] = segment;
        }
    }
    table
});

pub static SEGMENT_CARDINAL: Lazy<[[BB; 64]; 64]> = Lazy::new(|| {
    let mut table = [[BB::new(0); 64]; 64];
    for sq1 in Square::all() {
        for sq2 in Square::all() {
            let (r1, f1) = sq1.rank_file();
            let (r2, f2) = sq2.rank_file();
            let dr = r1 as i8 - r2 as i8;
            let df = f1 as i8 - f2 as i8;
            if !(dr == 0 || df == 0) {
                continue;
            }
            let mut segment = BB::new(0);
            let (mut r, mut f) = (r2 as i8, f2 as i8);
            while !(r as u8 == r1 && f as u8 == f1) {
                let sq = Square::from_rank_file(r as u8, f as u8);
                segment |= sq;
                r += dr.signum();
                f += df.signum();
            }
            segment |= sq1.bb();
            segment &= !sq2.bb();
            table[sq1 as usize][sq2 as usize] = segment;
        }
    }
    table
});

pub static SEGMENT_DIAGONAL: Lazy<[[BB; 64]; 64]> = Lazy::new(|| {
    let mut table = [[BB::new(0); 64]; 64];
    for sq1 in Square::all() {
        for sq2 in Square::all() {
            let (r1, f1) = sq1.rank_file();
            let (r2, f2) = sq2.rank_file();
            let dr = r1 as i8 - r2 as i8;
            let df = f1 as i8 - f2 as i8;
            if !(dr.abs() == df.abs()) {
                continue;
            }
            let mut segment = BB::new(0);
            let (mut r, mut f) = (r2 as i8, f2 as i8);
            while !(r as u8 == r1 && f as u8 == f1) {
                let sq = Square::from_rank_file(r as u8, f as u8);
                segment |= sq;
                r += dr.signum();
                f += df.signum();
            }
            segment |= sq1.bb();
            segment &= !sq2.bb();
            table[sq1 as usize][sq2 as usize] = segment;
        }
    }
    table
});

pub static RAY: Lazy<[[BB; 64]; 64]> = Lazy::new(|| {
    let mut table = [[BB::new(0); 64]; 64];
    for sq in Square::all() {
        for (dr, df) in [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)] {
            let (r, f) = sq.rank_file();
            let mut r = r as i8;
            let mut f = f as i8;
            while r >= 0 && r < 8 && f >= 0 && f < 8 {
                r += dr;
                f += df;
            }
            r -= dr;
            f -= df;
            let end = Square::from_rank_file(r as u8, f as u8);
            let ray = SEGMENT[end as usize][sq as usize];

            let (r, f) = sq.rank_file();
            let mut r = r as i8 + dr;
            let mut f = f as i8 + df;
            while r >= 0 && r < 8 && f >= 0 && f < 8 {
                let sq2 = Square::from_rank_file(r as u8, f as u8);
                table[sq as usize][sq2 as usize] = ray;
                r += dr;
                f += df;
            }
        }
    }
    table
});

pub static KING_DISTANCE: Lazy<[[u8; 64]; 64]> = Lazy::new(|| {
    let mut table = [[0; 64]; 64];
    for sq1 in Square::all() {
        let (r1, f1) = sq1.rank_file();
        for sq2  in Square::all() {
            let (r2, f2) = sq2.rank_file();
            let (dr, df) = (r1.abs_diff(r2), f1.abs_diff(f2));
            table[sq1 as usize][sq2 as usize] = u8::max(dr, df);
        }
    }
    table
});