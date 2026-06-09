/*
  Moves are encoded in a u16 as follows
  [ flags ][ target square ][ source square ]
  source/target square: 6 bits, corresponds to `Square`
  flags: 4 bits as follows
  0 1 0 0  Capture
  1 0 0 0  Promotion
  And the following combinations
  0 0 0 0  Quiet move
  0 0 0 1  Double pawn push
  0 0 1 0  King-side castle
  0 0 1 1  Queen-side castle
  0 1 0 0  Capture
  0 1 0 1  Enpassant capture
  1 0 0 0  Knight promotion
  1 0 0 1  Bishop promotion
  1 0 1 0  Rook promotion
  1 0 1 1  Queen promotion
  1 1 0 0  Knight promotion capture
  1 1 0 1  Bishop promotion capture
  1 1 1 0  Rook promotion capture
  1 1 1 1  Queen promotion capture  
*/

use std::num::NonZeroU16;

use crate::{bitboard::{Board, Piece, Square}, search::see::{see, see_sign}};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    QUIET          = 0b0000,
    PAWNPUSH       = 0b0001,
    KINGCASTLE     = 0b0010,
    QUEENCASTLE    = 0b0011,
    CAPTURE        = 0b0100,
    ENPASSANT      = 0b0101,
    KNIGHTPROM     = 0b1000,
    BISHOPPROM     = 0b1001,
    ROOKPROM       = 0b1010,
    QUEENPROM      = 0b1011,
    KNIGHTPROMCAP  = 0b1100,
    BISHOPPROMCAP  = 0b1101,
    ROOKPROMCAP    = 0b1110,
    QUEENPROMCAP   = 0b1111,
}

impl Flag {
    pub const CAPTURE_OFFSET: u8 = 2;
    pub const PROMOTION_OFFSET: u8 = 3;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(NonZeroU16);

impl Move {
    pub const SOURCE_OFFSET: u8 = 0;
    pub const TARGET_OFFSET: u8 = 6;
    pub const FLAG_OFFSET: u8 = 12;
    pub const CAPTURE_OFFSET: u8 = Move::FLAG_OFFSET + Flag::CAPTURE_OFFSET;
    pub const PROMOTION_OFFSET: u8 = Move::FLAG_OFFSET + Flag::PROMOTION_OFFSET;

    pub const SOURCE_MASK: u16 = 0b111111 << Move::SOURCE_OFFSET;
    pub const TARGET_MASK: u16 = 0b111111 << Move::TARGET_OFFSET;
    pub const FLAG_MASK: u16  =  0b001111 << Move::FLAG_OFFSET;

    /// Creates a new move
    pub fn new(flag: Flag, target: Square, source: Square) -> Self {
        Move(unsafe { std::mem::transmute(
            (flag as u16) << Move::FLAG_OFFSET
            | (target as u16) << Move::TARGET_OFFSET
            | (source as u16) << Move::SOURCE_OFFSET
        )})
    }

    /// Returns true if the move is a queen promotion
    pub fn is_queen_promotion(self) -> bool {
        const QUEEN_MASK: u16 = 0b11 << Move::FLAG_OFFSET;
        self.0.get() & QUEEN_MASK == QUEEN_MASK
    }

