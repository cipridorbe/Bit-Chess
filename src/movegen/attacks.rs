/*
 Contains functions that compute the squares that, potentially multiple,
 pieces can attack.
*/

use crate::{bitboard::{Board, Side, Square}, movegen::tables::{BISHOP_ATTACKS, BISHOP_BITS, BISHOP_MAGIC, BISHOP_MASK, KING_ATTACKS, ROOK_ATTACKS, ROOK_BITS, ROOK_MAGIC, ROOK_MASK}, util::{lsb_index_pow2, squares}};

/// Returns the bitboards of squares attacked by the pawns of the given side
pub fn pawn_attacks(pawns: u64, side: Side) -> u64 {
    let mut attacks = 0;
    if side == Side::White {
        attacks |= (pawns & !Board::A_FILE) << 7;
        attacks |= (pawns & !Board::H_FILE) << 9;
    } else {
        attacks |= (pawns & !Board::A_FILE) >> 9;
        attacks |= (pawns & !Board::H_FILE) >> 7;
    }
    attacks
}

/// Returns the bitboards of squares attacked by the given knights
pub fn knight_attacks(knights: u64) -> u64 {
    let mut attacks = 0;
    attacks |= (knights & !Board::A_FILE) << 15;
    attacks |= (knights & !Board::A_FILE) >> 17;
    attacks |= (knights & !(Board::A_FILE | Board::B_FILE)) << 6;
    attacks |= (knights & !(Board::A_FILE | Board::B_FILE)) >> 10;
    attacks |= (knights & !Board::H_FILE) << 17;
    attacks |= (knights & !Board::H_FILE) >> 15;
    attacks |= (knights & !(Board::H_FILE | Board::G_FILE)) << 10;
    attacks |= (knights & !(Board::H_FILE | Board::G_FILE)) >> 6;
    attacks
}

/// Returns the bitboards of squares attacked by the given king
pub fn king_attacks(king: u64) -> u64 {
    KING_ATTACKS[lsb_index_pow2(king) as usize]
}

/// Returns the squares that are attacked by at least one rook
pub fn rook_attacks(rooks: u64, occupancy: u64) -> u64 {
    let mut attacks = 0;
    for rook in squares(rooks) {
        attacks |= single_rook_attacks(rook, occupancy);
    }
    attacks
}

/// Returns the squares that are attacked by at least one bishop
pub fn bishop_attacks(bishops: u64, occupancy: u64) -> u64 {
    let mut attacks = 0;
    for bishop in squares(bishops) {
        attacks |= single_rook_attacks(bishop, occupancy);
    }
    attacks
}

/// Returns the squares that are attacked by at least one queen
pub fn queen_attacks(queens: u64, occupancy: u64) -> u64 {
    let mut attacks = 0;
    for queen in squares(queens) {
        attacks |= single_rook_attacks(queen, occupancy);
        attacks |= single_bishop_attacks(queen, occupancy);
    }
    attacks
}

/// Returns the attacks a rook at the given square makes with the given
/// occupancy
pub fn single_rook_attacks(square: Square, occupancy: u64) -> u64 {
    let index = ROOK_MAGIC[square as usize].wrapping_mul(occupancy & ROOK_MASK[square as usize]);
    ROOK_ATTACKS[square as usize][(index >> (64 - ROOK_BITS[square as usize])) as usize]
}

/// Returns the attacks a bishop at the given square makes with the given
/// occupancy
pub fn single_bishop_attacks(square: Square, occupancy: u64) -> u64 {
    let index = BISHOP_MAGIC[square as usize].wrapping_mul(occupancy & BISHOP_MASK[square as usize]);
    BISHOP_ATTACKS[square as usize][(index >> (64 - BISHOP_BITS[square as usize])) as usize]
}

// =============================================================================
//                           KING/CASTLING
// =============================================================================

/// Retrusn true if the king is in check
pub fn is_in_check(attacks: u64, king: u64) -> bool {
    attacks & king != 0
}

/// Determines whether or not an attack prevents the king from castling
/// queen/king side. True indicates the king can castle
pub fn can_castle(attacks: u64, occupancy: u64, side: Side) -> (bool, bool) {
    const KING_SIDE_WHITE:  u64 = (1 << (Square::e1 as u8)) | (1 << (Square::f1 as u8)) | (1 << (Square::g1 as u8));
    const QUEEN_SIDE_WHITE: u64 = (1 << (Square::c1 as u8)) | (1 << (Square::d1 as u8)) | (1 << (Square::e1 as u8));
    const KING_SIDE_BLACK:  u64 = (1 << (Square::e8 as u8)) | (1 << (Square::f8 as u8)) | (1 << (Square::g8 as u8));
    const QUEEN_SIDE_BLACK: u64 = (1 << (Square::c8 as u8)) | (1 << (Square::d8 as u8)) | (1 << (Square::e8 as u8));

    const WHITE_KING: u64 = 1 << (Square::e1 as u8);
    const BLACK_KING: u64 = 1 << (Square::e8 as u8);

    if side == Side::White {
        (
            occupancy & QUEEN_SIDE_WHITE == WHITE_KING && attacks & QUEEN_SIDE_WHITE == 0,
            occupancy & KING_SIDE_WHITE == WHITE_KING && attacks & KING_SIDE_WHITE == 0
        )
    } else {
        (
            occupancy & QUEEN_SIDE_BLACK == BLACK_KING && attacks & QUEEN_SIDE_BLACK == 0,
            occupancy & KING_SIDE_BLACK == BLACK_KING && attacks & KING_SIDE_BLACK == 0
        )
    }
}