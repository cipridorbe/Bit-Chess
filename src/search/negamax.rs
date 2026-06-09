use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::{eval::{Eval, MATE_CUTOFF}, movegen::r#move::Move, repr::board::{self, Board}, search::{MAX_PLY, state::SearchState, tt::{TTEntry, TTFlag, adjust_retrieve_eval}}};

pub fn search(board: &mut Board, search_state: &mut SearchState, depth: u8, stop_flag: &Arc<AtomicBool>) -> (Option<Move>, u8) {
    panic!()
}

pub fn negamax(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, depth: u8, ply: u8, mut alpha: Eval, mut beta: Eval, null_move_allowed: bool) -> (Option<Move>, Eval) {
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

    // Try to create a beta cutoff with a null move
    if null_move_allowed && ply > 1 && !board.in_check() && depth >= 4 && board.state.phase_unbounded > 0 && beta < MATE_CUTOFF && !is_pv {
        let null_unmake = board.null_makemove();
        let score = -negamax(stop_flag, board, state, depth / 2, ply + 1, -beta-1, -beta, false).1;
        board.null_unmakemove(null_unmake);
        if score >= beta {
            let tt_entry = TTEntry::new(board.state.hash, score, TTFlag::LowerBound, None, depth, state.tt.generation());
            state.tt.insert(tt_entry, ply);
            return (None, score)
        }
    }

    // If no tt move is found, find the best move by performing a shallower search
    // TODO: test if pv_node improves speed
    if tt_move.is_none() && depth >= 4 {
        let new_depth = depth / 2 + 1;
        tt_move = negamax(stop_flag, board, state, new_depth, ply + 1, -beta, -alpha, false).0;
    }

    // Try to create a beta cutoff by making the tt move first
    if let Some(mv) = tt_move {
        let unmake_info = board.makemove(mv);
        let score = -negamax(stop_flag, board, state, depth - 1, ply + 1, -beta, -alpha, true).1;
        board.unmakemove(mv, score, unmake_info, None);
        alpha = alpha.max(score);
        if alpha >= beta {
            state.beta_cutoff(mv, board.move_history.last().copied(), depth, ply, &[]);
            let tt_entry = TTEntry::new(board.state.hash, score, TTFlag::LowerBound, tt_move, depth, state.tt.generation());
            state.tt.insert(tt_entry, ply);
            return (tt_move, score)
        }
    }

    let mut movelist = board.generate_movelist(false);
    panic!()
}

pub fn quiescence(stop_flag: &Arc<AtomicBool>, board: &mut Board, depth: u8, ply: u8, alpha: Eval, beta: Eval) -> Eval {

    panic!()
}