    /// Creates a Move from a UCI move on a given board. Undefined behaviour
    /// may happen if the move is invalid.
    pub fn from_uci(board: &Board, uci: &str) -> Self {
        let source = Square::from_fen(&uci[0..2]).unwrap();
        let target = Square::from_fen(&uci[2..4]).unwrap();
        let piece = board.piece_at(source).unwrap();

        let mut flag = Flag::QUIET;
        if board.piece_at(target).is_some() {
            flag = Flag::CAPTURE;
        }
        if piece == Piece::WhiteKing && source == Square::e1 && target == Square::g1 {
            flag = Flag::KINGCASTLE
        } else if piece == Piece::WhiteKing && source == Square::e1 && target == Square::c1 {
            flag = Flag::QUEENCASTLE;
        } else if piece == Piece::BlackKing && source == Square::e8 && target == Square::g8 {
            flag = Flag::KINGCASTLE;
        } else if piece == Piece::BlackKing && source == Square::e8 && target == Square::c8 {
            flag = Flag::QUEENCASTLE;
        }

        if piece == Piece::WhitePawn && source.rank() == 1 && target.rank() == 3 {
            flag = Flag::PAWNPUSH;
        } else if piece == Piece::BlackPawn && source.rank() == 6 && target.rank() == 4 {
            flag = Flag::PAWNPUSH;
        }

        if board.enpassant.is_some() && target == board.enpassant.unwrap()
            && (piece == Piece::WhitePawn || piece == Piece::BlackPawn) {
            flag = Flag::ENPASSANT;
        }

        if uci.len() == 5 {
            let prom = Piece::from_fen(&uci[4..5]);
            flag = match prom {
                Piece::BlackKnight => Flag::KNIGHTPROM,
                Piece::BlackBishop => Flag::BISHOPPROM,
                Piece::BlackRook => Flag::ROOKPROM,
                Piece::BlackQueen => Flag::QUEENPROM,
                _ => panic!("invalid uci move")
            };
            if board.piece_at(target).is_some() {
                flag = unsafe { std::mem::transmute((flag as u8) | 1 << Flag::CAPTURE_OFFSET) };
            }
        }

        Move::new(flag, target, source)
    }

    /// Returns the source square of the move
    pub fn source_square(self) -> Square {
        unsafe{
            std::mem::transmute(((u16::from(self.0) & Move::SOURCE_MASK) >> Move::SOURCE_OFFSET) as u8)
        }
    }

    /// Returns the target square of the move
    pub fn target_square(self) -> Square {
        unsafe{
            std::mem::transmute(((u16::from(self.0) & Move::TARGET_MASK) >> Move::TARGET_OFFSET) as u8)
        }
    }

    /// Returns the flag associated with the move
    pub fn flag(self) -> Flag {
        unsafe{
            std::mem::transmute(((u16::from(self.0) & Move::FLAG_MASK) >> Move::FLAG_OFFSET) as u8)
        }
    }

    /// Returns true if the move is a capture
    pub fn is_capture(self) -> bool {
        u16::from(self.0) & 1 << Move::CAPTURE_OFFSET != 0
    }

    /// Returns true if the move is a promotion
    pub fn is_promotion(self) -> bool {
        u16::from(self.0) & 1 << Move::PROMOTION_OFFSET != 0
    }

    /// Converts the move to UCI notation (e.g. "e2e4", "e7e8q")
    pub fn to_uci(self) -> String {
        let promo = match self.flag() {
            Flag::KNIGHTPROM | Flag::KNIGHTPROMCAP => "n",
            Flag::BISHOPPROM | Flag::BISHOPPROMCAP => "b",
            Flag::ROOKPROM   | Flag::ROOKPROMCAP   => "r",
            Flag::QUEENPROM  | Flag::QUEENPROMCAP  => "q",
            _ => "",
        };
        format!("{}{}{}", self.source_square().to_fen(), self.target_square().to_fen(), promo)
    }

    /// Converts the move to a string
    pub fn to_string(self) -> String {
        format!(
            "{} to {}, flags: {:04b}",
            self.source_square().to_unicode(),
            self.target_square().to_unicode(),
            self.flag() as u8,
        )
    }

    fn mvvlva_score(piece: Option<Piece>) -> i16 {
        if let Some(p) = piece {
            match p {
                Piece::WhitePawn | Piece::BlackPawn => 1,
                Piece::WhiteKnight | Piece::BlackKnight => 2,
                Piece::WhiteBishop | Piece::BlackBishop => 3,
                Piece::WhiteRook | Piece::BlackRook => 4,
                Piece::WhiteQueen | Piece::BlackQueen => 5,
                Piece::WhiteKing | Piece::BlackKing => 0,
            }
        } else {
            // assume en passant
            1
        }
    }

    pub fn prom_bonus(self) -> i16 {
        if !self.is_promotion() {
            0
        } else if self.is_queen_promotion() {
            100
        } else {
            -8000
        }
    }

