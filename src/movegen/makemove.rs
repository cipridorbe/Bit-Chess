/*
Contains `make_move` which updates the board state to match the given move
*/

use crate::{bitboard::{Board, Piece, Side, Square}, movegen::{attacks::all_attacks, r#move::{Flag, Move}}};

/// Performs the given move on the board by updating its state
pub fn make_move(board: &mut Board, mv: Move) -> UnmakeInfo {
    let side = board.side;
    let piece = mv.piece();
    let source_bb = 1 << mv.source_square() as u8;
    let target_bb = 1 << mv.target_square() as u8;

    let mut unmake = UnmakeInfo::read(board);

    board.pieces[piece as usize] &= !source_bb;
    board.pieces[piece as usize] |= target_bb;

    board.enpassant = None;

    if mv.is_capture() && mv.flag() != Flag::ENPASSANT {
        // Remove captured piece
        for enemy_piece in Piece::of_side(side.other()) {
            if board.pieces[enemy_piece as usize] & target_bb != 0 {
                board.pieces[enemy_piece as usize] &= !target_bb;
                unmake.captured = Some(enemy_piece);
                break;
            }
        }
        match mv.target_square() {
            Square::h1 => board.castling &= !Board::WHITE_KING_CASTLE,
            Square::a1 => board.castling &= !Board::WHITE_QUEEN_CASTLE,
            Square::h8 => board.castling &= !Board::BLACK_KING_CASTLE,
            Square::a8 => board.castling &= !Board::BLACK_QUEEN_CASTLE,
        _ => {}
    }
    }

    if mv.is_promotion() {
        // Unset pawn at last row
        board.pieces[piece as usize] &= !(target_bb);
    }

    match mv.flag() {
        Flag::QUIET | Flag::CAPTURE => {},
        Flag::ENPASSANT => {
            let pawn_square = if side == Side::Black {
                unmake.captured = Some(Piece::WhitePawn);
                mv.target_square() as u8 + 8
            } else {
                unmake.captured = Some(Piece::BlackPawn);
                mv.target_square() as u8 - 8
            };
            board.pieces[Piece::pawn(side.other()) as usize] &= !(1 << pawn_square);
        }
        Flag::PAWNPUSH => {
            let new_enpassant = if side == Side::White {
                mv.source_square() as u8 + 8
            } else {
                mv.source_square() as u8 - 8
            };
            board.enpassant = Some(unsafe { std::mem::transmute(new_enpassant) });
        },
        Flag::KINGCASTLE => {
            if side == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::h1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::f1 as u8;
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::h8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::f8 as u8;
            }
        },
        Flag::QUEENCASTLE => {
            if side == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::a1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::d1 as u8;
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::a8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::d8 as u8;
            }
        },
        Flag::KNIGHTPROM | Flag::KNIGHTPROMCAP => board.pieces[Piece::knight(side) as usize] |= target_bb,
        Flag::BISHOPPROM | Flag::BISHOPPROMCAP => board.pieces[Piece::bishop(side) as usize] |= target_bb,
        Flag::ROOKPROM | Flag::ROOKPROMCAP => board.pieces[Piece::rook(side) as usize] |= target_bb,
        Flag::QUEENPROM | Flag::QUEENPROMCAP => board.pieces[Piece::queen(side) as usize] |= target_bb,
    }

    // update multi-piece bitboards
    board.sides[Side::White as usize] = 
        board.pieces[Piece::WhiteBishop as usize] | board.pieces[Piece::WhiteKing as usize] |
        board.pieces[Piece::WhiteKnight as usize] | board.pieces[Piece::WhitePawn as usize] |
        board.pieces[Piece::WhiteQueen as usize] | board.pieces[Piece::WhiteRook as usize];
    board.sides[Side::Black as usize] = 
        board.pieces[Piece::BlackBishop as usize] | board.pieces[Piece::BlackKing as usize] |
        board.pieces[Piece::BlackKnight as usize] | board.pieces[Piece::BlackPawn as usize] |
        board.pieces[Piece::BlackQueen as usize] | board.pieces[Piece::BlackRook as usize];
    board.occupied = board.sides[Side::White as usize] | board.sides[Side::Black as usize];

    // Update castling rights (only if they exist)
    if side == Side::White {
        if board.castling & (Board::WHITE_KING_CASTLE | Board::WHITE_QUEEN_CASTLE) != 0 {
            if piece == Piece::WhiteKing {
                board.castling &= !(Board::WHITE_KING_CASTLE | Board::WHITE_QUEEN_CASTLE);
            } else if piece == Piece::WhiteRook {
                if mv.source_square() == Square::h1 {
                    board.castling &= !Board::WHITE_KING_CASTLE;
                } else if mv.source_square() == Square::a1 {
                    board.castling &= !Board::WHITE_QUEEN_CASTLE;
                }
            }
        }
    } else {
        if board.castling & (Board::BLACK_KING_CASTLE | Board::BLACK_QUEEN_CASTLE) != 0 {
            if piece == Piece::BlackKing {
                board.castling &= !(Board::BLACK_KING_CASTLE | Board::BLACK_QUEEN_CASTLE);
            } else if piece == Piece::BlackRook {
                if mv.source_square() == Square::h8 {
                    board.castling &= !Board::BLACK_KING_CASTLE;
                } else if mv.source_square() == Square::a8 {
                    board.castling &= !Board::BLACK_QUEEN_CASTLE;
                }
            }
        }
    }

    // Update move counts
    if side == Side::Black {
        board.fullmoves += 1;
    }
    if mv.is_capture() || piece == Piece::WhitePawn || piece == Piece::BlackPawn {
        board.halfmoves = 0;
    } else {
        board.halfmoves += 1;
    }

    // change side to play
    board.side = side.other();
    
    unmake
}

