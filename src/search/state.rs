use crate::{movegen::r#move::Move, search::{tt::TT}};

const MAX_HISTORY: i16 = 8192;

pub struct SearchState<'a> {
    pub(crate) killers: [[Option<Move>; 2]; 48],
    pub(crate) tt: &'a mut TT,
    pub(crate) history: &'a mut [[i16; 64]; 64],
    pub(crate) counter_move: &'a mut[[Option<Move>; 64]; 64],
    pub(crate) max_depth: u8
}



impl<'a> SearchState<'a> {
    pub fn new_search(tt: &'a mut TT, history: &'a mut [[i16; 64];64], counter_move: &'a mut [[Option<Move>; 64]; 64]) -> Self {
        tt.new_search();
        for i in 0..64 {
            for j in 0..64 {
                history[i][j] /= 2;
            }
        }
        SearchState {
            killers: [[None; 2]; 48],
            tt: tt,
            history: history,
            counter_move: counter_move,
            max_depth: 20
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
            += bonus - (entry as i32 * bonus as i32 / MAX_HISTORY as i32) as i16;
        for quiet in tried_quiets {
            let entry = self.history[quiet.source_square() as usize][quiet.target_square() as usize];
            self.history[quiet.source_square() as usize][quiet.target_square() as usize]
                += -bonus - (entry as i32 * bonus as i32 / MAX_HISTORY as i32) as i16;
        }
    }
}