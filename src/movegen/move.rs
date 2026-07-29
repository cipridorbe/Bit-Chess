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

use std::{num::NonZeroU16, ops::{Index, IndexMut}};

use crate::{eval::Eval, repr::{bitboard::BB, board::Board, colour::Colour, piece::Piece, square::Square}, search::{see::{see_mvvlva, see_sign}, state::{History, SearchState}}, test_assert};

/// Maximum number of moves that can be made from any position
pub const MAX_MOVES: usize = 218;

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

pub type MoveScore = i16;
pub const TTSCORE: MoveScore = MoveScore::MAX;
pub const CAPTURE_BASE_SCORE: MoveScore = 30000;
pub const POSITIVE_SEE_OFFSET: MoveScore = 500;
pub const QUEEN_QUIET_PROM_SCORE: MoveScore = 29100;
pub const KILLERS_SCORE: [MoveScore; 2] = [29000, 28000];
pub const COUNTERMOVE_SCORE: MoveScore = 27000;
pub const REGULAR_QUIET_SCORE: MoveScore = 26000;
pub const MAX_HISTORY_VALUE: MoveScore = 16384;

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

    pub const NULL_MOVE: Move = unsafe { std::mem::transmute(0x0fffu16) };

    const fn new_invalid() -> Self {
        unsafe { std::mem::transmute(0xffffu16) }
    }

    pub fn promoted_piece(self, colour: Colour) -> Piece {
        test_assert!(self.is_promotion());
        let bits = (self.0.get() >> Move::FLAG_OFFSET) & 0b11;
        let piece = (bits as u8 + 1) + (colour as u8 * 6);
        unsafe { std::mem::transmute(piece) }
    }

    pub fn into_queen_prom(self) -> Self {
        test_assert!(self.flag() == Flag::QUIET || self.flag() == Flag::CAPTURE);
        Move(self.0 | 0b1011u16 << Move::FLAG_OFFSET)
    }

    pub fn into_knight_prom(&mut self) -> Self {
        test_assert!(self.flag() == Flag::QUIET || self.flag() == Flag::CAPTURE);
        Move(self.0 | 0b1000u16 << Move::FLAG_OFFSET)
    }

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
        const QUEEN_MASK: u16 = 0b1011 << Move::FLAG_OFFSET;
        self.0.get() & QUEEN_MASK == QUEEN_MASK
    }

    /// Creates a Move from a UCI move on a given board. Undefined behaviour
    /// may happen if the move is invalid.
    pub fn from_uci(board: &Board, uci: &str) -> Self {
        let source = Square::from_fen(&uci[0..2]).unwrap();
        let target = Square::from_fen(&uci[2..4]).unwrap();
        let piece = board[source].unwrap();

        let mut flag = Flag::QUIET;
        if board[target].is_some() {
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
            if board[target].is_some() {
                flag = unsafe { std::mem::transmute((flag as u8) | 1 << Flag::CAPTURE_OFFSET) };
            }
        }

        Move::new(flag, target, source)
    }

    /// Returns the source square of the move
    pub fn source_square(self) -> Square {
        Square::from_u8(((self.0.get() & Move::SOURCE_MASK) >> Move::SOURCE_OFFSET) as u8)
    }

    /// Returns the target square of the move
    pub fn target_square(self) -> Square {
        Square::from_u8(((self.0.get() & Move::TARGET_MASK) >> Move::TARGET_OFFSET) as u8)
    }

    /// Returns the flag associated with the move
    pub fn flag(self) -> Flag {
        unsafe{
            std::mem::transmute(((u16::from(self.0) & Move::FLAG_MASK) >> Move::FLAG_OFFSET) as u8)
        }
    }

    /// Returns true if the move is a capture
    pub fn is_capture(self) -> bool {
        self.0.get() & (1 << Move::CAPTURE_OFFSET) != 0
    }

    /// Returns true if the move is a promotion
    pub fn is_promotion(self) -> bool {
        self.0.get() & (1 << Move::PROMOTION_OFFSET) != 0
    }

    pub fn score(self, board: &Board, tt_move: Option<Move>, second_best_move: Option<Move>, killers: &[Option<Move>; 2], history: &History, counter_move: Option<Move>) -> MoveScore {
        if Some(self) == tt_move { return TTSCORE; }
        if Some(self) == second_best_move { return TTSCORE - 1; }
        if Some(self) == killers[0] { return KILLERS_SCORE[0]; }
        if Some(self) == killers[1] { return KILLERS_SCORE[1]; }
        // if Some(self) == counter_move { return COUNTERMOVE_SCORE; }
        if self.is_capture() {
            return see_mvvlva(board, self);
        } else {
            if self.is_queen_promotion() { return QUEEN_QUIET_PROM_SCORE; }
            history.get(board, self)
        }
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
}

