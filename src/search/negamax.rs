use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::{eval::Eval, movegen::r#move::Move, repr::board::{self, Board}, search::{MAX_PLY, state::SearchState, tt::{TTEntry, TTFlag, adjust_retrieve_eval}}};

pub fn search(board: &mut Board, search_state: &mut SearchState, depth: u8, stop_flag: &Arc<AtomicBool>) -> (Option<Move>, u8) {
    panic!()
}

pub fn negamax(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, depth: u8, ply: u8, mut alpha: Eval, mut beta: Eval) -> (Option<Move>, Eval) {
    state.node_count += 1;

    // if stop flag is set, stop the search
    if state.node_count % (1 << 12) == 0 {
        if stop_flag.load(Ordering::Relaxed) {
            return (None, 0);
        }
    }
    
    let is_pv = beta > alpha + 1;

    // leaf node. Get quiescence score for a more accurate/stable score
    if depth == 0 || ply >= MAX_PLY {
        return (None, quiescence(stop_flag, board, depth, ply, alpha, beta));
    }

    // TT lookup. Store the best move and exit early if possible.
    let mut tt_move = None;
    if let Some(entry) = state.tt.find(board.state.hash) {
        tt_move = entry.best_move;
        let eval = adjust_retrieve_eval(entry.eval, ply);
        if entry.depth >= depth && board.state.repetitions == 1 && board.halfmove_clock < 90 && !is_pv {
            match entry.flag {
                TTFlag::Exact => return (tt_move, eval),
                TTFlag::LowerBound => alpha = alpha.max(eval),
                TTFlag::UpperBound => beta = beta.min(eval),
            }
            if alpha >= beta {
                return (tt_move, eval);
            }
        }
    }

    let original_alpha = alpha;

    if let Some(mv) = tt_move {
        let unmake_info = board.makemove(mv);
        let score = -negamax(stop_flag, board, state, depth - 1, ply + 1, -beta, -alpha).1;
        board.unmakemove(mv, score, unmake_info, None);
        alpha = alpha.max(score);
        if alpha >= beta {
            state.beta_cutoff(mv, board.move_history.last().copied(), depth, ply, &[]);
            let tt_entry = TTEntry::new(board.state.hash, score, TTFlag::LowerBound, tt_move, depth, state.tt.generation());
            state.tt.insert(tt_entry);
            return (tt_move, score)
        }
    }
    panic!()
}

pub fn quiescence(stop_flag: &Arc<AtomicBool>, board: &mut Board, depth: u8, ply: u8, alpha: Eval, beta: Eval) -> Eval {

    panic!()
}