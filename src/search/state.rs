use std::sync::{Arc, atomic::AtomicBool};

use crate::{eval::pawnking::PawnTable, movegen::r#move::{MAX_HISTORY_VALUE, Move, MoveScore}, repr::{board::Board, hash::Hash}, search::{MAX_PLY, tt::{TT, TTEntry}}};

#[derive(Clone)]
pub struct SearchState {
    pub tt: Arc<TT<true>>,
    pub local_tt: TT<false>,
    pub pawn_table: Arc<PawnTable>,
    pub killers: [[Option<Move>; 2]; MAX_PLY as usize],
    pub history: History,
    pub counter_move: [[Option<Move>; 64]; 64],
    pub max_depth: u8,
    pub node_count: u64,
    pub stop_search: bool,
    pub is_main: bool,
    pub fake_stop_flag: Option<Arc<AtomicBool>>,
}

impl SearchState {
    pub fn new_default() -> Self {
        SearchState::new(23, 2, 20)
    }

    pub fn new(tt_bits: u8, tt_generation_cutoff: u8, pawn_table_bits: u8) -> Self {
        SearchState {
            tt: Arc::new(TT::new(tt_bits, tt_generation_cutoff)),
            local_tt: TT::new(tt_bits - 3, tt_generation_cutoff),
            pawn_table: Arc::new(PawnTable::new(pawn_table_bits)),
            killers: [[None; 2]; MAX_PLY as usize],
            history: History::new(),
            counter_move: [[None; 64]; 64],
            max_depth: 0,
            node_count: 0,
            stop_search: false,
            is_main: true,
            fake_stop_flag: None
        }
    }

    pub fn new_search(&mut self) {
        let tt = Arc::get_mut(&mut self.tt).expect("new_search() called with multiple active TTs");
        tt.new_search();
        self.local_tt.new_search();
        for i in 1..self.killers.len() {
            self.killers[i - 1] = self.killers[i]
        }
        self.killers[self.killers.len() - 1] = [None, None];
        self.history.new_search();
        self.counter_move = [[None; 64]; 64];
        self.node_count = 0;
        self.stop_search = false;
    }

    pub fn new_helper_thread(&self) -> Self {
        SearchState {
            tt: Arc::clone(&self.tt),
            local_tt: self.local_tt.clone(),
            pawn_table: Arc::clone(&self.pawn_table),
            killers: self.killers.clone(),
            history: self.history.clone(),
            counter_move: self.counter_move.clone(),
            max_depth: self.max_depth,
            node_count: 0,
            stop_search: false,
            is_main: false,
            fake_stop_flag: None
        }
    }

    pub fn beta_cutoff(&mut self, board: &Board, mv: Move, depth: u8, ply: u8, tried_quiets: &[Move]) {
        if mv.is_capture() {
            return;
        }
        if self.killers[ply as usize][0] != Some(mv) {
            self.killers[ply as usize][1] = self.killers[ply as usize][0];
            self.killers[ply as usize][0] = Some(mv);
        }
        self.history.beta_cutoff(board, mv, depth, tried_quiets);
    }

    // returns the better tt entry and the other tt's best move
    pub fn find_tt(&self, hash: Hash) -> (Option<&TTEntry>, Option<Move>) {
        let shared = self.tt.find(hash);
        let local = self.local_tt.find(hash);
        let depth_shared = shared.map(|entry| entry.depth).unwrap_or(0);
        let depth_local = local.map(|entry| entry.depth).unwrap_or(0);
        if depth_shared >= depth_local {
            (shared, local.map(|entry| entry.best_move).unwrap_or(None))
        } else {
            (local, shared.map(|entry| entry.best_move).unwrap_or(None))
        }
    }

    pub fn insert_tt(&mut self, entry: TTEntry, ply: u8) {
        self.tt.insert(entry.clone(), ply);
        self.local_tt.insert(entry, ply);
    }
}

type Hist<T> = [[T; 64]; 12];


#[derive(Clone)]
pub struct History {
    history: Hist<MoveScore>,
    continuation: Box<Hist<Hist<MoveScore>>>,
    follow_up: Box<Hist<Hist<MoveScore>>>,
}

impl History {
    pub fn new() -> Self {
        History {
            history: [[0; 64]; 12],
            continuation: [[[[0; 64]; 12]; 64]; 12].into(),
            follow_up: [[[[0; 64]; 12]; 64]; 12].into(),
        }
    }

    pub fn new_search(&mut self) {
        for entry in self.history.iter_mut().flatten() {
            *entry /= 2;
        }
        for entry in self.continuation.iter_mut().flatten().flatten().flatten() {
            *entry /= 2;
        }
        for entry in self.follow_up.iter_mut().flatten().flatten().flatten() {
            *entry /= 2;
        }
    }

    pub fn get(&self, board: &Board, mv: Move) -> MoveScore {
        let mut bonus: i32 = 0;
        let piece = board[mv.source_square()].unwrap();
        let dest = mv.target_square();
        bonus += self.history[piece][dest] as i32;
        let len = board.move_history.len();
        if len == 0 {
            return bonus as MoveScore;
        }
        let (prev_mv, prev_piece) = board.move_history[len - 1];
        if prev_mv == Move::NULL_MOVE {
            return bonus as MoveScore;
        }
        let prev_piece = prev_piece.unwrap();
        let prev_dest = prev_mv.target_square();
        bonus += self.continuation[prev_piece][prev_dest][piece][dest] as i32;
        if len == 1 {
            return (bonus / 2) as MoveScore;
        }
        let (prev_prev_mv, prev_prev_piece) = board.move_history[len - 2];
        if prev_prev_mv == Move::NULL_MOVE {
            return (bonus / 2) as MoveScore;
        }
        let prev_prev_piece = prev_prev_piece.unwrap();
        let prev_prev_dest = prev_prev_mv.target_square();
        bonus += self.follow_up[prev_prev_piece][prev_prev_dest][piece][dest] as i32;

        (bonus / 3) as MoveScore
    }

    pub fn beta_cutoff(&mut self, board: &Board, mv: Move, depth: u8, tried_quiets: &[Move]) {
        let piece = board[mv.source_square()].unwrap();
        let dest = mv.target_square();
        let bonus = depth as MoveScore * depth as MoveScore;
        History::update_entry(&mut self.history[piece][dest], bonus / 4, true);

        let len = board.move_history.len();
        if len == 0 {
            for tried_mv in tried_quiets {
                let tried_piece = board[tried_mv.source_square()].unwrap();
                let tried_dest = tried_mv.target_square();
                History::update_entry(&mut self.history[tried_piece][tried_dest], bonus / 4, false);
            }
            return;
        }
        let (prev_mv, prev_piece) = board.move_history[len - 1];
        if prev_mv == Move::NULL_MOVE {
            for tried_mv in tried_quiets {
                let tried_piece = board[tried_mv.source_square()].unwrap();
                let tried_dest = tried_mv.target_square();
                History::update_entry(&mut self.history[tried_piece][tried_dest], bonus / 4, false);
            }
            return;
        }
        let prev_piece = prev_piece.unwrap();
        let prev_dest = prev_mv.target_square();
        History::update_entry(&mut self.continuation[prev_piece][prev_dest][piece][dest], bonus, true);
        if len == 1 {
            for tried_mv in tried_quiets {
                let tried_piece = board[tried_mv.source_square()].unwrap();
                let tried_dest = tried_mv.target_square();
                History::update_entry(&mut self.history[tried_piece][tried_dest], bonus / 4, false);
                History::update_entry(&mut self.continuation[prev_piece][prev_dest][tried_piece][tried_dest], bonus, false);
            }
            return;
        }
        let (prev_prev_mv, prev_prev_piece) = board.move_history[len - 2];
        if prev_prev_mv == Move::NULL_MOVE {
            for tried_mv in tried_quiets {
                let tried_piece = board[tried_mv.source_square()].unwrap();
                let tried_dest = tried_mv.target_square();
                History::update_entry(&mut self.history[tried_piece][tried_dest], bonus / 4, false);
                History::update_entry(&mut self.continuation[prev_piece][prev_dest][tried_piece][tried_dest], bonus, false);
            }
            return;
        }
        let prev_prev_piece = prev_prev_piece.unwrap();
        let prev_prev_dest = prev_prev_mv.target_square();
        History::update_entry(&mut self.follow_up[prev_prev_piece][prev_prev_dest][piece][dest], bonus / 2, true);
        for tried_mv in tried_quiets {
            let tried_piece = board[tried_mv.source_square()].unwrap();
            let tried_dest = tried_mv.target_square();
            History::update_entry(&mut self.history[tried_piece][tried_dest], bonus / 4, false);
            History::update_entry(&mut self.continuation[prev_piece][prev_dest][tried_piece][tried_dest], bonus, false);
            History::update_entry(&mut self.follow_up[prev_prev_piece][prev_prev_dest][tried_piece][tried_dest], bonus / 2, false);
        }
    }

    #[inline]
    fn update_entry(entry: &mut MoveScore, bonus: MoveScore, positive: bool) {
        if positive {
            *entry += bonus - (*entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
        } else {
            *entry += -bonus - (*entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
        }
    }
}