/// Restores the board state to the state before the last move
pub fn unmake_move(board: &mut Board, mv: Move, unmake: UnmakeInfo) {
    let source_bb = 1 << mv.source_square() as u8;
    let target_bb = 1 << mv.target_square() as u8;
    let piece = mv.piece();

    board.pieces[piece as usize] &= !(target_bb);
    board.pieces[piece as usize] |= source_bb;

    if mv.is_capture() && mv.flag() != Flag::ENPASSANT {
        board.pieces[unmake.captured.unwrap() as usize] |= target_bb;
    }

    match mv.flag() {
        Flag::QUIET | Flag::CAPTURE | Flag::PAWNPUSH => {},
        Flag::ENPASSANT => {
            let captured_square = if board.side == Side::White {
                mv.target_square() as u8 + 8
            } else {
                mv.target_square() as u8 - 8
            };
            board.pieces[Piece::pawn(board.side) as usize] |= 1 << captured_square;
        },
        Flag::KINGCASTLE => {
            if board.side.other() == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::f1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::h1 as u8;
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::f8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::h8 as u8;
            }
        },
        Flag::QUEENCASTLE => {
            if board.side.other() == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::d1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::a1 as u8;
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::d8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::a8 as u8;
            }
        },
        Flag::KNIGHTPROM | Flag::KNIGHTPROMCAP => {
            board.pieces[Piece::knight(board.side.other()) as usize] &= !target_bb;
        },
        Flag::BISHOPPROM | Flag::BISHOPPROMCAP => {
            board.pieces[Piece::bishop(board.side.other()) as usize] &= !target_bb;
        },
        Flag::ROOKPROM | Flag::ROOKPROMCAP => {
            board.pieces[Piece::rook(board.side.other()) as usize] &= !target_bb;
        },
        Flag::QUEENPROM | Flag::QUEENPROMCAP => {
            board.pieces[Piece::queen(board.side.other()) as usize] &= !target_bb;
        }
    }
    
    board.sides[Side::White as usize] =
        board.pieces[Piece::WhiteBishop as usize] | board.pieces[Piece::WhiteKing as usize] |
        board.pieces[Piece::WhiteKnight as usize] | board.pieces[Piece::WhitePawn as usize] |
        board.pieces[Piece::WhiteQueen as usize]  | board.pieces[Piece::WhiteRook as usize];
    board.sides[Side::Black as usize] =
        board.pieces[Piece::BlackBishop as usize] | board.pieces[Piece::BlackKing as usize] |
        board.pieces[Piece::BlackKnight as usize] | board.pieces[Piece::BlackPawn as usize] |
        board.pieces[Piece::BlackQueen as usize]  | board.pieces[Piece::BlackRook as usize];
    board.occupied = board.sides[Side::White as usize] | board.sides[Side::Black as usize];

    board.side = board.side.other();
    unmake.write(board);
}

/// Struct containing information required for unmake move
#[derive(Clone, Copy)]
pub struct UnmakeInfo {
    castling: u8,
    enpassant: Option<Square>,
    halfmoves: u8,
    fullmoves: u8,
    captured: Option<Piece>
}

impl UnmakeInfo {
    /// Reads all but `captured` from the board
    pub fn read(board: &Board) -> Self {
        UnmakeInfo {
            castling: board.castling,
            enpassant: board.enpassant,
            halfmoves: board.halfmoves,
            fullmoves: board.fullmoves,
            captured: None
        }
    }

    /// Writes all but `captured` into the board
    pub fn write(self, board: &mut Board) {
        board.castling = self.castling;
        board.enpassant = self.enpassant;
        board.halfmoves = self.halfmoves;
        board.fullmoves = self.fullmoves;
    }
}

/// Returns true if the side that just moved left their king in check.
pub fn is_in_check_after_move(board: &Board) -> bool {
    let side = board.side.other();
    let king_bb = board.pieces[Piece::king(side) as usize];
    all_attacks(board, board.side) & king_bb != 0
}