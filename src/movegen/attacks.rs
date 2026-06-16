#[cfg(not(target_feature = "bmi2"))]
use crate::movegen::tables::{BISHOP_ATTACKS, BISHOP_BITS, BISHOP_MAGIC, BISHOP_MASK, ROOK_ATTACKS, ROOK_BITS, ROOK_MAGIC, ROOK_MASK};
use crate::{movegen::tables::{KING_ATTACKS, KNIGHT_ATTACKS}, repr::{bitboard::BB, board::Board, colour::Colour, piece::{Piece, PieceType}, square::{SEGMENT, Square}}};

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

/// Bitboard of squares attacked by a single knight
pub fn single_knight_attacks(knight: Square) -> BB {
    KNIGHT_ATTACKS[knight as usize]
}

/// Bitboard of squares attacked by a king
pub fn king_attacks(king: BB) -> BB {
    KING_ATTACKS[king.lsb() as usize]
}

pub fn single_king_attacks(king: Square) -> BB {
    KING_ATTACKS[king as usize]
}

/// Bitboard of squares attacked by a rook with the given occupancy
pub fn single_rook_attacks(rook: Square, occupied: BB) -> BB {
    #[cfg(not(target_feature = "bmi2"))] {
        let blockers = ROOK_MASK[rook as usize] & occupied;
        let idx = ROOK_MAGIC[rook as usize].wrapping_mul(blockers.0);
        ROOK_ATTACKS[rook as usize][(idx >> (64 - ROOK_BITS[rook as usize])) as usize]
    }
    #[cfg(target_feature = "bmi2")] {
        use crate::movegen::pext::{ROOK_ATTACKS_FLAT, ROOK_BLOCKER_MASKS, ROOK_OFFSETS, ROOK_POST_MASKS, pext_index};
        let sq = rook as usize;
        let idx = pext_index(occupied, BB::new(ROOK_BLOCKER_MASKS[sq]));
        let compressed = ROOK_ATTACKS_FLAT[ROOK_OFFSETS[sq] as usize + idx];
        BB::new(unsafe { std::arch::x86_64::_pdep_u64(compressed as u64, ROOK_POST_MASKS[sq]) })
    }
}

/// Bitboard of squares attacked by a bishop with the given occupancy
pub fn single_bishop_attacks(bishop: Square, occupied: BB) -> BB {
    #[cfg(not(target_feature = "bmi2"))] {
        let blockers = BISHOP_MASK[bishop as usize] & occupied;
        let idx = BISHOP_MAGIC[bishop as usize].wrapping_mul(blockers.0);
        BISHOP_ATTACKS[bishop as usize][(idx >> (64 - BISHOP_BITS[bishop as usize])) as usize]
    }
    #[cfg(target_feature = "bmi2")] {
        use crate::movegen::pext::{BISHOP_ATTACKS_FLAT, BISHOP_BLOCKER_MASKS, BISHOP_OFFSETS, BISHOP_POST_MASKS, pext_index};
        let sq = bishop as usize;
        let idx = pext_index(occupied, BB::new(BISHOP_BLOCKER_MASKS[sq]));
        let compressed = BISHOP_ATTACKS_FLAT[BISHOP_OFFSETS[sq] as usize + idx];
        BB::new(unsafe { std::arch::x86_64::_pdep_u64(compressed as u64, BISHOP_POST_MASKS[sq]) })
    }
}

/// Bitboard of squares attacked by a queen with the given occupancy
pub fn single_queen_attacks(queen: Square, occupied: BB) -> BB {
    single_rook_attacks(queen, occupied) | single_bishop_attacks(queen, occupied)
}

/// Bitboard of attacks made by all rooks
pub fn rook_attacks(rooks: BB, occupied: BB) -> BB {
    let mut out = BB::new(0);
    for rook in rooks.squares() {
        out |= single_rook_attacks(rook, occupied);
    }
    out
}

