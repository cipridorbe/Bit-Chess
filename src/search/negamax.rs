use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use once_cell::sync::Lazy;

use crate::{eval::{Eval, INF, MATE, MATE_CUTOFF, partial_relative_eval}, movegen::r#move::{Flag, Move}, repr::board::Board, search::{MAX_PLY, NUM_THREADS, state::SearchState, tt::{TTEntry, TTFlag, adjust_retrieve_eval}}};

pub const TIMEOUT_MOD: u64 = 1 << 12;

pub fn search(board: &mut Board, search_state: &mut SearchState, depth: u8, stop_flag: &Arc<AtomicBool>) -> (Option<Move>, Eval, u8, u64) {
    search_state.new_search();
    let mut threads = Vec::new();
    let fake_stop_flag = Arc::new(AtomicBool::new(false));
    for _ in 1..NUM_THREADS {
        let mut cloned_search_state = search_state.new_helper_thread();
        let mut cloned_board = board.clone();
        let cloned_stop_flag = Arc::clone(&fake_stop_flag);
        let thread = std::thread::spawn(move || {
            iterative_deepening(&mut cloned_board, &mut cloned_search_state, depth, &cloned_stop_flag)
        });
        threads.push(thread);
    }

    let (mv, eval, depth, mut nodes_visted) = iterative_deepening(board, search_state, depth, stop_flag);
    fake_stop_flag.store(true, Ordering::Relaxed);
    for thread in threads {
        if let Ok((_, _, _, nodes)) = thread.join() {
            nodes_visted += nodes;
        }
    }

    (mv, eval, depth, nodes_visted)
}

pub fn iterative_deepening(board: &mut Board, state: &mut SearchState, max_depth: u8, stop_flag: &Arc<AtomicBool>) -> (Option<Move>, Eval, u8, u64) {
    let mut best_move = None;
    let mut score = 0;
    let mut reached_depth = 0;
    for depth in 1..=max_depth {
        state.max_depth = depth + depth / 2;
        let (mv, current_score) = negamax(stop_flag, board, state, depth, 0, -INF, INF, false);
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        best_move = mv;
        score = current_score;
        reached_depth = depth;
    }
    (best_move, score, reached_depth, state.node_count)
}

pub fn negamax(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, depth: u8, ply: u8, mut alpha: Eval, mut beta: Eval, null_move_allowed: bool) -> (Option<Move>, Eval) {
    state.node_count += 1;

    // if stop flag is set, stop the search
    if state.node_count % TIMEOUT_MOD == 0 || depth > 5 {
        if stop_flag.load(Ordering::Relaxed) {
            return (None, 0);
        }
    }

    // leaf node. Get quiescence score for a more accurate/stable score
    if depth == 0 || ply >= MAX_PLY {
        return (None, quiescence(stop_flag, board, state, ply, alpha, beta));
    }

    let is_pv = beta > alpha + 1;
    let in_check = board.in_check();
    let full_search_depth = if in_check && ply + depth < state.max_depth { depth } else { depth - 1 };

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
        tt_move = negamax(stop_flag, board, state, new_depth, ply, alpha, beta, false).0;
    }

    let mut tried_quiets = [Move::NULL_MOVE; 218];
    let mut tried_quiets_idx = 0;
    let mut moved = false;
    let mut best_move = None;
    let mut best_score = -INF;
    let mut add_proms = false;

    // Try to create a beta cutoff by making the tt move first
    if let Some(mv) = tt_move {
        let unmake_info = board.makemove(mv);
        moved = true;
        let score = -negamax(stop_flag, board, state, full_search_depth, ply + 1, -beta, -alpha, true).1;
        add_proms = board.unmakemove(mv, score, unmake_info, None);
        alpha = alpha.max(score);
        if alpha >= beta {
            state.beta_cutoff(mv, board.move_history.last().copied(), depth, ply, &[]);
            let tt_entry = TTEntry::new(board.state.hash, score, TTFlag::LowerBound, tt_move, depth, state.tt.generation());
            state.tt.insert(tt_entry, ply);
            return (tt_move, score)
        }
        if !mv.is_capture() {
            tried_quiets[tried_quiets_idx] = mv;
            tried_quiets_idx += 1;
        }
        best_score = score;
        best_move = tt_move;
    }

    // If we haven't cut off, we have to start exploring more moves
    let mut movelist = board.generate_movelist(false);
    let mut scores = movelist.score(board, state, tt_move, ply);
    movelist.sort(&mut scores);
    if add_proms {
        let prom = tt_move.unwrap();
        if prom.is_capture() {
            movelist.add(Move::new(Flag::ROOKPROMCAP, prom.target_square(), prom.source_square()));
            movelist.add(Move::new(Flag::BISHOPPROMCAP, prom.target_square(), prom.source_square()));
        } else {
            movelist.add(Move::new(Flag::ROOKPROM, prom.target_square(), prom.source_square()));
            movelist.add(Move::new(Flag::BISHOPPROM, prom.target_square(), prom.source_square()));
        }
    }

    let mut i = if tt_move.is_none() { 0 } else { 1 };
    while i < movelist.length {
        let mv = movelist[i];
        i += 1;
        if !board.is_legal(mv) {
            continue;
        }
        let unmake_info = board.makemove(mv);

        let score = if board.is_rule_draw() { 0 } else {
            // PVS: search the first move normally. Search the rest with a null window and re-search if it beats alpha
            if !moved {
                -negamax(stop_flag, board, state, full_search_depth, ply + 1, -beta, -alpha, true).1
            } else {
                let mut reduced_search_depth = depth - 1;
                if depth >= 3 && tried_quiets_idx >= 3 && !in_check && ply > 1 && !mv.is_queen_promotion() {
                    reduced_search_depth = (depth - 1).saturating_sub(LMR_TABLE[depth.min(63) as usize][i.min(63) as usize]);
                }
                let mut score = -negamax(stop_flag, board, state, reduced_search_depth, ply + 1, -alpha-1, -alpha, true).1;
                if score > alpha {
                    score = -negamax(stop_flag, board, state, full_search_depth, ply + 1, -beta, -alpha, true).1;
                }
                score
            }
        };
        moved = true;
        board.unmakemove(mv, score, unmake_info, Some(&mut movelist));
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            state.beta_cutoff(mv, board.move_history.last().copied(), depth, ply, &tried_quiets[..tried_quiets_idx]);
            break;
        }
        if !mv.is_capture() && !mv.is_queen_promotion() {
            tried_quiets[tried_quiets_idx] = mv;
            tried_quiets_idx += 1;
        }
    }

    if !moved {
        return (None, if in_check { -(MATE - ply as Eval) } else { 0 });
    }

    if board.state.repetitions <= 1 {
        let tt_flag = 
            if best_score >= beta { TTFlag::LowerBound }
            else if best_score <= original_alpha { TTFlag::UpperBound }
            else { TTFlag::Exact };
        let tt_entry = TTEntry::new(board.state.hash, best_score, tt_flag, best_move, depth, state.tt.generation());
        state.tt.insert(tt_entry, ply);
    }

    return (best_move, best_score);
}

