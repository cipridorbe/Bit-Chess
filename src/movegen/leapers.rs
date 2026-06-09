/*
 Contains functions that return the bitboards of squares attacked by leaper
 pieces (pawns, knights, kings). Should only be used to initialize the attack
 tables which enable a faster lookup 
 */

use crate::bitboard::{Board, Side, Square};
use crate::util::format_bitboard;

/// Returns bitboards of squares attacked by a pawn at the given square 
/// of the given side
pub fn pawn_attacks(square: Square, side: Side, is_attacking: bool) -> u64 {
    let mut out = 0;
    let bitboard = 1 << square as u8;
    match side {
        Side::White => {
            if is_attacking {
                out |= (bitboard & !Board::A_FILE) << 7;
                out |= (bitboard & !Board::H_FILE) << 9;
            } else {
                out |= bitboard << 8;
                let (rank, _) = square.to_rank_file();
                if rank == 1 { out |= bitboard << 16; }
            }
        },
        Side::Black => {
            if is_attacking {
                out |= (bitboard & !Board::A_FILE) >> 9;
                out |= (bitboard & !Board::H_FILE) >> 7;
            } else {
                out |= bitboard >> 8;
                let (rank, _) = square.to_rank_file();
                if rank == 6 { out |= bitboard >> 16; }
            }
        }
    }
    out
}

/// Returns the bitboard of squares attacked by a knight at the given square
pub fn knight_attacks(square: Square) -> u64 {
    let mut out = 0;
    let bitboard = 1 << square as u8;
    // Move left by 1
    out |= (bitboard & !Board::A_FILE) << 15;
    out |= (bitboard & !Board::A_FILE) >> 17;
    // Move left by 2
    out |= (bitboard & !(Board::A_FILE | Board::B_FILE)) << 6;
    out |= (bitboard & !(Board::A_FILE | Board::B_FILE)) >> 10;
    // Move right by 1
    out |= (bitboard & !Board::H_FILE) << 17;
    out |= (bitboard & !Board::H_FILE) >> 15;
    // Move right by 2
    out |= (bitboard & !(Board::H_FILE | Board::G_FILE)) << 10;
    out |= (bitboard & !(Board::H_FILE | Board::G_FILE)) >> 6;

    out
}

/// Returns the bitboard of squares attacked by a king at the given square
pub fn king_attacks(square: Square) -> u64 {
    let mut out = 0;
    let bitboard = 1 << square as u8;
    // vertical attacks
    out |= bitboard << 8;
    out |= bitboard >> 8;
    // left attacks
    out |= (bitboard & !Board::A_FILE) >> 1;
    out |= (bitboard & !Board::A_FILE) >> 9;
    out |= (bitboard & !Board::A_FILE) << 7;
    // right attacks
    out |= (bitboard & !Board::H_FILE) << 1;
    out |= (bitboard & !Board::H_FILE) << 9;
    out |= (bitboard & !Board::H_FILE) >> 7;

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print_pawn(square: Square, side: Side, attacking: bool) {
        let label = format!("{} pawn at {} ({})",
            if matches!(side, Side::White) { "White" } else { "Black" },
            square.to_fen(),
            if attacking { "attacks" } else { "pushes" });
        println!("{}\n{}\n", label, format_bitboard(pawn_attacks(square, side, attacking)));
    }

    fn print_knight(square: Square) {
        println!("Knight at {}\n{}\n", square.to_fen(), format_bitboard(knight_attacks(square)));
    }

    fn print_king(square: Square) {
        println!("King at {}\n{}\n", square.to_fen(), format_bitboard(king_attacks(square)));
    }

    #[test]
    fn pawn_attack_scenarios() {
        // White attacks — normal and file edges
        print_pawn(Square::e4, Side::White, true);
        print_pawn(Square::a4, Side::White, true);
        print_pawn(Square::h4, Side::White, true);
        // Black attacks — normal and file edges
        print_pawn(Square::e5, Side::Black, true);
        print_pawn(Square::a5, Side::Black, true);
        print_pawn(Square::h5, Side::Black, true);
    }

    #[test]
    fn pawn_push_scenarios() {
        // White pushes — normal and starting rank (double push)
        print_pawn(Square::e4, Side::White, false);
        print_pawn(Square::e2, Side::White, false);
        print_pawn(Square::a2, Side::White, false);
        print_pawn(Square::h2, Side::White, false);
        // Black pushes — normal and starting rank (double push)
        print_pawn(Square::e5, Side::Black, false);
        print_pawn(Square::e7, Side::Black, false);
        print_pawn(Square::a7, Side::Black, false);
        print_pawn(Square::h7, Side::Black, false);
    }

    #[test]
    fn knight_attack_scenarios() {
        // Normal
        print_knight(Square::e4);
        // Corners
        print_knight(Square::a1);
        print_knight(Square::a8);
        print_knight(Square::h1);
        print_knight(Square::h8);
        // Near-corner
        print_knight(Square::b1);
        print_knight(Square::b8);
        // Edge files, middle rank
        print_knight(Square::a4);
        print_knight(Square::h4);
    }

    #[test]
    fn king_attack_scenarios() {
        // Normal
        print_king(Square::e4);
        // Corners
        print_king(Square::a1);
        print_king(Square::a8);
        print_king(Square::h1);
        print_king(Square::h8);
        // Edge files, middle rank
        print_king(Square::a4);
        print_king(Square::h4);
    }
}