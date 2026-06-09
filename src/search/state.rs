use std::sync::Arc;

use crate::{eval::Eval, movegen::r#move::Move, search::{MAX_PLY, tt::TT}};

pub const MAX_HISTORY_VALUE: Eval = 16384;

#[derive(Clone)]
pub struct SearchState {
    pub tt: Arc<TT>,
    pub killers: [[Option<Move>; 2]; MAX_PLY as usize],
    pub history: [[Eval; 64]; 64],
    pub counter_move: [[Option<Move>; 64]; 64],
    pub node_count: u64,
}

impl SearchState {
    pub fn new(tt_bits: u8, tt_generation_cutoff: u8) -> Self {
        SearchState {
            tt: Arc::new(TT::new(tt_bits, tt_generation_cutoff)),
            killers: [[None; 2]; MAX_PLY as usize],
            history: [[0; 64]; 64],
            counter_move: [[None; 64]; 64],
            node_count: 0
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
        self.node_count = 0
    }

    pub fn new_helper_thread(&self) -> Self {
        SearchState {
            tt: Arc::clone(&self.tt),
            killers: self.killers.clone(),
            history: self.history.clone(),
            counter_move: self.counter_move.clone(),
            node_count: 0
        }
    }

    pub fn beta_cutoff(&mut self, mv: Move, prev_move: Option<Move>, depth: u8, ply: u8, tried_quiets: &[Move]) {
        if mv.is_capture() {
            return;
        }
        if self.killers[ply as usize][0] != Some(mv) {
            self.killers[ply as usize][1] = self.killers[ply as usize][0];
            self.killers[ply as usize][0] = Some(mv);
        }
        if let Some(prev) = prev_move {
            self.counter_move[prev.source_square() as usize][prev.target_square() as usize] = Some(mv);
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
    }
}