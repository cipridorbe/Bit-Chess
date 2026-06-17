use std::sync::{Arc, atomic::AtomicBool};

use crate::{eval::pawnking::PawnTable, movegen::r#move::{MAX_HISTORY_VALUE, Move, MoveScore}, repr::board::Board, search::{MAX_PLY, tt::TT}};

#[derive(Clone)]
pub struct SearchState {
    pub tt: Arc<TT>,
    pub pawn_table: Arc<PawnTable>,
    pub killers: [[Option<Move>; 2]; MAX_PLY as usize],
    pub history: [[MoveScore; 64]; 64],
    // [prev_pieces][prev_target][piece][target]
    pub continuation_history: Box<[[[[MoveScore; 64]; 12]; 64]; 12]>,
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
            pawn_table: Arc::new(PawnTable::new(pawn_table_bits)),
            killers: [[None; 2]; MAX_PLY as usize],
            history: [[0; 64]; 64],
            continuation_history: Box::new([[[[0; 64]; 12]; 64]; 12]),
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
        for i in 1..self.killers.len() {
            self.killers[i - 1] = self.killers[i]
        }
        self.killers[self.killers.len() - 1] = [None, None];
        for i in 0..64 {
            for j in 0..64 {
                self.history[i][j] /= 2;
            }
        }
        self.counter_move = [[None; 64]; 64];
        self.node_count = 0;
        self.stop_search = false;
    }

    pub fn new_helper_thread(&self) -> Self {
        SearchState {
            tt: Arc::clone(&self.tt),
            pawn_table: Arc::clone(&self.pawn_table),
            killers: self.killers.clone(),
            history: self.history.clone(),
            continuation_history: self.continuation_history.clone(),
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
        let bonus = (depth * depth) as i16;
        let entry = self.history[mv.source_square() as usize][mv.target_square() as usize];
        self.history[mv.source_square() as usize][mv.target_square() as usize]
            += bonus - (entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
        for quiet in tried_quiets {
            let entry = self.history[quiet.source_square() as usize][quiet.target_square() as usize];
            self.history[quiet.source_square() as usize][quiet.target_square() as usize]
                += -bonus - (entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
        }
        if let Some(prev) = board.move_history.last().copied() {
            if prev != Move::NULL_MOVE {
                self.counter_move[prev.source_square() as usize][prev.target_square() as usize] = Some(mv);
                let prev_piece = board[prev.target_square()].unwrap();
                let piece = board[mv.source_square()].unwrap();
                let entry = &mut self.continuation_history[prev_piece as usize][prev.target_square() as usize][piece as usize][mv.target_square() as usize];
                *entry += bonus - (*entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
                for quiet in tried_quiets {
                    let quiet_p = board[quiet.source_square()].unwrap();
                    let entry = &mut self.continuation_history[prev_piece as usize][prev.target_square() as usize][quiet_p as usize][quiet.target_square() as usize];
                    *entry += -bonus - (*entry as i32 * bonus as i32 / MAX_HISTORY_VALUE as i32) as i16;
                }
            }
        }
    }
}