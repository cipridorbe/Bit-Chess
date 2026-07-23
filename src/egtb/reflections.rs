use crate::repr::square::Square;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Reflections {
    pub colour: bool,
    pub vertical: bool,
    pub horizontal: bool,
    pub diagonal: bool,
}

impl Reflections {
    pub fn empty() -> Self {
        Reflections {
            colour: false,
            vertical: false,
            horizontal: false,
            diagonal: false
        }
    }
}

pub const VERTICAL: [Square; 64] = {
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

pub const HORIZONTAL: [Square; 64] = {
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

// bottom left to top right diagonal reflection
pub const DIAGONAL: [Square; 64] = {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReflectionSequence {
    None,
    D,  // inverse is D
    V,  // inverse is V
    VD, // inverse is D then V
}

impl ReflectionSequence {
    pub fn apply_square(self, sq: Square) -> Square {
        match self {
            Self::None => sq,
            Self::D    => DIAGONAL[sq],
            Self::V    => VERTICAL[sq],
            Self::VD   => VERTICAL[DIAGONAL[sq]],
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn diagvert() {
        let square = Square::a2;
        let diag = DIAGONAL[square];
        let vert = VERTICAL[diag];
        assert!(vert == Square::b8);
    }
}