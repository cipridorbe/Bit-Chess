use crate::repr::{bitboard::BB, square::Square};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Reflection {
    Horizontal,
    Vertical,
    Diagonal,
    Rotation, // 90 degrees clockwise
}

impl Reflection {
    pub fn apply(self, square: Square) -> Square {
        match self {
            Self::Horizontal => HORIZONTAL[square],
            Self::Vertical =>   VERTICAL[square],
            Self::Diagonal =>   DIAGONAL[square],
            Self::Rotation =>   ROTATION[square],
        }
    }
}

#[inline]
pub fn reflect_sq(square: Square, reflection: Option<Reflection>) -> Square {
    match reflection {
        None => square,
        Some(reflection) => reflection.apply(square)
    }
}

#[inline]
pub fn reflect_bb(bb: BB, reflection: Option<Reflection>) -> BB {
    match reflection {
        None => bb,
        Some(reflection) => {
            let mut out = BB::new(0);
            for square in bb.squares() {
                out |= reflection.apply(square);
            }
            out
        }
    }
}

const HORIZONTAL: [Square; 64] = {
    let mut out = [Square::a1; 64];
    let mut i = 0u8;
    while i < 64 {
        let square: Square = unsafe { std::mem::transmute(i) };
        let (rank, file) = square.rank_file();
        let new_rank = rank;
        let new_file = 7 - file;
        out[i as usize] = Square::from_rank_file(new_rank, new_file);
        i += 1;
    }
    out
};

const VERTICAL: [Square; 64] = {
    let mut out = [Square::a1; 64];
    let mut i = 0u8;
    while i < 64 {
        let square: Square = unsafe { std::mem::transmute(i) };
        let (rank, file) = square.rank_file();
        let new_rank = 7 - rank;
        let new_file = file;
        out[i as usize] = Square::from_rank_file(new_rank, new_file);
        i += 1;
    }
    out
};

// Reflection across bottom-left to top-right axis
const DIAGONAL: [Square; 64] = {
    let mut out = [Square::a1; 64];
    let mut i = 0u8;
    while i < 64 {
        let square: Square = unsafe { std::mem::transmute(i) };
        let (rank, file) = square.rank_file();
        let new_rank = file;
        let new_file = rank;
        out[i as usize] = Square::from_rank_file(new_rank, new_file);
        i += 1;
    }
    out
};

// Reflection across top-left to bottom-right axis
const ROTATION: [Square; 64] = {
    let mut out = [Square::a1; 64];
    let mut i = 0;
    while i < 64 {
        out[i] = VERTICAL[DIAGONAL[i] as usize];
        i += 1;
    }
    out
};