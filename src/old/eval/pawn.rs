use crate::{bitboard::{Board, Piece, Side}, movegen::attacks::pawn_attacks, util::{populate_files, populate_files_down, populate_files_up}};

const FRIEND_BONUS: i16 = 10;
const ISOLATED_BONUS: i16 = -15;
const DOUBLED_BONUS: i16 = -15;
const RANK_BONUS: [i16; 6] = [10, 20, 40, 70, 120, 200];

/// Returns a bitboard of the passed pawns of the current side to play
pub fn passed_pawns(my_pawns: u64, enemy_pawns: u64, side: Side) -> u64 {
    if enemy_pawns.count_ones() == 8 || my_pawns == 0 {
        return 0;
    }
    let pawn_files = enemy_pawns | pawn_attacks(enemy_pawns, side.other());
    let blocked = if side == Side::White {
        populate_files_down(pawn_files)
    } else {
        populate_files_up(pawn_files)
    };
    my_pawns & !blocked
}

/// Returns a bitboard containing pawns with at least one pawn next to them
pub fn friend_pawns(pawns: u64) -> u64 {
    let mut left = (pawns & !Board::A_FILE) >> 1;
    let mut right = (pawns & !Board::H_FILE) << 1;
    left |= left << 8;
    left |= left >> 8;
    right |= right << 8;
    right |= right >> 8;
    pawns & (left | right)
}

/// Returns pawns with no pawns in the neighbouring files
pub fn isolated_pawns(pawns: u64) -> u64 {
    let left = (pawns & !Board::A_FILE) >> 1;
    let right = (pawns & !Board::H_FILE) << 1;
    let files = populate_files(left | right);
    pawns & !files
}

/// Returns a bitboard containing pawns in the same files
pub fn doubled_pawns(pawns: u64) -> u64 {
    let populated = populate_files_up(pawns);
    let mut xor = pawns;
    xor ^= xor << 8;
    xor ^= xor << 16;
    xor ^= xor << 32;
    let files = populate_files(populated ^ xor);
    files & pawns
}

/// Computes the bonus centiscore determined by pawns
pub fn pawn_bonus(board: &Board) -> i16 {
    let mut bonus = 0;
    let white = board.pieces[Piece::WhitePawn as usize];
    let black = board.pieces[Piece::BlackPawn as usize];

    let white_passed = passed_pawns(white, black, Side::White);
    let white_friends = friend_pawns(white);
    let white_isolated = isolated_pawns(white);
    let white_doubled = doubled_pawns(white);

    let black_passed = passed_pawns(black, white, Side::Black);
    let black_friends = friend_pawns(black);
    let black_isolated = isolated_pawns(black);
    let black_doubled = doubled_pawns(black);

    const RANK2: u64 = Board::RANK_2;
    const RANK3: u64 = RANK2 << 8;
    const RANK4: u64 = RANK2 << 16;
    const RANK5: u64 = RANK2 << 24;
    const RANK6: u64 = RANK2 << 32;
    const RANK7: u64 = RANK2 << 40;

    bonus += (white_friends.count_ones() as i16 - black_friends.count_ones() as i16) * FRIEND_BONUS;
    bonus += (white_isolated.count_ones() as i16 - black_isolated.count_ones() as i16) * ISOLATED_BONUS;
    bonus += (white_doubled.count_ones() as i16 - black_doubled.count_ones() as i16) * DOUBLED_BONUS;
    bonus += ((white_passed & RANK2).count_ones() as i16 - (black_passed & RANK7).count_ones() as i16) * RANK_BONUS[0];
    bonus += ((white_passed & RANK3).count_ones() as i16 - (black_passed & RANK6).count_ones() as i16) * RANK_BONUS[1];
    bonus += ((white_passed & RANK4).count_ones() as i16 - (black_passed & RANK5).count_ones() as i16) * RANK_BONUS[2];
    bonus += ((white_passed & RANK5).count_ones() as i16 - (black_passed & RANK4).count_ones() as i16) * RANK_BONUS[3];
    bonus += ((white_passed & RANK6).count_ones() as i16 - (black_passed & RANK3).count_ones() as i16) * RANK_BONUS[4];
    bonus += ((white_passed & RANK7).count_ones() as i16 - (black_passed & RANK2).count_ones() as i16) * RANK_BONUS[5];

    bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::{Board, Piece, Side};

    fn print_pawn_board(label: &str, white: u64, black: u64, highlighted: u64) {
        println!("\n{}", label);
        println!("  a b c d e f g h");
        for rank in (0..8).rev() {
            print!("{} ", rank + 1);
            for file in 0..8 {
                let bit = 1u64 << (rank * 8 + file);
                let ch = if highlighted & bit != 0 {
                    'X'
                } else if white & bit != 0 {
                    'P'
                } else if black & bit != 0 {
                    'p'
                } else {
                    '.'
                };
                print!("{} ", ch);
            }
            println!();
        }
    }

    #[test]
    fn test_passed_pawns() {
        // White: a6 (passed), d4 (blocked by d7), e4 (blocked by e7) — Black: d7, e7
        let board = Board::from_fen("4k3/3pp3/P7/8/3PP3/8/8/4K3 w - - 0 20");
        let white = board.pieces[Piece::WhitePawn as usize];
        let black = board.pieces[Piece::BlackPawn as usize];
        let passed = passed_pawns(white, black, Side::White);
        print_pawn_board("Passed pawns (X = passed, P = not passed, p = enemy):", white, black, passed);
        assert!(passed & (1 << 40) != 0, "a6 should be passed");
        assert!(passed & (1 << 27) == 0, "d4 blocked by d7");
        assert!(passed & (1 << 28) == 0, "e4 blocked by e7");
    }

    #[test]
    fn test_friend_pawns() {
        // White: a4 (isolated), d4 (friends with e4), e4 (friends with d4)
        let board = Board::from_fen("4k3/8/8/8/P2PP3/8/8/4K3 w - - 0 20");
        let white = board.pieces[Piece::WhitePawn as usize];
        let friends = friend_pawns(white);
        print_pawn_board("Friend pawns (X = has friend, P = isolated):", white, 0, friends);
        assert!(friends & (1 << 24) == 0, "a4 is isolated");
        assert!(friends & (1 << 27) != 0, "d4 has friend e4");
        assert!(friends & (1 << 28) != 0, "e4 has friend d4");
    }

    #[test]
    fn test_doubled_pawns() {
        // White: d4 (not doubled), e3 and e5 (doubled pair)
        let board = Board::from_fen("4k3/8/8/4P3/3P4/4P3/8/4K3 w - - 0 20");
        let white = board.pieces[Piece::WhitePawn as usize];
        let doubled = doubled_pawns(white);
        print_pawn_board("Doubled pawns (X = doubled, P = not doubled):", white, 0, doubled);
        assert!(doubled & (1 << 27) == 0, "d4 is not doubled");
        assert!(doubled & (1 << 20) != 0, "e3 is doubled");
        assert!(doubled & (1 << 36) != 0, "e5 is doubled");
    }
}