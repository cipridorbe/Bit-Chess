use crate::{bitboard::{Board, Piece, Side}, movegen::tables::PAWN_ATTACKS, util::populate_files};

const MISSING_BONUS: i16 = -20;
// indexed by 2-bit file status: bit1=own pawn exists, bit0=enemy pawn exists
// 0b00=open, 0b01=semi-open(us), 0b10=semi-open(enemy), 0b11=closed
const FILE_STATUS_BONUS: [i16; 4] = [-40, -25, -10, 0];

/// Returns the bitboard of missing guards
pub fn missing_guards(king: u64, pawns: u64, side: Side) -> u64 {
    let left = (king & !Board::A_FILE) >> 1;
    let right = (king & !Board::H_FILE) << 1;
    let mask = if side == Side::White {
        (left | right | king) << 8
    } else {
        (left | right | king) >> 8
    };
    mask & !pawns
}

/// closed file = 2/3 (10/11), semi-open file = 1 (01), open file = 0 (00)
pub fn king_files(king: u64, my_pawns: u64, enemy_pawns: u64) -> (u8, u8, u8) {
    let king_file = populate_files(king);
    let left_file = (king_file & !Board::A_FILE) >> 1;
    let right_file = (king_file & !Board::H_FILE) << 1;
    let king_out = ((((king_file & my_pawns) != 0) as u8) << 1) | (((king_file & enemy_pawns) != 0) as u8);
    let left_out = ((((left_file & my_pawns) != 0) as u8) << 1) | (((left_file & enemy_pawns) != 0) as u8);
    let right_out = ((((right_file & my_pawns) != 0) as u8) << 1) | (((right_file & enemy_pawns) != 0) as u8);
    (left_out, king_out, right_out)
}

pub fn king_bonus(board: &Board) -> i16 {
    let white = board.pieces[Piece::WhiteKing as usize];
    let white_pawns = board.pieces[Piece::WhitePawn as usize];
    let black = board.pieces[Piece::BlackKing as usize];
    let black_pawns = board.pieces[Piece::BlackPawn as usize];

    let mut bonus = 0;

    let white_missing = missing_guards(white, white_pawns, Side::White);
    let (wleft, wking, wright) = king_files(white, white_pawns, black_pawns);
    let black_missing = missing_guards(black, black_pawns, Side::Black);
    let (bleft, bking, bright) = king_files(black, black_pawns, white_pawns);

    bonus += (white_missing.count_ones() as i16 - black_missing.count_ones() as i16) * MISSING_BONUS;
    bonus += FILE_STATUS_BONUS[wking as usize] - FILE_STATUS_BONUS[bking as usize];
    if white & !Board::A_FILE != 0 {
        bonus += FILE_STATUS_BONUS[wleft as usize];
    }
    if white & !Board::H_FILE != 0 {
        bonus += FILE_STATUS_BONUS[wright as usize];
    }
    if black & !Board::A_FILE != 0 {
        bonus -= FILE_STATUS_BONUS[bleft as usize];
    }
    if black & !Board::H_FILE != 0 {
        bonus -= FILE_STATUS_BONUS[bright as usize];
    }

    bonus
}