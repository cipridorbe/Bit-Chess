use std::cell::UnsafeCell;

use crate::{eval::Eval, movegen::attacks::pawn_attacks, repr::{bitboard::BB, board::Board, colour::Colour, hash::Hash, piece::Piece}, search::state::SearchState, test_assert, util::{populate_files, populate_files_down, populate_files_up}};

const FRIEND_BONUS: Eval = 10;
const ISOLATED_BONUS: Eval = -15;
const DOUBLED_BONUS: Eval = -10;
const RANK_BONUS: [Eval; 6] = [10, 20, 40, 70, 120, 200];
const PROTECTED_PASSED_PAWN_BONUS: Eval = 25;
const BACKWARD_PAWN_BONUS: Eval = -20;
const PAWN_ISLAND_BONUS: Eval = -10;

const MISSING_GUARD_BONUS: Eval = -20;
const KING_DISTANCE_BONUS: [Eval; 8] = [0, 0, 50, 40, 30, 20, 10, 0];


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

pub fn pawns_in_files(pawns: BB) -> u8 {
    let files = populate_files_down(pawns);
    (files.0 & 0xff) as u8
}

pub fn backward_pawns(my_pawns: BB, enemy_pawns: BB, colour: Colour) -> BB {
    let controlled = enemy_pawns | pawn_attacks(enemy_pawns, !colour);
    let defense = pawn_attacks(my_pawns, colour);
    match colour {
        Colour::White => (controlled >> 8) & my_pawns & !populate_files_up(defense),
        Colour::Black => (controlled << 8) & my_pawns & !populate_files_down(defense),
    }
}

pub fn pawn_islands_count(pawn_files: u8) -> u8 {
    let islands = pawn_files & ((!pawn_files << 1) | 1);
    islands.count_ones() as u8 
}

/// Computes the bonus centiscore determined by pawns
pub fn pawn_bonus(board: &Board, search_state: &mut SearchState) -> PawnTableEntry {
    if let Some(entry) = search_state.pawn_table.find(board.state.pawn_hash) {
        if entry.pawn_hash == board.state.pawn_hash {
            return entry;
        }
    }
    let mut bonus = 0;
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;
    let white = board[Piece::WhitePawn];
    let black = board[Piece::BlackPawn];
    let files = [pawns_in_files(white), pawns_in_files(black)];

    let white_passed = passed_pawns(white, black, Colour::White);
    let white_friends = friend_pawns(white);
    let white_isolated = isolated_pawns(white);
    let white_doubled = doubled_pawns(white);
    let white_islands = pawn_islands_count(files[Colour::White as usize]);
    let white_backward = backward_pawns(white, black, Colour::White);
    let white_protected_passed = white_passed & pawn_attacks(white, Colour::White);

    let black_passed = passed_pawns(black, white, Colour::Black);
    let black_friends = friend_pawns(black);
    let black_isolated = isolated_pawns(black);
    let black_doubled = doubled_pawns(black);
    let black_islands = pawn_islands_count(files[Colour::Black as usize]);
    let black_backward = backward_pawns(black, white, Colour::Black);
    let black_protected_passed = black_passed & pawn_attacks(black, Colour::Black);

    const RANK2: BB = Board::RANK_2;
    const RANK3: BB = Board::RANK_3;
    const RANK4: BB = Board::RANK_4;
    const RANK5: BB = Board::RANK_5;
    const RANK6: BB = Board::RANK_6;
    const RANK7: BB = Board::RANK_7;

    bonus += (white_friends.count_ones() as Eval - black_friends.count_ones() as Eval) * FRIEND_BONUS;
    bonus += (white_isolated.count_ones() as Eval - black_isolated.count_ones() as Eval) * ISOLATED_BONUS;
    bonus += (white_doubled.count_ones() as Eval - black_doubled.count_ones() as Eval) * DOUBLED_BONUS;
    bonus += (white_islands.count_ones() as Eval - black_islands.count_ones() as Eval) * PAWN_ISLAND_BONUS;
    bonus += (white_backward.count_ones() as Eval - black_backward.count_ones() as Eval) * BACKWARD_PAWN_BONUS;
    bonus += (white_protected_passed.count_ones() as Eval - black_protected_passed.count_ones() as Eval) * PROTECTED_PASSED_PAWN_BONUS;
    eg_bonus += ((white_passed & RANK2).count_ones() as Eval - (black_passed & RANK7).count_ones() as Eval) * RANK_BONUS[0];
    eg_bonus += ((white_passed & RANK3).count_ones() as Eval - (black_passed & RANK6).count_ones() as Eval) * RANK_BONUS[1];
    eg_bonus += ((white_passed & RANK4).count_ones() as Eval - (black_passed & RANK5).count_ones() as Eval) * RANK_BONUS[2];
    eg_bonus += ((white_passed & RANK5).count_ones() as Eval - (black_passed & RANK4).count_ones() as Eval) * RANK_BONUS[3];
    eg_bonus += ((white_passed & RANK6).count_ones() as Eval - (black_passed & RANK3).count_ones() as Eval) * RANK_BONUS[4];
    eg_bonus += ((white_passed & RANK7).count_ones() as Eval - (black_passed & RANK2).count_ones() as Eval) * RANK_BONUS[5];

    let (mg_king, eg_king) = king_bonus(board[Piece::WhiteKing], board[Piece::WhitePawn], files[0], board[Piece::BlackKing], board[Piece::BlackPawn], files[1]);
    mg_bonus += mg_king;
    eg_bonus += eg_king;

    let entry = PawnTableEntry::new(board.state.pawn_hash, bonus + mg_bonus, bonus + eg_bonus, files);
    search_state.pawn_table.insert(entry);

    entry
}

