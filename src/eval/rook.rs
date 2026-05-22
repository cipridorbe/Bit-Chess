use crate::{bitboard::{Board, Piece}, eval::{king::file_status, pawn::doubled_pawns}, util::squares};

// indexed by file_status: bit1=own pawn, bit0=enemy pawn
// 0=open, 1=semi-open (enemy pawn only), 2=semi-open (own pawn only), 3=closed
const ROOK_FILE_BONUS: [i16; 4] = [25, 15, 0, 0];
const DOUBLED_ROOK_BONUS: i16 = 15;
const RANK7_BONUS: i16 = 25;

/// middle game and end game bonuses
pub fn rook_bonus(board: &Board) -> (i16, i16) {
    let bonus = rook_bonus_common(board);
    let mg_bonus = rook_bonus_mg(board);
    (bonus + mg_bonus, bonus)
}

fn rook_bonus_mg(board: &Board) -> i16 {
    let mut bonus = 0;
    let white_rq = board.pieces[Piece::WhiteRook as usize] | board.pieces[Piece::WhiteQueen as usize];
    let black_rq = board.pieces[Piece::BlackRook as usize] | board.pieces[Piece::BlackQueen as usize];
    let white_doubled = doubled_pawns(white_rq);
    let black_doubled = doubled_pawns(black_rq);
    bonus += (white_doubled.count_ones() as i16 - black_doubled.count_ones() as i16) * DOUBLED_ROOK_BONUS;
    bonus += (white_rq & Board::RANK_7).count_ones() as i16 * RANK7_BONUS;
    bonus -= (black_rq & Board::RANK_2).count_ones() as i16 * RANK7_BONUS;
    bonus
}

fn rook_bonus_common(board: &Board) -> i16 {
    let white_rooks = board.pieces[Piece::WhiteRook as usize];
    let black_rooks = board.pieces[Piece::BlackRook as usize];
    let white_pawns = board.pieces[Piece::WhitePawn as usize];
    let black_pawns = board.pieces[Piece::BlackPawn as usize];
    let mut bonus = 0;
    for rook in squares(white_rooks) {
        let file_status = file_status(1 << rook as u8, white_pawns, black_pawns);
        bonus += ROOK_FILE_BONUS[file_status as usize];
    }
    for rook in squares(black_rooks) {
        let file_status = file_status(1 << rook as u8, black_pawns, white_pawns);
        bonus -= ROOK_FILE_BONUS[file_status as usize];
    }
    bonus
}