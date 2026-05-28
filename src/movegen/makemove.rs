/*
Contains `make_move` which updates the board state to match the given move
*/

use crate::{bitboard::{Board, CASTLING_HASH, ENPASSANT_HASH, POSITION_PIECE_HASH, Piece, SIDE_HASH, Side, Square}, eval::{PIECE_VALUE_EG, PIECE_VALUE_MG, PST_EG, PST_MG}, movegen::{attacks::{king_attacks, knight_attacks, pawn_attacks, single_bishop_attacks, single_rook_attacks}, r#move::{Flag, Move}}, util::lsb_index};

/// Performs the given move on the board by updating its state
pub fn make_move(board: &mut Board, mv: Move) -> UnmakeInfo {
    let side = board.side;
    let piece = board.piece_at(mv.source_square()).unwrap();
    let source_bb = 1 << mv.source_square() as u8;
    let target_bb = 1 << mv.target_square() as u8;

    let mut unmake = UnmakeInfo::read(board);
    unmake.moved = piece;

    let mut other_in_check_after = false;
    let mut self_in_check_after = false;

    let captured = board.mailbox[mv.target_square() as usize];

    board.pieces[piece as usize] &= !source_bb;
    board.pieces[piece as usize] |= target_bb;
    board.mailbox[mv.source_square() as usize] = None;
    board.mailbox[mv.target_square() as usize] = Some(piece);
    board.mg_score += PST_MG[piece as usize][mv.target_square() as usize] - PST_MG[piece as usize][mv.source_square() as usize];
    board.eg_score += PST_EG[piece as usize][mv.target_square() as usize] - PST_EG[piece as usize][mv.source_square() as usize];

    board.hash ^= POSITION_PIECE_HASH[piece as usize][mv.source_square() as usize];
    board.hash ^= POSITION_PIECE_HASH[piece as usize][mv.target_square() as usize];
    board.hash ^= CASTLING_HASH[board.castling as usize];
    board.hash ^= SIDE_HASH[board.side as usize];
    if let Some(square) = board.enpassant {
        let (_, file) = square.to_rank_file();
        board.hash ^= ENPASSANT_HASH[file as usize];
    }
    board.enpassant = None;

    if let Some(cap) = captured {
        // Remove captured piece
        unmake.captured = Some(cap);
        board.pieces[cap as usize] &= !target_bb;
        board.mg_score -= PIECE_VALUE_MG[cap as usize] + PST_MG[cap as usize][mv.target_square() as usize];
        board.eg_score -= PIECE_VALUE_EG[cap as usize] + PST_EG[cap as usize][mv.target_square() as usize];
        board.hash ^= POSITION_PIECE_HASH[cap as usize][mv.target_square() as usize];
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
        board.mg_score -= PST_MG[piece as usize][mv.target_square() as usize] + PIECE_VALUE_MG[piece as usize];
        board.eg_score -= PST_EG[piece as usize][mv.target_square() as usize] + PIECE_VALUE_EG[piece as usize];
        board.hash ^= POSITION_PIECE_HASH[piece as usize][mv.target_square() as usize];
    } else {
        let other_king = board.pieces[Piece::king(side.other()) as usize];
        other_in_check_after = match piece {
            Piece::WhitePawn | Piece::BlackPawn => pawn_attacks(board.pieces[Piece::pawn(side) as usize], side) & other_king != 0,
            Piece::WhiteKnight | Piece::BlackKnight => knight_attacks(board.pieces[Piece::knight(side) as usize]) & other_king != 0,
            Piece::WhiteKing | Piece::BlackKing => {
                let king_attacks = king_attacks(board.pieces[Piece::king(side) as usize]);
                self_in_check_after = king_attacks & other_king != 0;
                king_attacks & other_king != 0
            },
            _ => false
        }
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
            board.mailbox[pawn_square as usize] = None;
            board.mg_score -= PST_MG[Piece::pawn(side.other()) as usize][pawn_square as usize] + PIECE_VALUE_MG[Piece::pawn(side.other()) as usize];
            board.eg_score -= PST_EG[Piece::pawn(side.other()) as usize][pawn_square as usize] + PIECE_VALUE_EG[Piece::pawn(side.other()) as usize];
            board.hash ^= POSITION_PIECE_HASH[Piece::pawn(side.other()) as usize][pawn_square as usize];
        }
        Flag::PAWNPUSH => {
            let new_enpassant = if side == Side::White {
                mv.source_square() as u8 + 8
            } else {
                mv.source_square() as u8 - 8
            };
            let enpassant_square = unsafe { std::mem::transmute(new_enpassant) }; 
            board.enpassant = Some(enpassant_square);
            let (_, enpassant_file) = enpassant_square.to_rank_file();
            board.hash ^= ENPASSANT_HASH[enpassant_file as usize];
        },
        Flag::KINGCASTLE => {
            if side == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::h1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::f1 as u8;
                board.mailbox[Square::h1 as usize] = None;
                board.mailbox[Square::f1 as usize] = Some(Piece::WhiteRook);
                board.mg_score += PST_MG[Piece::WhiteRook as usize][Square::f1 as usize] - PST_MG[Piece::WhiteRook as usize][Square::h1 as usize];
                board.eg_score += PST_EG[Piece::WhiteRook as usize][Square::f1 as usize] - PST_EG[Piece::WhiteRook as usize][Square::h1 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::WhiteRook as usize][Square::h1 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::WhiteRook as usize][Square::f1 as usize];
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::h8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::f8 as u8;
                board.mailbox[Square::h8 as usize] = None;
                board.mailbox[Square::f8 as usize] = Some(Piece::BlackRook);
                board.mg_score += PST_MG[Piece::BlackRook as usize][Square::f8 as usize] - PST_MG[Piece::BlackRook as usize][Square::h8 as usize];
                board.eg_score += PST_EG[Piece::BlackRook as usize][Square::f8 as usize] - PST_EG[Piece::BlackRook as usize][Square::h8 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::BlackRook as usize][Square::h8 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::BlackRook as usize][Square::f8 as usize];
            }
        },
        Flag::QUEENCASTLE => {
            if side == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::a1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::d1 as u8;
                board.mailbox[Square::a1 as usize] = None;
                board.mailbox[Square::d1 as usize] = Some(Piece::WhiteRook);
                board.mg_score += PST_MG[Piece::WhiteRook as usize][Square::d1 as usize] - PST_MG[Piece::WhiteRook as usize][Square::a1 as usize];
                board.eg_score += PST_EG[Piece::WhiteRook as usize][Square::d1 as usize] - PST_EG[Piece::WhiteRook as usize][Square::a1 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::WhiteRook as usize][Square::a1 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::WhiteRook as usize][Square::d1 as usize];
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::a8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::d8 as u8;
                board.mailbox[Square::a8 as usize] = None;
                board.mailbox[Square::d8 as usize] = Some(Piece::BlackRook);
                board.mg_score += PST_MG[Piece::BlackRook as usize][Square::d8 as usize] - PST_MG[Piece::BlackRook as usize][Square::a8 as usize];
                board.eg_score += PST_EG[Piece::BlackRook as usize][Square::d8 as usize] - PST_EG[Piece::BlackRook as usize][Square::a8 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::BlackRook as usize][Square::a8 as usize];
                board.hash ^= POSITION_PIECE_HASH[Piece::BlackRook as usize][Square::d8 as usize];
            }
        },
        Flag::KNIGHTPROM | Flag::KNIGHTPROMCAP => {
            board.pieces[Piece::knight(side) as usize] |= target_bb;
            board.mailbox[mv.target_square() as usize] = Some(Piece::knight(side));
            board.mg_score += PIECE_VALUE_MG[Piece::knight(side) as usize] + PST_MG[Piece::knight(side) as usize][mv.target_square() as usize];
            board.eg_score += PIECE_VALUE_EG[Piece::knight(side) as usize] + PST_EG[Piece::knight(side) as usize][mv.target_square() as usize];
            board.hash ^= POSITION_PIECE_HASH[Piece::knight(side) as usize][mv.target_square() as usize];
            other_in_check_after = knight_attacks(board.pieces[Piece::knight(side) as usize]) & board.pieces[Piece::king(side.other()) as usize] != 0
        },
        Flag::BISHOPPROM | Flag::BISHOPPROMCAP => {
            board.pieces[Piece::bishop(side) as usize] |= target_bb;
            board.mailbox[mv.target_square() as usize] = Some(Piece::bishop(side));
            board.mg_score += PIECE_VALUE_MG[Piece::bishop(side) as usize] + PST_MG[Piece::bishop(side) as usize][mv.target_square() as usize];
            board.eg_score += PIECE_VALUE_EG[Piece::bishop(side) as usize] + PST_EG[Piece::bishop(side) as usize][mv.target_square() as usize];
            board.hash ^= POSITION_PIECE_HASH[Piece::bishop(side) as usize][mv.target_square() as usize];
        },
        Flag::ROOKPROM | Flag::ROOKPROMCAP => {
            board.pieces[Piece::rook(side) as usize] |= target_bb;
            board.mailbox[mv.target_square() as usize] = Some(Piece::rook(side));
            board.mg_score += PIECE_VALUE_MG[Piece::rook(side) as usize] + PST_MG[Piece::rook(side) as usize][mv.target_square() as usize];
            board.eg_score += PIECE_VALUE_EG[Piece::rook(side) as usize] + PST_EG[Piece::rook(side) as usize][mv.target_square() as usize];
            board.hash ^= POSITION_PIECE_HASH[Piece::rook(side) as usize][mv.target_square() as usize];
        },
        Flag::QUEENPROM | Flag::QUEENPROMCAP => {
            board.pieces[Piece::queen(side) as usize] |= target_bb;
            board.mailbox[mv.target_square() as usize] = Some(Piece::queen(side));
            board.mg_score += PIECE_VALUE_MG[Piece::queen(side) as usize] + PST_MG[Piece::queen(side) as usize][mv.target_square() as usize];
            board.eg_score += PIECE_VALUE_EG[Piece::queen(side) as usize] + PST_EG[Piece::queen(side) as usize][mv.target_square() as usize];
            board.hash ^= POSITION_PIECE_HASH[Piece::queen(side) as usize][mv.target_square() as usize];
        },
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
    
    board.hash ^= CASTLING_HASH[board.castling as usize];
    board.hash ^= SIDE_HASH[board.side as usize];

    board.repetitions = board.history.add(board.hash, mv, piece);

    other_in_check_after = other_in_check_after || in_slider_attacked(board, board.pieces[Piece::king(side.other()) as usize], side.other());
    let in_check_before = board.in_check(side);
    if !self_in_check_after {
        if in_check_before || piece == Piece::WhiteKing || piece == Piece::BlackKing {
            self_in_check_after = square_attacked(board, board.pieces[Piece::king(side) as usize], side);
        } else {
            self_in_check_after = in_slider_attacked(board, board.pieces[Piece::king(side) as usize], side);
        }
    }

    if side == Side::White {
        board.white_in_check = self_in_check_after;
        board.black_in_check = other_in_check_after;
    } else {
        board.white_in_check = other_in_check_after;
        board.black_in_check = self_in_check_after;
    }

    board.phase = board.calculate_phase();

    unmake
}

