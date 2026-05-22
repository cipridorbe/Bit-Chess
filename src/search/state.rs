use crate::{movegen::r#move::Move, search::{tt::TT}};

pub struct SearchState<'a> {
    pub(crate) killers: [[Option<Move>; 2]; 32],
    pub(crate) tt: &'a mut TT,
    pub(crate) history: &'a mut [[i16; 64]; 64],
    pub(crate) max_depth: u8
}



impl<'a> SearchState<'a> {
    pub fn new_search(tt: &'a mut TT, history: &'a mut [[i16; 64];64]) -> Self {
        tt.new_search();
        for i in 0..64 {
            for j in 0..64 {
                history[i][j] /= 2;
            }
        }
        SearchState {
            killers: [[None; 2]; 32],
            tt: tt,
            history: history,
            max_depth: 20
        }
    }

    pub fn beta_cutoff(&mut self, mv: Move, depth: u8, ply: u8) {
        if mv.is_capture() {
            return;
        }
        if self.killers[ply as usize][0] != Some(mv) {
            self.killers[ply as usize][1] = self.killers[ply as usize][0];
            self.killers[ply as usize][0] = Some(mv);
        }
        self.history[mv.source_square() as usize][mv.target_square() as usize] += (depth*depth) as i16;
    }
}