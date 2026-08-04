use crate::{egtb::threepiece::reflection::Reflection, repr::{piece::Piece, square::Square}, test_assert};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MovingPiece {
    WhiteKing,
    BlackKing,
    P1,
    P2,
    P3
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Quiet,
    Enpassant,
    Promotion,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RevMove {
    pub(crate) to: Square,
    pub(crate) uncaptured: Option<Piece>,
    pub(crate) moving: MovingPiece,
    pub(crate) flag: Flag,
    pub(crate) enpassant: Option<Square>,
    pub(crate) reflection: Option<Reflection>,
}

impl RevMove {
    const EMPTY: Self = Self {
        to: Square::a1,
        uncaptured: Some(Piece::WhitePawn),
        moving: MovingPiece::WhiteKing,
        flag: Flag::Quiet,
        enpassant: Some(Square::a1),
        reflection: Some(Reflection::Horizontal)
    };

    pub fn new_full(to: Square, uncaptured: Option<Piece>, moving: MovingPiece, flag: Flag, enpassant: Option<Square>, reflection: Option<Reflection>) -> Self {
        Self { to, uncaptured, moving, flag, enpassant, reflection }
    }

    pub fn new_quiet(to: Square, uncaptured: Option<Piece>, moving: MovingPiece) -> Self {
        Self::new_full(to, uncaptured, moving, Flag::Quiet, None, None)
    }
}

pub struct RevMoveList {
    pub list: [RevMove; Self::MAX_MOVES],
    pub length: usize,
}

impl RevMoveList {
    pub const MAX_MOVES: usize = 600;

    pub fn new() -> Self {
        Self {
            list: [RevMove::EMPTY; Self::MAX_MOVES],
            length: 0
        }
    }

    pub fn add(&mut self, revmove: RevMove) {
        self.list[self.length] = revmove;
        self.length += 1;
    }

    pub fn remove(&mut self, index: usize) {
        test_assert!(index < self.length);
        self.list[index] = self.list[self.length - 1];
        self.length -= 1;
    }
}