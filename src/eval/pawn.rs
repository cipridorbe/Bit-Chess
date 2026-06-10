use std::cell::UnsafeCell;

use crate::{eval::Eval, movegen::attacks::pawn_attacks, repr::{bitboard::BB, board::Board, colour::Colour, hash::Hash, piece::Piece}, search::state::SearchState, util::{populate_files, populate_files_down, populate_files_up}};

const FRIEND_BONUS: Eval = 10;
const ISOLATED_BONUS: Eval = -15;
const DOUBLED_BONUS: Eval = -15;
const RANK_BONUS: [Eval; 6] = [10, 20, 40, 70, 120, 200];

/// Returns a bitboard of the passed pawns of the current side to play
pub fn passed_pawns(my_pawns: BB, enemy_pawns: BB, colour: Colour) -> BB {
    if enemy_pawns.count_ones() >= 8 || my_pawns == 0 {
        return BB::new(0);
    }
    let pawn_files = enemy_pawns | pawn_attacks(enemy_pawns, !colour);
    let blocked = if colour == Colour::White {
        populate_files_down(pawn_files)
    } else {
        populate_files_up(pawn_files)
    };
    my_pawns & !blocked
}

/// Returns a bitboard containing pawns with at least one pawn next to them
pub fn friend_pawns(pawns: BB) -> BB {
    let mut left = (pawns & !Board::A_FILE) >> 1;
    let mut right = (pawns & !Board::H_FILE) << 1;
    left |= left << 8;
    left |= left >> 8;
    right |= right << 8;
    right |= right >> 8;
    pawns & (left | right)
}

/// Returns pawns with no pawns in the neighbouring files
pub fn isolated_pawns(pawns: BB) -> BB {
    let left = (pawns & !Board::A_FILE) >> 1;
    let right = (pawns & !Board::H_FILE) << 1;
    let files = populate_files(left | right);
    pawns & !files
}

/// Returns a bitboard containing pawns in the same files
pub fn doubled_pawns(pawns: BB) -> BB {
    let populated = populate_files_up(pawns);
    let mut xor = pawns;
    xor ^= xor << 8;
    xor ^= xor << 16;
    xor ^= xor << 32;
    let files = populate_files(populated ^ xor);
    files & pawns
}

/// Computes the bonus centiscore determined by pawns
pub fn pawn_bonus(board: &Board, search_state: &mut SearchState) -> (Eval, Eval) {
    if let Some(entry) = search_state.pawn_table.find(board.state.pawn_hash) {
        if entry.pawn_hash == board.state.pawn_hash {
            return (entry.mg_eval, entry.eg_eval);
        }
    }
    let mut bonus = 0;
    let white = board[Piece::WhitePawn];
    let black = board[Piece::BlackPawn];

    let white_passed = passed_pawns(white, black, Colour::White);
    let white_friends = friend_pawns(white);
    let white_isolated = isolated_pawns(white);
    let white_doubled = doubled_pawns(white);

    let black_passed = passed_pawns(black, white, Colour::Black);
    let black_friends = friend_pawns(black);
    let black_isolated = isolated_pawns(black);
    let black_doubled = doubled_pawns(black);

    const RANK2: BB = Board::RANK_2;
    const RANK3: BB = Board::RANK_3;
    const RANK4: BB = Board::RANK_4;
    const RANK5: BB = Board::RANK_5;
    const RANK6: BB = Board::RANK_6;
    const RANK7: BB = Board::RANK_7;

    bonus += (white_friends.count_ones() as Eval - black_friends.count_ones() as Eval) * FRIEND_BONUS;
    bonus += (white_isolated.count_ones() as Eval - black_isolated.count_ones() as Eval) * ISOLATED_BONUS;
    bonus += (white_doubled.count_ones() as Eval - black_doubled.count_ones() as Eval) * DOUBLED_BONUS;
    bonus += ((white_passed & RANK2).count_ones() as Eval - (black_passed & RANK7).count_ones() as Eval) * RANK_BONUS[0];
    bonus += ((white_passed & RANK3).count_ones() as Eval - (black_passed & RANK6).count_ones() as Eval) * RANK_BONUS[1];
    bonus += ((white_passed & RANK4).count_ones() as Eval - (black_passed & RANK5).count_ones() as Eval) * RANK_BONUS[2];
    bonus += ((white_passed & RANK5).count_ones() as Eval - (black_passed & RANK4).count_ones() as Eval) * RANK_BONUS[3];
    bonus += ((white_passed & RANK6).count_ones() as Eval - (black_passed & RANK3).count_ones() as Eval) * RANK_BONUS[4];
    bonus += ((white_passed & RANK7).count_ones() as Eval - (black_passed & RANK2).count_ones() as Eval) * RANK_BONUS[5];

    let entry = PawnTableEntry::new(board.state.pawn_hash, bonus, bonus);
    search_state.pawn_table.insert(entry);

    (bonus, bonus)
}

