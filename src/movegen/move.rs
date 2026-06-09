/*
  Moves are encoded in a u32 as follows
  [ 0s ][ flags ][ piece moved ][ target square ][ source square ]
  source/target square: 6 bits, corresponds to `Square`
  piece moved: 4 bits, corresponds to `Piece`
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

use crate::bitboard::{Board, Piece, Side, Square};

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
pub struct Move(u32);

impl Move {
    pub const SOURCE_OFFSET: u8 = 0;
    pub const TARGET_OFFSET: u8 = 6;
    pub const PIECE_OFFSET: u8 = 12;
    pub const FLAG_OFFSET: u8 = 16;
    pub const CAPTURE_OFFSET: u8 = Move::FLAG_OFFSET + Flag::CAPTURE_OFFSET;
    pub const PROMOTION_OFFSET: u8 = Move::FLAG_OFFSET + Flag::PROMOTION_OFFSET;

    pub const SOURCE_MASK: u32 = 0b111111 << Move::SOURCE_OFFSET;
    pub const TARGET_MASK: u32 = 0b111111 << Move::TARGET_OFFSET;
    pub const PIECE_MASK: u32 =  0b001111 << Move::PIECE_OFFSET;
    pub const FLAG_MASK: u32  =  0b001111 << Move::FLAG_OFFSET;

    /// Creates a new move
    pub fn new(flag: Flag, piece: Piece, target: Square, source: Square) -> Self {
        Move(
            (flag as u32) << Move::FLAG_OFFSET
            | (piece as u32) << Move::PIECE_OFFSET
            | (target as u32) << Move::TARGET_OFFSET
            | (source as u32) << Move::SOURCE_OFFSET
        )
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

        Move::new(flag, piece, target, source)
    }

    /// Returns the source square of the move
    pub fn source_square(self) -> Square {
        unsafe{
            std::mem::transmute(((self.0 & Move::SOURCE_MASK) >> Move::SOURCE_OFFSET) as u8)
        }
    }

    /// Returns the target square of the move
    pub fn target_square(self) -> Square {
        unsafe{
            std::mem::transmute(((self.0 & Move::TARGET_MASK) >> Move::TARGET_OFFSET) as u8)
        }
    }

    /// Returns the moving piece of the move
    pub fn piece(self) -> Piece {
        unsafe{
            std::mem::transmute(((self.0 & Move::PIECE_MASK) >> Move::PIECE_OFFSET) as u8)
        }
    }

    /// Returns the flag associated with the move
    pub fn flag(self) -> Flag {
        unsafe{
            std::mem::transmute(((self.0 & Move::FLAG_MASK) >> Move::FLAG_OFFSET) as u8)
        }
    }

    /// Returns true if the move is a capture
    pub fn is_capture(self) -> bool {
        self.0 & 1 << Move::CAPTURE_OFFSET != 0
    }

    /// Returns true if the move is a promotion
    pub fn is_promotion(self) -> bool {
        self.0 & 1 << Move::PROMOTION_OFFSET != 0
    }

    /// Converts the move to a string
    pub fn to_string(self) -> String {
        format!(
            "{} from {} to {}, flags: {:04b}",
            self.piece().to_unicode(),
            self.source_square().to_unicode(),
            self.target_square().to_unicode(),
            self.flag() as u8,
        )
    }
}

/// Simple list of possible moves in a single position. Capped at 218 as it 
/// is the maximum number of legal moves in a given position.
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; 218],
    length: usize, 
}

impl MoveList {
    /// Creates a new, empty, movelist
    pub fn new() -> Self {
        MoveList {
            moves: unsafe { std::mem::transmute([0; 218]) },
            length: 0
        }
    }

    /// Adds the given move to the end of the movelist. Panics if
    /// length >= 218
    pub fn add(&mut self, move_: Move) {
        self.moves[self.length] = move_;
        self.length += 1;
    }

    /// Removes the last move by decreasing the length. Undefined behaviour
    /// if length == 0
    pub fn remove_last(&mut self) {
        self.length -= 1;
    }
}