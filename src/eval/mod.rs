use crate::{repr::{board::Board, colour::Colour}, search::MAX_PLY};

pub mod pst;
pub mod pawn;
pub mod king;

pub type Eval = i16;

pub const INF: Eval = 31000;
pub const MATE: Eval = INF - 1;
pub const MATE_CUTOFF: Eval = MATE - MAX_PLY as Eval * 2;

pub const EVAL_BONUS_DELTA: Eval = 200;

pub fn partial_relative_eval(board: &Board, alpha: Eval, beta: Eval) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    if partial_eval <= alpha - EVAL_BONUS_DELTA || partial_eval >= beta + EVAL_BONUS_DELTA {
        mult * partial_eval
    } else {
        mult * (partial_eval + bonus_eval(board))
    }
}

pub fn relative_eval(board: &Board) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    mult * (partial_eval + bonus_eval(board))
}

pub fn bonus_eval(board: &Board) -> Eval {
    0
}

fn phase_eval(phase_unbounded: u8, mg_eval: Eval, eg_eval: Eval) -> Eval {
    let phase = phase_unbounded.min(24) as i32;
    ((mg_eval as i32 * phase + eg_eval as i32 * (24 - phase)) / 24) as Eval
}