pub struct MoveList {
    moves: [Move; MAX_MOVES],
    pub length: usize,
    pub explored: isize,
    captures: usize,
    pub pinned: BB,
    pub queen_proms: u8,
}

impl MoveList {
    /// Creates a new, empty movelist
    pub fn new() -> Self {
        MoveList {
            moves: [Move::new_invalid(); MAX_MOVES],
            length: 0,
            explored: -1,
            captures: 0,
            pinned: BB::new(0),
            queen_proms: 0,
        }
    }

    pub fn num_total_moves(&self) -> usize {
        self.length + self.queen_proms as usize * 2
    }

    /// Adds a move to the movelist
    pub fn add(&mut self, mv: Move) {
        if !mv.is_capture() {
            self.moves[self.length] = mv;
        } else {
            self.moves[self.length] = self.moves[self.captures];
            self.moves[self.captures] = mv;
            self.captures += 1;
        }
        self.length += 1;
    }

    pub fn add_to_end(&mut self, mv: Move) {
        self.moves[self.length] = mv;
        self.length += 1;
        if mv.is_capture() {
            self.captures += 1;
        }
    }

    pub fn score(&self, board: &Board, search_state: &SearchState, tt_move: Option<Move>, second_best_move: Option<Move>, ply: u8) -> [Eval; MAX_MOVES] {
        let mut out = [0; MAX_MOVES];
        let mut i = 0;
        let killers = &search_state.killers[ply as usize];
        let counter_move = board.move_history.last().and_then(|prev| 
            search_state.counter_move[prev.0.source_square() as usize][prev.0.target_square() as usize]
        );
        while i < self.length {
            out[i] = self[i].score(board, tt_move, second_best_move, killers, &search_state.history, counter_move);
            i += 1;
        }
        out
    }

    pub fn quiescense_score(&self, board: &Board) -> [Eval; MAX_MOVES] {
        let mut out = [0; MAX_MOVES];
        for i in 0..self.length {
            out[i] = see_sign(board, self[i]);
        }
        out
    }

    pub fn sort(&mut self, scores: &mut [MoveScore; MAX_MOVES]) {
        for i in 1..self.length {
            let mv = self[i];
            let score = scores[i];
            let mut j = i;
            while j > 0 && scores[j - 1] < score {
                self[j] = self[j - 1];
                scores[j] = scores[j - 1];
                j -= 1;
            }
            self[j] = mv;
            scores[j] = score;
        }
    }

    // shifts a move upwards
    #[inline]
    pub fn shift(&mut self, scores: &mut [MoveScore; MAX_MOVES], start: usize, end: usize) {
        let mv = self[start];
        let score = scores[start];
        let mut i = start;
        while i > end {
            self[i] = self[i - 1];
            scores[i] = scores[i - 1];
            i -= 1;
        }
        self[end] = mv;
        scores[end] = score; 
    }

    #[inline]
    pub fn maybe_add_proms(&mut self, score: Eval, mv: Option<Move>, i: usize) {
        if i as isize > self.explored && score == 0 && mv.is_some() && mv.unwrap().is_queen_promotion() {
            let tt_move = mv.unwrap();
            if tt_move.is_capture() {
                self.add(Move::new(Flag::ROOKPROMCAP, tt_move.target_square(), tt_move.source_square()));
                self.add(Move::new(Flag::BISHOPPROMCAP, tt_move.target_square(), tt_move.source_square()));
            } else {
                self.add(Move::new(Flag::ROOKPROM, tt_move.target_square(), tt_move.source_square()));
                self.add(Move::new(Flag::BISHOPPROM, tt_move.target_square(), tt_move.source_square()));
            }
        }
        self.explored = self.explored.max(i as isize);
    }
}

impl Index<usize> for MoveList {
    type Output = Move;
    fn index(&self, index: usize) -> &Self::Output {
        test_assert!(index < self.length);
        test_assert!(self.moves[index] != Move::new_invalid());
        &self.moves[index]
    }
}

impl IndexMut<usize> for MoveList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        test_assert!(index < self.length);
        test_assert!(self.moves[index] != Move::new_invalid());
        &mut self.moves[index]
    }
}