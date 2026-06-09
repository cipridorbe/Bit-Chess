/*
Generates the list of all pseudo-legal moves in a given chess position.
*/

use crate::{bitboard::{Board, Piece, Side, Square}, movegen::{attacks::{all_attacks, can_castle, single_bishop_attacks, single_queen_attacks, single_rook_attacks}, r#move::{Flag, Move, MoveList}, tables::{KING_ATTACKS, KNIGHT_ATTACKS, PAWN_ATTACKS}}, util::squares};

/// Generates all pseudo-legal moves from the given board state.
pub fn generate_movelist(board: &Board, captures_only: bool) -> MoveList {
    let mut movelist = MoveList::new();
    let side = board.side;
    generate_castling_movelist(&mut movelist, board, captures_only);
    generate_pawn_movelist(&mut movelist, board, captures_only);
    generate_piece_movelist(&mut movelist, board, captures_only, Piece::knight(side), 
    |square, _| KNIGHT_ATTACKS[square as usize]
    );
    generate_piece_movelist(&mut movelist, board, captures_only, Piece::bishop(side), 
        single_bishop_attacks
    );
    generate_piece_movelist(&mut movelist, board, captures_only, Piece::rook(side), 
        single_rook_attacks
    );
    generate_piece_movelist(&mut movelist, board, captures_only, Piece::queen(side), 
        single_queen_attacks
    );
    generate_piece_movelist(&mut movelist, board, captures_only, Piece::king(side), 
    |square, _| KING_ATTACKS[square as usize]
    );

    movelist
}

/// Generates and appends all pawn moves
fn generate_pawn_movelist(movelist: &mut MoveList, board: &Board, captures_only: bool) {
    let side = board.side;
    let occupancy_other = board.sides[side.other() as usize];
    let occupancy = board.occupied;

    let piece = Piece::pawn(side);
    let pawns = board.pieces[piece as usize];
    for pawn in squares(pawns) {
        // Attacks
        // Can only attack an enemy piece
        let attacks = PAWN_ATTACKS[side as usize][pawn as usize] & occupancy_other;
        for square in squares(attacks) {
            if (1 << square as u8) & (Board::RANK_1 | Board::RANK_8) != 0 {
                // Promotion
                movelist.add(Move::new(Flag::QUEENPROMCAP, square, pawn));
                movelist.add(Move::new(Flag::ROOKPROMCAP, square, pawn));
                movelist.add(Move::new(Flag::BISHOPPROMCAP, square, pawn));
                movelist.add(Move::new(Flag::KNIGHTPROMCAP, square, pawn));
            } else {
                // Regular capture
                movelist.add(Move::new(Flag::CAPTURE, square, pawn));
            }
        }
        // Quiet move and double pawn push
        if !captures_only {
            let push_square = if side == Side::White {
                pawn as u8 + 8
            } else {
                pawn as u8 - 8
            };
            if (1 << push_square) & occupancy == 0 {
                let target = unsafe { std::mem::transmute(push_square) };
                movelist.add(Move::new(Flag::QUIET, target, pawn));
                if (1 << target as u8) & (Board::RANK_1 | Board::RANK_8) != 0 {
                    // promotion
                    movelist.remove_last();
                    movelist.add(Move::new(Flag::QUEENPROM, target, pawn));
                    movelist.add(Move::new(Flag::ROOKPROM, target, pawn));
                    movelist.add(Move::new(Flag::BISHOPPROM, target, pawn));
                    movelist.add(Move::new(Flag::KNIGHTPROM, target, pawn));
                } else if (1 << pawn as u8) & (Board::RANK_2 | Board::RANK_7) != 0 {
                    // double pawn push
                    let double_push_square = if side == Side::White {
                        pawn as u8 + 16
                    } else {
                        pawn as u8 - 16
                    };
                    if (1 << double_push_square) & occupancy == 0 {
                        let double_push_target = unsafe { std::mem::transmute(double_push_square) };
                        movelist.add(Move::new(Flag::PAWNPUSH, double_push_target, pawn));
                    }
                }
            }
        }
    }

    // enpassant
    if let Some(enpsasant) = board.enpassant {
        let enpassant_attacks = PAWN_ATTACKS[side.other() as usize][enpsasant as usize];
        for square in squares(enpassant_attacks & pawns) {
            movelist.add(Move::new(Flag::ENPASSANT, enpsasant, square));
        }
    }
}

/// Generates and appends all moves for the given piece
fn generate_piece_movelist(movelist: &mut MoveList, board: &Board, captures_only: bool, piece: Piece, attacks: impl Fn(Square, u64) -> u64) {
    let side = board.side;
    let occupancy_other = board.sides[side.other() as usize];
    let occupancy = board.occupied;

    for square in squares(board.pieces[piece as usize]) {
        let moves = attacks(square, occupancy);
        // attacks
        for target in squares(moves & occupancy_other) {
            movelist.add(Move::new(Flag::CAPTURE, target, square));
        }
        // Non-attacks
        if !captures_only {
            for target in squares(moves & !occupancy) {
                movelist.add(Move::new(Flag::QUIET, target, square));
            }
        }
    }
}

/// Generates and appends all castling moves
fn generate_castling_movelist(movelist: &mut MoveList, board: &Board, captures_only: bool) {
    if captures_only {
        return;
    }
    let side = board.side;
    let (queenside, kingside) = board.castling_rights(board.side);
    if queenside || kingside {
        let attacks = all_attacks(board, side.other());
        let (queenlegal, kinglegal) = can_castle(attacks, board.occupied, side);
        if queenlegal && queenside {
            let (target, source) = match side {
                Side::White => (Square::c1, Square::e1),
                Side::Black => (Square::c8, Square::e8)
            };
            movelist.add(Move::new(Flag::QUEENCASTLE, target, source));
        }
        if kinglegal && kingside {
            let (target, source) = match side {
                Side::White => (Square::g1, Square::e1),
                Side::Black => (Square::g8, Square::e8)
            };
            movelist.add(Move::new(Flag::KINGCASTLE, target, source));
        }
    }
}