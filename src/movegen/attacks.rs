use crate::repr::{bitboard::BB, board::Board, colour::Colour};

/// Bitboard of pawn attacks of the given colour
pub fn pawn_attacks(pawns: BB, colour: Colour) -> BB {
    let mut out = BB::new(0);
    match colour {
        Colour::White => {
            out |= (pawns & !Board::A_FILE) << 7;
            out |= (pawns & !Board::H_FILE) << 9;
        },
        Colour::Black => {
            out |= (pawns & !Board::A_FILE) >> 9;
            out |= (pawns & !Board::H_FILE) >> 7;
        }
    }
    out
}

/// Bitboard of knight attacks
pub fn knight_attacks(knights: BB) -> BB {
    let mut out = BB::new(0);
    out |= (knights & !Board::A_FILE) << 15;
    out |= (knights & !Board::A_FILE) >> 17;
    out |= (knights & !(Board::A_FILE | Board::B_FILE)) << 6;
    out |= (knights & !(Board::A_FILE | Board::B_FILE)) >> 10;
    out |= (knights & !Board::H_FILE) << 17;
    out |= (knights & !Board::H_FILE) >> 15;
    out |= (knights & !(Board::H_FILE | Board::G_FILE)) << 10;
    out |= (knights & !(Board::H_FILE | Board::G_FILE)) >> 6;
    out
}

/// Bitboard of king attacks
pub fn king_attacks(king: BB) -> BB {
    
}