pub fn king_bonus(white_king: BB, white_pawns: BB, white_files: u8, black_king: BB, black_pawns: BB, black_files: u8) -> (Eval, Eval) {
    let wk_square_file = white_king.lsb().file();
    let bk_square_file = black_king.lsb().file();
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;

    let white_missing_guards = missing_guards(white_king, white_pawns, Colour::White).count_ones() as Eval;
    let black_missing_guards = missing_guards(black_king, black_pawns, Colour::Black).count_ones() as Eval;
    mg_bonus += (white_missing_guards - black_missing_guards) * MISSING_GUARD_BONUS;

    let wkfile = FileStatus::from_files(white_files, black_files, wk_square_file);
    let bkfile = FileStatus::from_files(white_files, black_files, bk_square_file);
    mg_bonus += wkfile.king_bonus(Colour::White);
    mg_bonus += bkfile.king_bonus(Colour::Black);
    if wk_square_file != 0 {
        let status = FileStatus::from_files(white_files, black_files, wk_square_file - 1);
        mg_bonus += status.king_bonus(Colour::White);
    }
    if wk_square_file != 7 {
        let status = FileStatus::from_files(white_files, black_files, wk_square_file + 1);
        mg_bonus += status.king_bonus(Colour::White);
    }
    if bk_square_file != 0 {
        let status = FileStatus::from_files(white_files, black_files, bk_square_file - 1);
        mg_bonus += status.king_bonus(Colour::Black);
    }
    if bk_square_file != 7 {
        let status = FileStatus::from_files(white_files, black_files, bk_square_file + 1);
        mg_bonus += status.king_bonus(Colour::Black);
    }
    (mg_bonus, eg_bonus)
}

pub fn missing_guards(king: BB, pawns: BB, colour: Colour) -> BB {
    let next = king | (king & !Board::A_FILE) >> 1 | (king & !Board::H_FILE) << 1;
    let guards = if colour == Colour::White { next << 8 } else { next >> 8 };
    guards & !pawns
}

#[derive(Clone, Copy)]
pub struct PawnTableEntry {
    pub pawn_hash: Hash,
    pub mg_eval: Eval,
    pub eg_eval: Eval,
    pub files: [u8; 2]
}

pub struct PawnTable {
    table: Vec<UnsafeCell<PawnTableEntry>>,
    mask: u64,
}

unsafe impl Sync for PawnTable {}
unsafe impl Send for PawnTable {}

impl PawnTableEntry {
    pub fn empty() -> Self {
        PawnTableEntry::new(unsafe { std::mem::transmute(0u64) }, 0, 0, [0, 0])
    }
    
    pub fn new(pawn_hash: Hash, mg_eval: Eval, eg_eval: Eval, files: [u8; 2]) -> Self {
        PawnTableEntry{ pawn_hash, mg_eval, eg_eval, files }
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

    pub fn insert(&self, mut pawn_table_entry: PawnTableEntry) {
        let idx = pawn_table_entry.pawn_hash.0 & self.mask;
        let current = unsafe { &mut *self.table[idx as usize].get() };
        let original_hash = pawn_table_entry.pawn_hash;
        pawn_table_entry.pawn_hash = Hash(0);
        *current = pawn_table_entry;
        current.pawn_hash = original_hash;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Open,
    WhiteOnly,
    BlackOnly,
    Closed
}

impl FileStatus {
    const FILE_STATUS_BONUS_WHITE: [Eval; 4] = [-40, 0, -20, 10];
    const FILE_STATUS_BONUS_BLACK: [Eval; 4] = [40, 20, 0, -10];

    pub fn new(white_file: bool, black_file: bool) -> Self {
        unsafe { std::mem::transmute(((black_file as u8) << 1) | white_file as u8) }
    }

    pub fn from_files(white_files: u8, black_files: u8, file: u8) -> Self {
        test_assert!(file < 8);
        let mask = 1 << file;
        FileStatus::new(white_files & mask != 0, black_files & mask != 0)
    }
 
    pub fn king_bonus(self, colour: Colour) -> Eval {
        match colour {
            Colour::White => FileStatus::FILE_STATUS_BONUS_WHITE[self as usize],
            Colour::Black => FileStatus::FILE_STATUS_BONUS_BLACK[self as usize],
        }
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