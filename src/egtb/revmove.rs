use crate::{egtb::reflections::ReflectionSequence, repr::{piece::Piece, square::Square}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MovingPiece {
    WhiteKing,
    BlackKing,
    P1,
    P2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RevMove {
    // to is after possible unpromotion rotations
    pub to: Square,
    pub flag: u8,
    pub reflection: ReflectionSequence,
}

impl RevMove {
    pub const WHITE: [Option<Piece>; 6] = [None, Some(Piece::WhitePawn), Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen)];
    pub const BLACK: [Option<Piece>; 6] = [None, Some(Piece::BlackPawn), Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)];
    pub const WHITEPAWNLESS: [Option<Piece>; 5] = [None, Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen)];
    pub const BLACKPAWNLESS: [Option<Piece>; 5] = [None, Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)];
    pub const NONE: Self = Self { to: Square::a1, flag: 0, reflection: ReflectionSequence::None };

    const UNCAPTURE_MASK: u8 = 0x0f;
    const MOVING_PIECE_MASK: u8 = 0x30;
    const PROM_MASK: u8 = 0x80;
    const ENPASSANT_MASK: u8 = 0x40;

    pub fn new_raw(to: Square, flag: u8) -> Self {
        Self { to, flag, reflection: ReflectionSequence::None }
    }

    pub fn new(to: Square, uncaptured: Option<Piece>, moving_piece: MovingPiece, is_unpromotion: bool, is_unenpassant: bool) -> Self {
        let mut flag: u8 = match uncaptured {
            Some(p) => p as u8,
            None => 12,
        };
        flag |= (moving_piece as u8) << 4;
        if is_unpromotion { flag |= Self::PROM_MASK; }
        if is_unenpassant { flag |= Self::ENPASSANT_MASK; }
        Self::new_raw(to, flag)
    }

    pub fn with_reflection(mut self, seq: ReflectionSequence) -> Self {
        self.reflection = seq;
        self
    }

    pub fn is_quiet(self) -> bool {
        self.flag & !Self::MOVING_PIECE_MASK == 0 
    }

    pub fn uncaptured_piece(self) -> Option<Piece> {
        match self.flag & Self::UNCAPTURE_MASK {
            12 => None,
            n => Some(unsafe { std::mem::transmute(n) }),
        }
    }

    pub fn moving_piece(self) -> MovingPiece {
        unsafe { std::mem::transmute((self.flag & Self::MOVING_PIECE_MASK) >> 4) }
    }

    pub fn is_uncapture(self) -> bool {
        self.uncaptured_piece().is_some()
    }

    pub fn is_unpromotion(self) -> bool {
        self.flag & Self::PROM_MASK != 0
    }

    pub fn is_unenpassant(self) -> bool {
        self.flag & Self::ENPASSANT_MASK != 0 && !self.is_unpromotion()
    }

    pub fn unpromote_diagonal(self) -> bool {
        self.flag & Self::ENPASSANT_MASK != 0 && self.is_unpromotion()
    }

}

pub const MAX_REV_MOVES: usize = 400;

pub struct RevMoveList {
    pub list: [RevMove; MAX_REV_MOVES],
    pub length: usize,
}

impl RevMoveList {
    pub fn new() -> Self {
        Self {
            list: [RevMove::NONE; MAX_REV_MOVES],
            length: 0
        }
    }

    pub fn add(&mut self, rev_move: RevMove) {
        self.list[self.length] = rev_move;
        self.length += 1;
    }
}