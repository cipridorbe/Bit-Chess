/*
 Contains tables used to quickly access/calculate attack bitboards for 
 different pieces
*/

use once_cell::sync::Lazy;

use crate::{bitboard::{self, Board, Side, Square}, util::all_squares};
// =============================================================================
//                              LEAPER PIECES
// =============================================================================

/// Table of precomputed pawn attacks, indexed by `Side` and `Square`
pub static PAWN_ATTACKS: Lazy<[[u64; 64]; 2]> = Lazy::new(|| {
    let mut table = [[0; 64]; 2];
    for square in all_squares() {
        let bitboard = 1 << square as u8;

        table[Side::White as usize][square as usize] |= (bitboard & !Board::A_FILE) << 7;
        table[Side::White as usize][square as usize] |= (bitboard & !Board::H_FILE) << 9;

        table[Side::Black as usize][square as usize] |= (bitboard & !Board::A_FILE) >> 9;
        table[Side::Black as usize][square as usize] |= (bitboard & !Board::H_FILE) >> 7;
    }
    table
});

/// Table of precomputed knight attacks, indexed by `Square`
pub static KNIGHT_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let mut attacks = 0;
        let bitboard = 1 << square as u8;
        // Move left by 1
        attacks |= (bitboard & !Board::A_FILE) << 15;
        attacks |= (bitboard & !Board::A_FILE) >> 17;
        // Move left by 2
        attacks |= (bitboard & !(Board::A_FILE | Board::B_FILE)) << 6;
        attacks |= (bitboard & !(Board::A_FILE | Board::B_FILE)) >> 10;
        // Move right by 1
        attacks |= (bitboard & !Board::H_FILE) << 17;
        attacks |= (bitboard & !Board::H_FILE) >> 15;
        // Move right by 2
        attacks |= (bitboard & !(Board::H_FILE | Board::G_FILE)) << 10;
        attacks |= (bitboard & !(Board::H_FILE | Board::G_FILE)) >> 6;
        
        table[square as usize] = attacks;
    }
    table
});

/// Table of precomputed knight attacks, indexed by `Square`
pub static KING_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let mut attacks = 0;
        let bitboard = 1 << square as u8;
        // vertical attacks
        attacks |= bitboard << 8;
        attacks |= bitboard >> 8;
        // left attacks
        attacks |= (bitboard & !Board::A_FILE) >> 1;
        attacks |= (bitboard & !Board::A_FILE) >> 9;
        attacks |= (bitboard & !Board::A_FILE) << 7;
        // right attacks
        attacks |= (bitboard & !Board::H_FILE) << 1;
        attacks |= (bitboard & !Board::H_FILE) << 9;
        attacks |= (bitboard & !Board::H_FILE) >> 7;
        
        table[square as usize] = attacks;
    }
    table
});

// =============================================================================
//                      SLIDER PIECES (MAGIC BITBOARDS)
// =============================================================================

/// Mask for board occupancy when calculating rook attacks.
/// The mask consists of the squares the rook would attack in an empty board,
/// not including the last squares
pub static ROOK_MASK: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        for r in 1..7 {
            if r == rank { continue; }
            let attacked_square = Square::from_rank_file(r, file);
            mask |= 1 << attacked_square as u8;
        }
        for f in 1..7 {
            if f == file { continue; }
            let attacked_square = Square::from_rank_file(rank, f);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});

/// Mask for board occupancy when calculating bishop attacks.
/// The mask consists of the squares the bishop would attack in an empty board,
/// not including the last squares
pub static BISHOP_MASK: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        // antidiagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 + i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        // main diagonal
        for i in -8..8 {
            let r = rank as i8 - i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});