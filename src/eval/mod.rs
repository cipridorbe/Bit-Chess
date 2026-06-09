use once_cell::sync::Lazy;
use crate::{bitboard::{Board, Piece}, util::squares};

pub const PIECE_VALUE: [f32; 12] = [
    01., 03., 03., 05., 09., 0.,
    -1., -3., -3., -5., -9., 0.
];

// Piece-square tables in centipawns from White's perspective.
// Indexed rank 1..8 × file a..h, matching square indices 0..63.
// Source: Tomasz Michniewski's simplified evaluation function.
#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,  // rank 1
     5, 10, 10,-20,-20, 10, 10,  5,  // rank 2
     5, -5,-10,  0,  0,-10, -5,  5,  // rank 3
     0,  0,  0, 20, 20,  0,  0,  0,  // rank 4
     5,  5, 10, 25, 25, 10,  5,  5,  // rank 5
    10, 10, 20, 30, 30, 20, 10, 10,  // rank 6
    50, 50, 50, 50, 50, 50, 50, 50,  // rank 7
     0,  0,  0,  0,  0,  0,  0,  0,  // rank 8
];

#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,  // rank 1
    -40,-20,  0,  5,  5,  0,-20,-40,  // rank 2
    -30,  5, 10, 15, 15, 10,  5,-30,  // rank 3
    -30,  0, 15, 20, 20, 15,  0,-30,  // rank 4
    -30,  5, 15, 20, 20, 15,  5,-30,  // rank 5
    -30,  0, 10, 15, 15, 10,  0,-30,  // rank 6
    -40,-20,  0,  0,  0,  0,-20,-40,  // rank 7
    -50,-40,-30,-30,-30,-30,-40,-50,  // rank 8
];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,  // rank 1
    -10,  5,  0,  0,  0,  0,  5,-10,  // rank 2
    -10, 10, 10, 10, 10, 10, 10,-10,  // rank 3
    -10,  0, 10, 10, 10, 10,  0,-10,  // rank 4
    -10,  5,  5, 10, 10,  5,  5,-10,  // rank 5
    -10,  0,  5, 10, 10,  5,  0,-10,  // rank 6
    -10,  0,  0,  0,  0,  0,  0,-10,  // rank 7
    -20,-10,-10,-10,-10,-10,-10,-20,  // rank 8
];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
     0,  0,  0,  5,  5,  0,  0,  0,  // rank 1
    -5,  0,  0,  0,  0,  0,  0, -5,  // rank 2
    -5,  0,  0,  0,  0,  0,  0, -5,  // rank 3
    -5,  0,  0,  0,  0,  0,  0, -5,  // rank 4
    -5,  0,  0,  0,  0,  0,  0, -5,  // rank 5
    -5,  0,  0,  0,  0,  0,  0, -5,  // rank 6
     5, 10, 10, 10, 10, 10, 10,  5,  // rank 7
     0,  0,  0,  0,  0,  0,  0,  0,  // rank 8
];

#[rustfmt::skip]
const QUEEN_PST: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,  // rank 1
    -10,  0,  5,  0,  0,  0,  0,-10,  // rank 2
    -10,  5,  5,  5,  5,  5,  0,-10,  // rank 3
      0,  0,  5,  5,  5,  5,  0, -5,  // rank 4
     -5,  0,  5,  5,  5,  5,  0, -5,  // rank 5
    -10,  0,  5,  5,  5,  5,  0,-10,  // rank 6
    -10,  0,  0,  0,  0,  0,  0,-10,  // rank 7
    -20,-10,-10, -5, -5,-10,-10,-20,  // rank 8
];

#[rustfmt::skip]
const KING_PST: [i32; 64] = [
     20, 30, 10,  0,  0, 10, 30, 20,  // rank 1
     20, 20,  0,  0,  0,  0, 20, 20,  // rank 2
    -10,-20,-20,-20,-20,-20,-20,-10,  // rank 3
    -20,-30,-30,-40,-40,-30,-30,-20,  // rank 4
    -30,-40,-40,-50,-50,-40,-40,-30,  // rank 5
    -30,-40,-40,-50,-50,-40,-40,-30,  // rank 6
    -30,-40,-40,-50,-50,-40,-40,-30,  // rank 7
    -30,-40,-40,-50,-50,-40,-40,-30,  // rank 8
];

/// PST[piece][square] — positional bonus in centipawns.
/// White pieces (0–5) use the tables above directly.
/// Black pieces (6–11) mirror the table vertically (sq ^ 56 flips rank)
/// and negate, so the score is still from White's perspective.
pub static PST: Lazy<[[i32; 64]; 12]> = Lazy::new(|| {
    let white = [PAWN_PST, KNIGHT_PST, BISHOP_PST, ROOK_PST, QUEEN_PST, KING_PST];
    let mut pst = [[0i32; 64]; 12];
    for (i, table) in white.iter().enumerate() {
        pst[i] = *table;
        for sq in 0..64usize {
            pst[i + 6][sq] = -table[sq ^ 56];
        }
    }
    pst
});

pub fn eval(board: &Board) -> f32 {
    piece_eval(board) + piece_position_bonus(board)
}

/// Evaluates a board state using only piece values
pub fn piece_eval(board: &Board) -> f32 {
    let mut score = 0.;
    for piece in Piece::ALL {
        let bb = board.pieces[piece as usize];
        score += PIECE_VALUE[piece as usize] * bb.count_ones() as f32;
    }
    score
}

/// Evaluates positional bonuses using piece-square tables.
/// Returns a score in pawn units (positive = good for White).
pub fn piece_position_bonus(board: &Board) -> f32 {
    let mut score = 0i32;
    for piece in Piece::ALL {
        let bb = board.pieces[piece as usize];
        for square in squares(bb) {
            score += PST[piece as usize][square as usize];
        }
    }
    score as f32 / 100.0
}