    /// Scores a move by the given tables
    pub fn score(self, board: &Board, predicted_best: Option<Move>, killers: &[Option<Move>; 2], history: &[[i16; 64]; 64], counter_move: Option<Move>) -> i16 {
        if Some(self) == predicted_best {
            return i16::MAX;
        }
        
        let prom_bonus = self.prom_bonus();
        if prom_bonus < 0 {
            if self.is_capture() {
                return -10000;
            } else {
                return -11000;
            }
        }
        if self.is_capture() {
            let see = see_sign(board, self);
            if see < 0 {
                return prom_bonus + see - 10000;
            }
            let attacker = board.piece_at(self.source_square());
            let victim = board.piece_at(self.target_square());
            let attacker_score = Move::mvvlva_score(attacker);
            let victim_score = Move::mvvlva_score(victim);
            let mvv =  victim_score * 10 - attacker_score;
            if see == 0 {
                return prom_bonus + mvv + 9910;
            } else {
                return prom_bonus + mvv + 10000;
            }
        }
        // if self.is_queen_promotion() {
        //     return 9950;
        // }
        if Some(self) == killers[0] {
            return 9900;
        }
        if Some(self) == killers[1] {
            return 9850;
        }
        if Some(self) == counter_move {
            return 9800;
        }
        return prom_bonus + history[self.source_square() as usize][self.target_square() as usize];
    }
}

/// Simple list of possible moves in a single position. Capped at 218 as it 
/// is the maximum number of legal moves in a given position.
#[derive(Clone)]
pub struct MoveList {
    pub(crate) moves: [Move; 218],
    pub(crate) length: usize,
    pub(crate) captures: usize,
}

impl MoveList {
    /// Creates a new, empty, movelist
    pub fn new() -> Self {
        MoveList {
            moves: unsafe { std::mem::transmute([u16::MAX; 218]) },
            length: 0,
            captures: 0,
        }
    }

    /// Adds the given move to the movelist. Panics if
    /// length >= 218
    pub fn add(&mut self, move_: Move) {
        if move_.is_capture() {
            self.moves[self.length] = self.moves[self.captures];
            self.moves[self.captures] = move_;
            self.captures += 1;
        } else {
            self.moves[self.length] = move_;
        }
        self.length += 1;
    }

    pub fn sort(&mut self, board: &Board, predicted_best_move: Option<Move>, killers: &[Option<Move>; 2], history: &[[i16; 64]; 64], counter_move: Option<Move>) {
        let mut scores = [0i16; 218];
        for i in 0..self.length {
            scores[i] = self.moves[i].score(board, predicted_best_move, killers, history, counter_move);
        }
        for i in 1..self.length {
            let key = self.moves[i];
            let key_score = scores[i];
            let mut j = i;
            while j > 0 && scores[j - 1] < key_score {
                self.moves[j] = self.moves[j - 1];
                scores[j] = scores[j - 1];
                j -= 1;
            }
            self.moves[j] = key;
            scores[j] = key_score;
        }
    }

    /// Scores the movelist
    pub fn scores(&self, board: &Board, predicted_best_move: Option<Move>, killers: &[Option<Move>; 2], history: &[[i16; 64]; 64], counter_move: Option<Move>) -> [i16; 218] {
        let mut scores = [0i16; 218];
        for i in 0..self.length {
            scores[i] = self.moves[i].score(board, predicted_best_move, killers, history, counter_move);
        }
        scores
    } 

    /// Sorts captures by SEE score descending, leaving quiet moves in place.
    /// Returns the SEE scores parallel to the move list so callers can avoid recomputing.
    pub fn sort_see(&mut self, board: &Board) -> [i16; 218] {
        let mut scores = [0i16; 218];
        for i in 0..self.captures {
            scores[i] = see(board, self.moves[i]);
        }
        for i in 1..self.captures {
            let key = self.moves[i];
            let key_score = scores[i];
            let mut j = i;
            while j > 0 && scores[j - 1] < key_score {
                self.moves[j] = self.moves[j - 1];
                scores[j] = scores[j - 1];
                j -= 1;
            }
            self.moves[j] = key;
            scores[j] = key_score;
        }
        scores
    }

