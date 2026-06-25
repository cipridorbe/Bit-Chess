use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};

use once_cell::sync::Lazy;

use crate::{eval::{Eval, INF, MATE, MATE_CUTOFF, partial_relative_eval, relative_eval}, movegen::r#move::{Flag, MAX_MOVES, Move, MoveList, REGULAR_QUIET_SCORE}, repr::{board::Board, piece::Piece}, search::{MAX_PLY, NUM_THREADS, state::SearchState, tt::{TTEntry, TTFlag, adjust_retrieve_eval}}};

pub const TIMEOUT_MOD: u64 = 1 << 13;

const ASPIRATION_WINDOW_DELTAS: [Eval; 2] = [25, 40];

const NULL_MOVE_SEARCH_MARGIN: Eval = 65;
const FUTILITY_MARGIN: Eval = 150;
const TT_FUTILITY_MARGIN: Eval = 100;
const REVERSE_FUTILITY_PRUNING_MARGIN: Eval = 215;
const DELTA_PRUNING_MARGIN: Eval = 160;

pub fn search(board: &mut Board, search_state: &mut SearchState, depth: u8, stop_flag: &Arc<AtomicBool>, end: Option<Instant>) -> (Option<Move>, Eval, u8, u64) {
    search_state.new_search();
    let mut threads = Vec::new();
    let fake_stop_flag = Arc::new(AtomicBool::new(false));
    for _ in 1..NUM_THREADS {
        let mut cloned_search_state = search_state.new_helper_thread();
        let mut cloned_board = board.clone();
        let cloned_stop_flag = Arc::clone(&fake_stop_flag);
        let thread = std::thread::spawn(move || {
            iterative_deepening(&mut cloned_board, &mut cloned_search_state, depth, &cloned_stop_flag, end)
        });
        threads.push(thread);
    }

    search_state.fake_stop_flag = Some(Arc::clone(&fake_stop_flag));
    let (mv, eval, reached, mut nodes_visited) = iterative_deepening(board, search_state, depth, stop_flag, end);
    fake_stop_flag.store(true, Ordering::Relaxed);
    search_state.fake_stop_flag = None;
    for thread in threads {
        if let Ok((_, _, _, nodes)) = thread.join() {
            nodes_visited += nodes;
        }
    }

    (mv, eval, reached, nodes_visited)
}

pub fn iterative_deepening(board: &mut Board, state: &mut SearchState, max_depth: u8, stop_flag: &Arc<AtomicBool>, end: Option<Instant>) -> (Option<Move>, Eval, u8, u64) {
    let mut best_move = None;
    let mut score = 0;
    let mut reached_depth = 0;
    let mut last_iteration_duration = Duration::ZERO;
    for depth in 1..=max_depth {
        let start = Instant::now();
        if let Some(end) = end {
            let remaining = end.saturating_duration_since(start);
            let predicted = last_iteration_duration;
            if remaining < predicted {
                break;
            }
        }
        state.max_depth = depth + depth / 2;
        let (mv, current_score) = if depth <= 5 { 
            negamax(stop_flag, board, state, depth, 0, -INF, INF, false)
        } else {
            let mut a = 0;
            let mut b = 0;
            let (mut _mv, mut _current_score) = (None, 0);
            loop {
                let alpha = if a < ASPIRATION_WINDOW_DELTAS.len() {
                    score - ASPIRATION_WINDOW_DELTAS[a]
                } else {
                    -INF
                };
                let beta = if b < ASPIRATION_WINDOW_DELTAS.len() {
                    score + ASPIRATION_WINDOW_DELTAS[b]
                } else {
                    INF
                };
                (_mv, _current_score) = negamax(stop_flag, board, state, depth, 0, alpha, beta, false);
                if _current_score <= alpha && alpha > -INF {
                    if _current_score <= -MATE_CUTOFF { a = ASPIRATION_WINDOW_DELTAS.len(); }
                    else { a += 1; }
                    continue;
                }
                if _current_score >= beta && beta < INF {
                    if _current_score >= MATE_CUTOFF { b = ASPIRATION_WINDOW_DELTAS.len(); }
                    else { b += 1; }
                    continue;
                }

                break;
            }
            (_mv, _current_score)
        };
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        best_move = mv;
        score = current_score;
        reached_depth = depth;
        last_iteration_duration = Instant::now() - start;
        if current_score.abs() >= MATE_CUTOFF {
            break;
        }
    }
    (best_move, score, reached_depth, state.node_count)
}

pub fn negamax(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, depth: u8, ply: u8, mut alpha: Eval, mut beta: Eval, null_move_allowed: bool) -> (Option<Move>, Eval) {
    state.node_count += 1;

    if state.stop_search {
        return (None, 0);
    }

    // if stop flag is set, stop the search
    if state.node_count % TIMEOUT_MOD == 0 {
        if stop_flag.load(Ordering::Relaxed) {
            state.stop_search = true;
            if state.is_main {
                if let Some(ref fake_flag) = state.fake_stop_flag {
                    fake_flag.store(true, Ordering::Relaxed);
                }
            }
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
    let (tt_entry, second_best_move) = state.find_tt(board.state.hash);
    if let Some(entry) = tt_entry {
        tt_move = entry.best_move;
        let eval = adjust_retrieve_eval(entry.eval, ply);
        if entry.depth >= depth && board.state.repetitions <= 1 && board.halfmove_clock < 90 && !is_pv {
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

    alpha = alpha.max(-(MATE - ply as Eval));
    beta  = beta.min(MATE - (ply + 1) as Eval);
    if alpha >= beta { return (None, alpha); }

    let original_alpha = alpha;

    // Try to create a beta cutoff with a null move
    if null_move_allowed && ply > 1 && !board.in_check() && depth >= 4 && board.state.phase_unbounded > 0 && beta < MATE_CUTOFF && !is_pv && relative_eval(board, state) >= beta - NULL_MOVE_SEARCH_MARGIN {
        let null_unmake = board.null_makemove();
        let score = -negamax(stop_flag, board, state, depth / 2, ply + 1, -beta-1, -beta, false).1;
        board.null_unmakemove(null_unmake);
        if score >= beta {
            let tt_entry = TTEntry::new(board.state.hash, score, TTFlag::LowerBound, None, depth, state.tt.generation());
            state.insert_tt(tt_entry, ply);
            return (None, score)
        }
    }

    // If no tt move is found, find the best move by performing a shallower search
    // TODO: test if pv_node improves speed
    if tt_move.is_none() && depth >= 4 && is_pv {
        let new_depth = depth / 2 + 1;
        tt_move = negamax(stop_flag, board, state, new_depth, ply, alpha, beta, false).0;
    }

    let mut tried_quiets = [Move::NULL_MOVE; MAX_MOVES];
    let mut tried_quiets_idx = 0;
    let mut moved = false;
    let mut best_move = None;
    let mut best_score = -INF;

    let mut skip_quiets = false;

    // Futility pruning
    if ply > 0 && depth <= 2 && !in_check && !is_pv && alpha < MATE_CUTOFF {
        let eval = board.pst_eval();
        if eval >= beta + REVERSE_FUTILITY_PRUNING_MARGIN * depth as Eval {
            return (None, eval);
        }
        if eval <= alpha - FUTILITY_MARGIN * depth as Eval {
            skip_quiets = true;
        }
    }

    // SEE pruning
    let see_pruning = ply > 0 && !is_pv && !in_check && depth == 1;

    // Try to create a beta cutoff by making the tt move first
    let mut movelist = MoveList::new();
    let mut scores = [0; MAX_MOVES];

    if let Some(tt_mv) = tt_move {
        let unmake_info = board.makemove(tt_mv);
        moved = true;
        let tt_score = if board.is_rule_draw() { 0 } else {
            -negamax(stop_flag, board, state, full_search_depth, ply + 1, -beta, -alpha, true).1
        };
        board.unmakemove(tt_mv, unmake_info);
        if tt_score > alpha {
            alpha = tt_score;
        } else {
            if ply > 0 && depth <= 3 && !in_check && !is_pv && tt_score <= alpha - TT_FUTILITY_MARGIN * depth as Eval && alpha < MATE_CUTOFF {
                skip_quiets = true;
            }
        }
        if alpha >= beta {
            state.beta_cutoff(board, tt_mv, depth, ply, &[]);
            if board.state.repetitions <= 1 {
                let tt_entry = TTEntry::new(board.state.hash, tt_score, TTFlag::LowerBound, tt_move, depth, state.tt.generation());
                state.tt.insert(tt_entry, ply);
            }
            return (tt_move, tt_score)
        }
        if !tt_mv.is_capture() {
            tried_quiets[tried_quiets_idx] = tt_mv;
            tried_quiets_idx += 1;
        }
        best_score = tt_score;
        best_move = tt_move;
        // singularity extensions: if there is only one good move, search deeper
        if depth >= 99 && tt_score.abs() >= MATE_CUTOFF {
            movelist = board.generate_movelist(false);
            scores = movelist.score(board, state, tt_move, second_best_move, ply);
            movelist.sort(&mut scores);
            movelist.maybe_add_proms(tt_score, tt_move, 0);
            let singularity_beta = 
                if depth >= 12 { tt_score - 33 - 20 * is_pv as Eval }
                else { tt_score - 40 - 30 * is_pv as Eval };
            let mut fail_high = false;
            let mut i = 1;
            while i < movelist.length {
                let mv = movelist[i];
                i += 1;
                let unmake = board.makemove(mv);
                let score = if board.is_rule_draw() { 0 } else {
                    -negamax(stop_flag, board, state, depth / 2, ply + 1, -singularity_beta - 1, -singularity_beta, true).1
                };
                board.unmakemove(mv, unmake);
                movelist.maybe_add_proms(score, Some(mv), i - 1);
                if score >= singularity_beta {
                    fail_high = true;
                    movelist.shift(&mut scores, i - 1, 1);
                    break;
                }
            }

            if !fail_high {
                let singularity_depth = depth;
                let score = 
                if depth + ply >= state.max_depth {
                    // same search as original search
                    tt_score
                } else {
                    let unmake = board.makemove(tt_mv);
                    let score = if board.is_rule_draw() { 0 } else {
                        -negamax(stop_flag, board, state, singularity_depth, ply + 1, -beta, -original_alpha, true).1
                    };
                    board.unmakemove(tt_mv, unmake);
                    score
                };
                let flag = if score >= beta { TTFlag::LowerBound }
                    else if score <= original_alpha { TTFlag::UpperBound }
                    else { TTFlag::Exact };
                let tt_entry = TTEntry::new(board.state.hash, score, flag, Some(tt_mv), singularity_depth, state.tt.generation());
                state.insert_tt(tt_entry, ply);
                return (Some(tt_mv), score);
            }
        }
    }

    // If we haven't cut off, we have to start exploring more moves
    if movelist.length == 0 {
        movelist = board.generate_movelist(false);
        scores = movelist.score(board, state, tt_move, second_best_move, ply);
        movelist.sort(&mut scores);
    } else {
        movelist.maybe_add_proms(best_score, tt_move, 0);
    }

    let mut early_exit = false;
    let mut i = if tt_move.is_none() { 0 } else { 1 };
    while i < movelist.length {
        if state.stop_search {
            return (None, 0);
        }
        let mv = movelist[i];
        let mv_prescore = scores[i];
        i += 1;
        if moved && skip_quiets && mv_prescore <= REGULAR_QUIET_SCORE && best_score > -MATE_CUTOFF{
            early_exit = true;
            break;
        }
        if moved && see_pruning && mv.is_capture() && mv_prescore < 0 && best_score > -MATE_CUTOFF {
            early_exit = true;
            break;
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
        board.unmakemove(mv, unmake_info);
        movelist.maybe_add_proms(score, Some(mv), i - 1);
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            state.beta_cutoff(board, mv, depth, ply, &tried_quiets[..tried_quiets_idx]);
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

    if board.state.repetitions <= 1 && !early_exit {
        let tt_flag = 
            if best_score >= beta { TTFlag::LowerBound }
            else if best_score <= original_alpha { TTFlag::UpperBound }
            else { TTFlag::Exact };
        let tt_entry = TTEntry::new(board.state.hash, best_score, tt_flag, best_move, depth, state.tt.generation());
        state.insert_tt(tt_entry, ply);
    }

    return (best_move, best_score);
}

pub fn quiescence(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, ply: u8, mut alpha: Eval, beta: Eval) -> Eval {
    state.node_count += 1;
    if state.stop_search {
        return 0;
    }
    
    if state.node_count % TIMEOUT_MOD == 0 {
        if stop_flag.load(Ordering::Relaxed) {
            state.stop_search = true;
            if state.is_main {
                if let Some(ref fake_stop_flag) = state.fake_stop_flag {
                    fake_stop_flag.store(true, Ordering::Relaxed);
                }
            }
            return 0;
        }
    }

    let is_pv = beta > alpha + 1;
    let in_check = board.in_check();
    // consider stand pat (not capturing)
    let stand_pat = partial_relative_eval(board, state, alpha, beta);
    if stand_pat > alpha && !in_check {
        alpha = stand_pat;
    }
    
    let pawn_endgame = board.state.phase_unbounded == 0;
    let promoting = board[Piece::WhitePawn] & Board::RANK_7 != 0 || board[Piece::BlackPawn] & Board::RANK_2 != 0;
    let delta_pruning = !is_pv && !in_check && !pawn_endgame && !promoting;
    if alpha >= beta || ply >= MAX_PLY || (delta_pruning && stand_pat + Piece::WhiteQueen.abs_regular_value() <= alpha - DELTA_PRUNING_MARGIN) {
        return stand_pat;
    }

    let mut movelist = board.generate_movelist(!in_check);
    // TODO: try different scoring methods for quiescence search
    let mut scores = if in_check {
        movelist.score(board, &state, None, None, ply)
    } else {
        movelist.quiescense_score(board)
    };
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
        if delta_pruning {
            let captured_value = if mv.flag() == Flag::ENPASSANT { Piece::WhitePawn.abs_regular_value() } else {
                board[mv.target_square()].unwrap().abs_regular_value()
            };
            if stand_pat + captured_value <= alpha - DELTA_PRUNING_MARGIN {
                continue;
            }
        }
        let unmake_info = board.makemove(mv);
        moved = true;
        let score = if board.is_rule_draw() { 0 } else {
            -quiescence(stop_flag, board, state, ply + 1, -beta, -alpha)
        };
        board.unmakemove(mv, unmake_info);
        movelist.maybe_add_proms(score, Some(mv), i - 1);
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

pub fn simple_search(stop_flag: &Arc<AtomicBool>, board: &mut Board, state: &mut SearchState, depth: u8, ply: u8, beta: Eval, null_move_allowed: bool, movelist: &mut MoveList, start: usize, end: usize, fail_highs: u8) -> (Option<usize>, Eval) {
    let mut best_score = -INF;
    let mut best_move = None;
    let mut failed_highs = 0;
    let mut i = start;
    while i < movelist.length.min(end) {
        let mv = movelist[i];
        i += 1;
        let unmake = board.makemove(mv);
        let score = if board.is_rule_draw() { 0 } else {
            -negamax(stop_flag, board, state, depth, ply + 1, -beta, -beta + 1, true).1
        };
        board.unmakemove(mv, unmake);
        movelist.maybe_add_proms(score, Some(mv), i - 1);
        if score >= best_score {
            best_score = score;
            best_move = Some(i);
        }
        if score >= beta {
            failed_highs += 1;
            if failed_highs >= fail_highs {
                break;
            }
        }
    }
    (best_move, best_score)
}

pub static LMR_TABLE: Lazy<[[u8; 64]; 64]> = Lazy::new(|| {
    let mut table = [[0; 64]; 64];
    
    for d in 0..64 {
        for i in 0..64 {
            if d == 0 || i == 0 { continue; }
            let r = 0.5 + (f64::ln(d as f64) + f64::ln(i as f64)) / 2.25;
            table[d][i] = r as u8;
        }
    }

    table
});