/// Bitboard of attacks made by all bishops
pub fn bishop_attacks(bishop: BB, occupied: BB) -> BB {
    let mut out = BB::new(0);
    for bishop in bishop.squares() {
        out |= single_bishop_attacks(bishop, occupied);
    }
    out
}

/// Bitboard of attacks made by all queens
pub fn queen_attacks(queens: BB, occupied: BB) -> BB {
    let mut out = BB::new(0);
    for queen in queens.squares() {
        out |= single_queen_attacks(queen, occupied);
    }
    out
}

impl Board {
    /// Calculates all attacks for the given piece type of the given colour
    pub fn calculate_attacks(&self, colour: Colour, piece_type: PieceType) -> BB {
        match (colour, piece_type) {
            (Colour::White, PieceType::Leaper) => {
                pawn_attacks(self[Piece::WhitePawn], Colour::White)
                | knight_attacks(self[Piece::WhiteKnight])
                | king_attacks(self[Piece::WhiteKing])
            },
            (Colour::Black, PieceType::Leaper) => {
                pawn_attacks(self[Piece::BlackPawn], Colour::Black)
                | knight_attacks(self[Piece::BlackKnight])
                | king_attacks(self[Piece::BlackKing])
            },
            (Colour::White, PieceType::Slider) => {
                rook_attacks(self[Piece::WhiteRook], self.occupied() & !self[Piece::BlackKing])
                | bishop_attacks(self[Piece::WhiteBishop], self.occupied() & !self[Piece::BlackKing])
                | queen_attacks(self[Piece::WhiteQueen], self.occupied() & !self[Piece::BlackKing])
            },
            (Colour::Black, PieceType::Slider) => {
                rook_attacks(self[Piece::BlackRook], self.occupied() & !self[Piece::WhiteKing])
                | bishop_attacks(self[Piece::BlackBishop], self.occupied() & !self[Piece::WhiteKing])
                | queen_attacks(self[Piece::BlackQueen], self.occupied() & !self[Piece::WhiteKing])
            },
        }
    }

    /// Calculates bitboard of pieces checking the king of the side to move
    pub fn calculate_checkers(&self) -> BB {
        let mut out = BB::new(0);
        let king = self[Piece::king(self.colour)];
        let occupied = self.occupied();
        let other = !self.colour;
        out |= pawn_attacks(king, self.colour) & self[Piece::pawn(other)];
        out |= single_knight_attacks(king.lsb()) & self[Piece::knight(other)];
        let rook_attacks = single_rook_attacks(king.lsb(), occupied);
        out |= rook_attacks & self[Piece::rook(!self.colour)];
        let bishop_attacks = single_bishop_attacks(king.lsb(), occupied);
        out |= bishop_attacks & self[Piece::bishop(other)];
        out |= (rook_attacks | bishop_attacks) & self[Piece::queen(other)];
        out
    }

    pub fn compute_raw_xray_and_pinners(&self, colour: Colour) -> (BB, BB) {
        let king = self[Piece::king(colour)].lsb();
        let rook_attacks = single_rook_attacks(king, self.occupied());
        let xray_rook = single_rook_attacks(king, self.occupied() & !(rook_attacks & self[colour]));
        let bishop_attacks = single_bishop_attacks(king, self.occupied());
        let xray_bishop = single_bishop_attacks(king, self.occupied() & !(bishop_attacks & self[colour]));
        let xray = xray_rook | xray_bishop;
        let pinners = (xray_rook & (self[Piece::rook(!colour)] | self[Piece::queen(!colour)]))
            | (xray_bishop & (self[Piece::bishop(!colour)] | self[Piece::queen(!colour)]));
        (xray, pinners)
    }

    pub fn pinned_and_pinners(&self) -> (BB, BB) {
        let colour = self.colour;
        let king = self[Piece::king(colour)].lsb();
        let pinners = self.state.pinners[!colour as usize];
        let mut pinned = BB::new(0);
        for pinner in pinners.squares() {
            pinned |= SEGMENT[pinner as usize][king as usize] & self[colour];
        }
        (pinned, pinners)
    }
}