    pub fn sort_see_sign(&mut self, board: &Board) -> [i16; 218] {
        let mut scores = [0i16; 218];
        for i in 0..self.captures {
            scores[i] = see_sign(board, self.moves[i]);
        }
        for i in 1..self.captures {
            let key = self.moves[i];
            let key_score = scores[i];
            let mut j = i;
            while j > 0 && scores[j - 1] < key_score {
                self.moves[j] = self.moves[j - 1];
                scores[j] = scores[j - 1];
                j -= 1;
            }
            self.moves[j] = key;
            scores[j] = key_score;
        }
        scores
    }

    /// Sorts the movelist by mvv-lva
    pub fn sort_mvvlva(&mut self, board: &Board, predicted_best_move: Option<Move>) {
        let mut start_idx = 0;
        if let Some(mv) = predicted_best_move {
            if let Some(idx) = self.moves[..self.length].iter().position(|&m| m == mv) {
                start_idx = 1;
                if mv.is_capture() {
                    self.moves[idx] = self.moves[0];
                    self.moves[0] = mv;
                } else {
                    self.moves[idx] = self.moves[self.captures];
                    self.moves[self.captures] = self.moves[0];
                    self.moves[0] = mv;
                }
            }
        }
        let piece_value = |p: Piece| -> i32 {
            match p {
                Piece::WhitePawn   | Piece::BlackPawn   => 1,
                Piece::WhiteKnight | Piece::BlackKnight => 3,
                Piece::WhiteBishop | Piece::BlackBishop => 3,
                Piece::WhiteRook   | Piece::BlackRook   => 5,
                Piece::WhiteQueen  | Piece::BlackQueen  => 9,
                Piece::WhiteKing   | Piece::BlackKing   => 0,
            }
        };

        let score = |mv: Move| -> i32 {
            let attacker = board.piece_at(mv.source_square()).map_or(0, &piece_value);
            let victim = if mv.flag() == Flag::ENPASSANT {
                1
            } else {
                board.piece_at(mv.target_square()).map_or(0, &piece_value)
            };
            victim * 10 - attacker
        };

        // insertion sort descending on moves[0..self.captures]
        for i in (start_idx + 1)..self.captures {
            let key = self.moves[i];
            let key_score = score(key);
            let mut j = i;
            while j > start_idx && score(self.moves[j - 1]) < key_score {
                self.moves[j] = self.moves[j - 1];
                j -= 1;
            }
            self.moves[j] = key;
        }
    }

    /// Removes the last move by decreasing the length. Undefined behaviour
    /// if length == 0
    pub fn remove_last(&mut self) {
        self.length -= 1;
    }

    pub fn iter(&self) -> impl Iterator<Item = Move> + '_ {
        self.moves[..self.length].iter().copied()
    }

    pub fn lazy_iter(&mut self, board: &Board, predicted_best_move: Option<Move>, killers: &[Option<Move>; 2], history: &[[i16; 64]; 64], counter_move: Option<Move>) -> LazyMoveIter {
        let scores = self.scores(board, predicted_best_move, killers, history, counter_move);
        LazyMoveIter {
            movelist: self,
            scores: scores,
            sorted: 0,
            current: 0
        }
    }
}

pub struct LazyMoveIter<'a> {
    movelist: &'a mut MoveList,
    scores: [i16; 218],
    sorted: usize,
    current: usize,
}

impl<'a> LazyMoveIter<'a> {
    pub fn reset(&mut self) {
        self.current = 0;
    }

    pub fn captures(&self) -> usize {
        self.movelist.captures
    }
}

impl<'a> Iterator for LazyMoveIter<'a> {
    type Item = Move;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.movelist.length {
            None
        } else if self.current < self.sorted {
            self.current += 1;
            Some(self.movelist.moves[self.current - 1])
        } else {
            let mut best = self.current;
            for i in (self.current + 1)..self.movelist.length {
                if self.scores[i] > self.scores[best] {
                    best = i;
                }
            }
            let tmp = self.movelist.moves[best];
            self.movelist.moves[best] = self.movelist.moves[self.current];
            self.movelist.moves[self.current] = tmp;
            let tmp = self.scores[best];
            self.scores[best] = self.scores[self.current];
            self.scores[self.current] = tmp;
            self.current += 1;
            self.sorted = self.current;
            Some(self.movelist.moves[self.current - 1])
        }
    }
}