#[derive(Clone, Copy)]
pub struct PawnTableEntry {
    pawn_hash: Hash,
    mg_eval: Eval,
    eg_eval: Eval,
}

pub struct PawnTable {
    table: Vec<UnsafeCell<PawnTableEntry>>,
    mask: u64,
}

unsafe impl Sync for PawnTable {}
unsafe impl Send for PawnTable {}

impl PawnTableEntry {
    pub fn empty() -> Self {
        PawnTableEntry::new(unsafe { std::mem::transmute(0u64) }, 0, 0)
    }
    
    pub fn new(pawn_hash: Hash, mg_eval: Eval, eg_eval: Eval) -> Self {
        PawnTableEntry{ pawn_hash, mg_eval, eg_eval }
    }
}

impl PawnTable {
    pub fn new(bits: u8) -> Self {
        let length = 1 << bits;
        let mut table = Vec::with_capacity(length);
        for _ in 0..length {
            table.push(UnsafeCell::new(PawnTableEntry::empty()));
        }
        PawnTable {
            table: table,
            mask: length as u64 - 1,
        }
    }

    pub fn find(&self, pawn_hash: Hash) -> Option<PawnTableEntry> {
        let idx = pawn_hash.0 & self.mask;
        let entry = unsafe { *self.table[idx as usize].get() } ;
        if entry.pawn_hash.0 == 0 {
            None
        } else {
            Some(entry)
        }
    }

    pub fn insert(&self, pawn_table_entry: PawnTableEntry) {
        let idx = pawn_table_entry.pawn_hash.0 & self.mask;
        let current = unsafe { &mut *self.table[idx as usize].get() };
        *current = pawn_table_entry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repr::{board::Board, piece::Piece, colour::Colour};

    fn print_pawn_board(label: &str, white: BB, black: BB, highlighted: BB) {
        println!("\n{}", label);
        println!("  a b c d e f g h");
        for rank in (0..8).rev() {
            print!("{} ", rank + 1);
            for file in 0..8 {
                let bit = 1u64 << (rank * 8 + file);
                let ch = if highlighted.0 & bit != 0 {
                    'X'
                } else if white.0 & bit != 0 {
                    'P'
                } else if black.0 & bit != 0 {
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
        let passed = passed_pawns(white, black, Colour::White);
        print_pawn_board("Passed pawns (X = passed, P = not passed, p = enemy):", white, black, passed);
        assert!(passed.0 & (1 << 40) != 0, "a6 should be passed");
        assert!(passed.0 & (1 << 27) == 0, "d4 blocked by d7");
        assert!(passed.0 & (1 << 28) == 0, "e4 blocked by e7");
    }

    #[test]
    fn test_friend_pawns() {
        // White: a4 (isolated), d4 (friends with e4), e4 (friends with d4)
        let board = Board::from_fen("4k3/8/8/8/P2PP3/8/8/4K3 w - - 0 20");
        let white = board.pieces[Piece::WhitePawn as usize];
        let friends = friend_pawns(white);
        print_pawn_board("Friend pawns (X = has friend, P = isolated):", white, BB::new(0), friends);
        assert!(friends.0 & (1 << 24) == 0, "a4 is isolated");
        assert!(friends.0 & (1 << 27) != 0, "d4 has friend e4");
        assert!(friends.0 & (1 << 28) != 0, "e4 has friend d4");
    }

    #[test]
    fn test_doubled_pawns() {
        // White: d4 (not doubled), e3 and e5 (doubled pair)
        let board = Board::from_fen("4k3/8/8/4P3/3P4/4P3/8/4K3 w - - 0 20");
        let white = board.pieces[Piece::WhitePawn as usize];
        let doubled = doubled_pawns(white);
        print_pawn_board("Doubled pawns (X = doubled, P = not doubled):", white, BB::new(0), doubled);
        assert!(doubled.0 & (1 << 27) == 0, "d4 is not doubled");
        assert!(doubled.0 & (1 << 20) != 0, "e3 is doubled");
        assert!(doubled.0 & (1 << 36) != 0, "e5 is doubled");
    }
}