// checks if the bb square of the given side is being checked by a slider of the other side
fn in_slider_attacked(board: &Board, bb: u64, side: Side) -> bool {
    let square = unsafe { std::mem::transmute(lsb_index(bb)) };
    single_bishop_attacks(square, board.occupied) & (board.pieces[Piece::bishop(side.other()) as usize] | board.pieces[Piece::queen(side.other()) as usize]) != 0
    || single_rook_attacks(square, board.occupied) & (board.pieces[Piece::rook(side.other()) as usize] | board.pieces[Piece::queen(side.other()) as usize]) != 0
}

// bb should only have 1 set bit
fn square_attacked(board: &Board, bb: u64, side: Side) -> bool {
    pawn_attacks(bb, side) & board.pieces[Piece::pawn(side.other()) as usize] != 0
    || knight_attacks(bb) & board.pieces[Piece::knight(side.other()) as usize] != 0
    || in_slider_attacked(board, bb, side)
    || king_attacks(bb) & board.pieces[Piece::king(side.other()) as usize] != 0
}

/// Restores the board state to the state before the last move
pub fn unmake_move(board: &mut Board, mv: Move, unmake: &UnmakeInfo) {
    let source_bb = 1 << mv.source_square() as u8;
    let target_bb = 1 << mv.target_square() as u8;
    let piece = unmake.moved;

    board.pieces[piece as usize] &= !(target_bb);
    board.pieces[piece as usize] |= source_bb;
    board.mailbox[mv.target_square() as usize] = None;
    board.mailbox[mv.source_square() as usize] = Some(piece);

    if mv.is_capture() && mv.flag() != Flag::ENPASSANT {
        board.pieces[unmake.captured.unwrap() as usize] |= target_bb;
        board.mailbox[mv.target_square() as usize] = unmake.captured;
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
            board.mailbox[captured_square as usize] = Some(Piece::pawn(board.side));
        },
        Flag::KINGCASTLE => {
            if board.side.other() == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::f1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::h1 as u8;
                board.mailbox[Square::f1 as usize] = None;
                board.mailbox[Square::h1 as usize] = Some(Piece::WhiteRook);
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::f8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::h8 as u8;
                board.mailbox[Square::f8 as usize] = None;
                board.mailbox[Square::h8 as usize] = Some(Piece::BlackRook);
            }
        },
        Flag::QUEENCASTLE => {
            if board.side.other() == Side::White {
                board.pieces[Piece::WhiteRook as usize] &= !(1 << Square::d1 as u8);
                board.pieces[Piece::WhiteRook as usize] |= 1 << Square::a1 as u8;
                board.mailbox[Square::d1 as usize] = None;
                board.mailbox[Square::a1 as usize] = Some(Piece::WhiteRook);
            } else {
                board.pieces[Piece::BlackRook as usize] &= !(1 << Square::d8 as u8);
                board.pieces[Piece::BlackRook as usize] |= 1 << Square::a8 as u8;
                board.mailbox[Square::d8 as usize] = None;
                board.mailbox[Square::a8 as usize] = Some(Piece::BlackRook);
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

/// Makes a null move on the given board
pub fn make_null_move(board: &mut Board) -> UnmakeInfo {
    let unmake = UnmakeInfo::read(board);
    if board.side == Side::Black {
        board.fullmoves += 1;
    }
    board.halfmoves = 0;
    board.side = board.side.other();
    board.hash ^= SIDE_HASH[Side::White as usize] ^ SIDE_HASH[Side::Black as usize];
    if let Some(square) = board.enpassant {
        let (_, file) = square.to_rank_file();
        board.hash ^= ENPASSANT_HASH[file as usize];
        board.enpassant = None;
    }
    board.history.hashes.push(board.hash);
    board.repetitions = 0;
    board.white_in_check = false;
    board.black_in_check = false;
    unmake
}

/// Unmakes a null move on the given board
pub fn unmake_null_move(board: &mut Board, unmake: &UnmakeInfo) {
    board.side = board.side.other();
    unmake.write(board);
}

/// Struct containing information required for unmake move
#[derive(Clone)]
pub struct UnmakeInfo {
    pub(crate) castling: u8,
    pub(crate) enpassant: Option<Square>,
    pub(crate) halfmoves: u8,
    pub(crate) fullmoves: u8,
    pub(crate) moved: Piece,
    pub(crate) captured: Option<Piece>,
    pub(crate) hash: u64,
    pub(crate) start_idx: usize,
    pub(crate) mg_score: i16,
    pub(crate) eg_score: i16,
    pub(crate) repetitions: u8,
    pub(crate) white_in_check: bool,
    pub(crate) black_in_check: bool,
    pub(crate) phase: u8,
}

impl UnmakeInfo {
    /// Reads all but `captured` from the board
    pub fn read(board: &Board) -> Self {
        UnmakeInfo {
            castling: board.castling,
            enpassant: board.enpassant,
            halfmoves: board.halfmoves,
            fullmoves: board.fullmoves,
            moved: Piece::WhitePawn,
            captured: None,
            hash: board.hash,
            start_idx: board.history.start_idx,
            mg_score: board.mg_score,
            eg_score: board.eg_score,
            repetitions: board.repetitions,
            white_in_check: board.white_in_check,
            black_in_check: board.black_in_check,
            phase: board.phase,
        }
    }

    /// Writes all but `captured` into the board
    pub fn write(&self, board: &mut Board) {
        board.castling = self.castling;
        board.enpassant = self.enpassant;
        board.halfmoves = self.halfmoves;
        board.fullmoves = self.fullmoves;
        board.hash = self.hash;
        board.history.start_idx = self.start_idx;
        board.history.pop();
        board.mg_score = self.mg_score;
        board.eg_score = self.eg_score;
        board.repetitions = self.repetitions;
        board.white_in_check = self.white_in_check;
        board.black_in_check = self.black_in_check;
        board.phase = self.phase;
    }
}

/// Returns true if the side that just moved left their king in check.
// pub fn is_in_check_after_move(board: &Board) -> bool {
//     let side = board.side.other();
//     let king_bb = board.pieces[Piece::king(side) as usize];
//     all_attacks(board, board.side) & king_bb != 0
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::r#move::Move;

    fn play(board: &mut Board, uci: &str) {
        let mv = Move::from_uci(board, uci);
        make_move(board, mv);
    }

    // Kings-only position — no captures or pawn moves possible, so halfmoves
    // keeps incrementing and start_idx never moves.
    const KINGS_FEN: &str = "8/8/8/8/8/8/7k/4K3 w - - 0 1";

    #[test]
    fn threefold_repetition_detected() {
        let mut board = Board::from_fen(KINGS_FEN);
        // Shuttle the kings back and forth.  The initial hash is already in
        // history at index 0.  One full cycle (4 moves) brings the position
        // back once (2nd occurrence, not yet drawn).  After two full cycles
        // (8 moves) it's the 3rd occurrence → draw detected.
        for _ in 0..2 {
            play(&mut board, "e1d1");
            play(&mut board, "h2g2");
            play(&mut board, "d1e1");
            play(&mut board, "g2h2");
        }
        assert!(board.is_rule_draw(), "expected threefold repetition after two full cycles");
    }

    #[test]
    fn not_draw_after_one_cycle() {
        let mut board = Board::from_fen(KINGS_FEN);
        play(&mut board, "e1d1");
        play(&mut board, "h2g2");
        play(&mut board, "d1e1");
        play(&mut board, "g2h2");
        assert!(!board.is_rule_draw(), "one cycle is only the 2nd occurrence — not yet drawn");
    }

    #[test]
    fn fifty_move_rule_triggers() {
        // halfmoves = 49; one more quiet king move brings it to 50 → draw
        let mut board = Board::from_fen("8/8/8/8/8/8/7k/4K3 w - - 49 1");
        assert!(!board.is_rule_draw());
        play(&mut board, "e1d1");
        assert!(board.is_rule_draw(), "halfmoves=50 should trigger the 50-move rule");
    }

    #[test]
    fn fifty_move_rule_not_triggered_below_limit() {
        // halfmoves = 48 → after one quiet move it's 49, still not a draw
        let mut board = Board::from_fen("8/8/8/8/8/8/7k/4K3 w - - 48 1");
        play(&mut board, "e1d1");
        assert!(!board.is_rule_draw(), "halfmoves=49 should not yet be a draw");
    }
}