use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::{eval::{Eval, INF, MATE_CUTOFF}, movegen::r#move::Move, repr::board::{self, Board}, search::{MAX_PLY, state::SearchState, tt::{TTEntry, TTFlag, adjust_retrieve_eval}}};

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
    let in_check = board.in_check();

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

    // If we haven't cut off, we have to start exploring more moves
    let mut movelist = board.generate_movelist(false);
    let mut scores = movelist.score(board, state, tt_move, ply);
    movelist.sort(&mut scores);

    let mut best_move = None;
    let mut best_score = -INF;
    let mut moved = false;
    let mut tried_quiets = [Move::NULL_MOVE; 218];
    let mut tried_quiets_idx = 0;
    let mut i = if tt_move.is_none() { 0 } else { 1 };
    while i < movelist.length {
        let mv = movelist[i];
        let mv_score = scores[i];
        i += 1;
        if !board.is_legal(mv) {
            continue;
        }
        let unmake_info = board.makemove(mv);
        moved = true;
        let score = if board.is_rule_draw() { 0 } else {
            // PVS: search the first move normally. Search the rest with a null window and re-search if it beats alpha
            let pv_search_depth = if in_check && ply + depth < state.max_depth { depth } else { depth - 1 };
            if i == 0 {
                -negamax(stop_flag, board, state, pv_search_depth, ply + 1, -beta, -alpha, true).1
            } else {
                let reduced_search_depth = depth - 1;
                if depth >= 3 && tried_quiets_idx > 3 && !in_check && ply > 1 && !mv.is_queen_promotion() {

                }
                0
            }
        };
        
    }

    return (best_move, best_score);
}

pub fn quiescence(stop_flag: &Arc<AtomicBool>, board: &mut Board, depth: u8, ply: u8, alpha: Eval, beta: Eval) -> Eval {

    panic!()
}

const fn build_lmr_table() -> [[u8; 64]; 64] {
    let mut table = [[0; 64]; 64];

    let mut d = 0;
    while d < 64 {
        let mut i = 0;
        while i < 64 {
            if d == 0 || i == 0 { continue; }
            let r = 0.5 + (f64::ln(d as f64) + f64::ln(i as f64)) / 2.25;

            i += 1;
        }
        d += 1;
    }

    table
}