pub fn quiescence(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, ply: u8, mut alpha: Eval, beta: Eval) -> Eval {
    state.node_count += 1;
    if state.node_count % TIMEOUT_MOD == 0 {
        if stop_flag.load(Ordering::Relaxed) {
            return 0;
        }
    }

    // consider stand pat (not capturing)
    let stand_pat = partial_relative_eval(board, alpha, beta);
    if stand_pat > alpha {
        alpha = stand_pat;
    }
    if alpha >= beta || ply >= MAX_PLY {
        return stand_pat;
    }

    let in_check = board.in_check();
    let mut movelist = board.generate_movelist(!in_check);
    // TODO: try different scoring methods for quiescence search
    let mut scores = movelist.score(board, &state, None, ply);
    movelist.sort(&mut scores);

    let mut best_score = stand_pat;
    let mut moved = false;
    let mut i = 0;
    while i < movelist.length {
        let mv = movelist[i];
        let mv_prescore = scores[i];
        i += 1;
        if !in_check && mv_prescore < 0 {
            break;
        }
        if !board.is_legal(mv) {
            continue;
        }
        let unmake_info = board.makemove(mv);
        moved = true;
        let score = if board.is_rule_draw() { 0 } else {
            -quiescence(stop_flag, board, state, ply + 1, -beta, -alpha)
        };
        board.unmakemove(mv, score, unmake_info, Some(&mut movelist));
        best_score = Eval::max(best_score, score);
        alpha = Eval::max(alpha, score);
        if alpha >= beta {
            break;
        }
    }

    if !moved && in_check {
        return -(MATE - ply as Eval);
    }
    best_score
}

pub static LMR_TABLE: Lazy<[[u8; 64]; 64]> = Lazy::new(|| {
    let mut table = [[0; 64]; 64];
    
    for d in 0..64 {
        for i in 0..64 {
            if d == 0 || i == 0 { continue; }
            let r = 0.5 + (f64::ln(d as f64) + f64::ln(i as f64)) / 2.25;
            table[d][i] = (r + 0.5) as u8;
        }
    }

    table
});