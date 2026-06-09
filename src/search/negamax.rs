/*
Implementation of negamax, used as the main search algorithm.
*/

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use once_cell::sync::Lazy;

use crate::{bitboard::{Board, Piece, Square}, eval::{PIECE_VALUE, relative_eval}, movegen::{generator::generate_movelist, makemove::{make_move, make_null_move, unmake_move, unmake_null_move}, r#move::{Flag, LazyMoveIter, Move}}, search::{state::SearchState, tt::{TT, TTEntry, TTFlag}}};

const MATE_VAL: i16 = 30000;
const MATE_CUTOFF: i16 = 29000;
pub const INF: i16 = 31000;
const DELTA_SEARCH: i16 = 50;
const FUTILITY_MARGIN: i16 = 150;
const REV_FUTILITY_MARGIN: i16 = 200;
const DELTA_PRUNING_MARGIN: i16 = 200;

pub static mut NODE_COUNT: u64 = 0;
pub static mut TT_LOOKUPS_DEPTH: [u64; 64] = [0; 64];
pub static mut TT_LOOKUPS_PLY: [u64; 64] = [0; 64];
pub static mut TT_LOOKUPS_DEPTH_SUCCESS: [u64; 64] = [0; 64];
pub static mut TT_LOOKUPS_PLY_SUCESS: [u64; 64] = [0; 64];

pub static LMR_TABLE: Lazy<[[u8; 64]; 64]> = Lazy::new(|| {
    let mut table = [[0; 64]; 64];

    for d in 0..64 {
        for m in 0..64 {
            let r = 0.5 + (f64::ln(d as f64) + f64::ln(m as f64)) / 2.5;
            table[d][m] = (r + 0.5) as u8;
        }
    }
    
    table
});

pub fn store_score(score: i16, plies: u8) -> i16 {
    if score.abs() > MATE_CUTOFF {
        score.signum() * (score.abs() + plies as i16)
    } else {
        score
    }
}

pub fn retrieve_score(score: i16, plies: u8) -> i16 {
    if score.abs() > MATE_CUTOFF {
        score.signum() * (score.abs() - plies as i16)
    } else {
        score
    }
}

pub fn search(stop: &Arc<AtomicBool>, board: &mut Board, mut max_depth: u8, tt: &mut Arc<TT>, history: &mut [[i16; 64]; 64], counter_move: &mut [[Option<Move>; 64]; 64]) -> Option<Move> {
    max_depth += 1;
    let original_history = history.clone();
    let original_countermove = counter_move.clone();
    let original_board = board.clone();
    Arc::get_mut(tt).unwrap().new_search();
    max_depth += match board.phase {
        0 => 4,
        1..=4 => 2,
        5..=9 => 1,
        10..=13 => 0,
        _ => 0
    };

    let mut helpers: Vec<_> = Vec::new();
    for _ in 1..3 {
        let thread_tt = Arc::clone(tt);
        let mut thread_history = original_history.clone();
        let mut thread_countermove = original_countermove.clone();
        let stop = Arc::clone(stop);
        let mut thread_board = original_board.clone();
        let handle = std::thread::spawn(move || {
            let mut thread_state = SearchState::new_helper(&thread_tt, &mut thread_history, &mut thread_countermove);
            iterative_deepening(&stop, &mut thread_board, max_depth, &mut thread_state);
        });
        helpers.push(handle);
    }

    let main_tt = Arc::clone(tt);
    let main_stop = Arc::clone(stop);
    let mut main_state = SearchState::new_helper(&main_tt, history, counter_move);
    let (best_move, _) = iterative_deepening(&main_stop, board, max_depth,  &mut main_state);

    stop.store(true, Ordering::Relaxed);
    for helper in helpers {
        let _ = helper.join();
    }

    best_move
}

pub fn iterative_deepening(stop: &Arc<AtomicBool>, board: &mut Board, max_depth: u8, state: &mut SearchState) -> (Option<Move>, i16) {
    let mut best_move = None;
    let mut iteration_score = 0;
    
    let aspiration_deltas = [(DELTA_SEARCH as f32 * 1.) as i16];
    for depth in 1..=max_depth {
        state.max_depth = depth;

        let mut aspiration_misses = [0, 0];

        let mut aspiration_alpha = if depth <= 2 { -INF } else { iteration_score - aspiration_deltas[0] };
        let mut aspiration_beta = if depth <= 2 { INF } else { iteration_score + aspiration_deltas[0] };

        loop {
            let (score, mv) = negamax(stop, board, aspiration_alpha, aspiration_beta, depth, 0, false, None, state);
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if score <= aspiration_alpha && score.abs() <= MATE_CUTOFF {
                while score <= aspiration_alpha {
                    aspiration_misses[0] += 1;
                    if aspiration_misses[0] < aspiration_deltas.len() {
                        aspiration_alpha = iteration_score - aspiration_deltas[aspiration_misses[0]];
                    } else {
                        aspiration_alpha = -INF;
                    }
                }
            } else if score >= aspiration_beta && score.abs() <= MATE_CUTOFF {
                while score >= aspiration_beta {
                    aspiration_misses[1] += 1;
                    if aspiration_misses[1] < aspiration_deltas.len() {
                        aspiration_beta = iteration_score + aspiration_deltas[aspiration_misses[1]];
                    } else {
                        aspiration_beta = INF;
                    }
                }
            } else {
                best_move = mv;
                iteration_score = score;
                if score.abs() > MATE_CUTOFF {
                    return (best_move, iteration_score)
                }
                break;
            }
        }
    }
    (best_move, iteration_score)
}

pub fn search_move(stop: &Arc<AtomicBool>, board: &mut Board, alpha: i16, beta: i16, depth: u8, ply: u8, i: usize, mv: Move, quiet_moves_made: usize, state: &mut SearchState) -> i16 {
    let mut full_search_depth = depth - 1;
    if board.in_check(board.side) && ply + depth < (state.max_depth + state.max_depth / 2) {
        full_search_depth = depth;
    }
    if i == 0 {
        return -negamax(stop, board, -beta, -alpha, full_search_depth, ply + 1, true, Some(mv), state).0
    }

    let mut reduced_depth_search = depth - 1;
    if quiet_moves_made > 3 && depth >= 3 && !mv.is_queen_promotion() && ply > 1 && !board.in_check(board.side) {
        let reduction = LMR_TABLE[depth.min(63) as usize][i.min(63) as usize];
        reduced_depth_search = depth - reduction.min(depth - 1);
    }
    let mut score = -negamax(stop, board, -alpha-1, -alpha, reduced_depth_search, ply + 1, true, Some(mv), state).0;
    if score > alpha {
        score = -negamax(stop, board, -beta, -alpha, full_search_depth, ply + 1, true, Some(mv), state).0;
    } 
    score
}

pub fn search_moves(stop: &Arc<AtomicBool>, board: &mut Board, mut alpha: i16, beta: i16, depth: u8, ply: u8, movelist: &mut LazyMoveIter, skip_quiet: bool, skip_tried: bool, previous_move: Option<Move>, state: &mut SearchState) -> (i16, Option<Move>, bool) {
    let is_pv_node = beta > alpha + 1;
    let mut value = -INF;
    let mut best_move = None;
    let mut moved = false;
    let mut tried_quiets = [Move::new(Flag::QUIET, Square::a2, Square::a2); 218];
    let mut tried_quiets_idx: usize = 0;
    let lmp = !board.in_check(board.side) && depth <= 2;
    let see_pruning = lmp && !is_pv_node;
    for (i, (mv, mv_score)) in movelist.enumerate() {
        if skip_tried && (i == 0 || mv.is_capture()) {
            continue;
        }
        if lmp && !mv.is_capture() && !mv.is_queen_promotion() && moved && tried_quiets_idx as u8 >= 2 + depth * depth {
            continue;
        }
        if see_pruning && mv.is_capture() && Move::see_negative_score(mv_score) < -1 - depth as i16 && moved {
            break;
        }
        let unmake = make_move(board, mv);
        if board.in_check(board.side.other()) || (skip_quiet && i != 0 && !mv.is_capture() && !board.in_check(board.side)) {
            unmake_move(board, mv, &unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() { 0 } else {
            search_move(stop, board, alpha, beta, depth, ply, i, mv, tried_quiets_idx, state)
        };
        unmake_move(board, mv, &unmake);
        if score > value {
            value = score;
            best_move = Some(mv);
        }
        alpha = i16::max(alpha, score);
        if alpha >= beta {
            state.beta_cutoff(mv, previous_move, depth, ply, &tried_quiets[..tried_quiets_idx]);
            break;
        }
        if !mv.is_capture() && !mv.is_promotion() {
            tried_quiets[tried_quiets_idx] = mv;
            tried_quiets_idx += 1;
        }
    }
    (value, best_move, moved)
}

pub fn negamax(stop: &Arc<AtomicBool>, board: &mut Board, mut alpha: i16, mut beta: i16, depth: u8, ply: u8, allow_null_move: bool, previous_move: Option<Move>, state: &mut SearchState) -> (i16, Option<Move>) {
    if unsafe { NODE_COUNT } & 0b1111111111 == 0 && stop.load(Ordering::Relaxed) {
        return (0, None)
    }

    if depth == 0 {
        return (quiescence(stop, board, alpha, beta, ply, state), None);
    }

    unsafe { NODE_COUNT += 1; }

    let is_pv_node = beta > alpha + 1;
    let mut predicted_best_move = None;
    unsafe { TT_LOOKUPS_DEPTH[depth.min(63) as usize] += 1; TT_LOOKUPS_PLY[ply.min(63) as usize] += 1; }
    if let Some(entry) = state.tt.find(board.hash) {
        unsafe { TT_LOOKUPS_DEPTH_SUCCESS[depth.min(63) as usize] += 1; TT_LOOKUPS_PLY_SUCESS[ply.min(63) as usize] += 1; }
        predicted_best_move = entry.best_move;
        if !is_pv_node && entry.depth >= depth && board.repetitions <= 1 {
            let retrieved_score = retrieve_score(entry.score, ply);
            match entry.flag {
                TTFlag::Exact => return (retrieved_score, predicted_best_move),
                TTFlag::Lower => alpha = i16::max(alpha, retrieved_score),
                TTFlag::Upper => beta = i16::min(beta, retrieved_score)
            }
            if alpha >= beta { return (retrieved_score, predicted_best_move); }
        }
    }

    if !is_pv_node && allow_null_move && board.phase > 0 && beta < INF && depth > 3 && !board.in_check(board.side) {
        let unmake = make_null_move(board);
        let null_move_score = -negamax(stop, board, -beta, -beta + 1, depth - 3, ply + 1, false, None, state).0;
        unmake_null_move(board, &unmake);
        if null_move_score >= beta {
            let insert_score = store_score(null_move_score, ply);
            state.tt.insert(TTEntry::new(board.hash, depth, insert_score, TTFlag::Lower, None, board.fullmoves, state.tt.generation));
            return (null_move_score, None);
        }
    }

    if is_pv_node && depth >= 4 && predicted_best_move == None {
        predicted_best_move = negamax(stop, board, -beta, -alpha, depth / 2, ply, false, previous_move, state).1;
    }

    let mut skip_quiet = false; 
    if ply > 0 && !is_pv_node && depth <= 2 && !board.in_check(board.side) {
        let eval = relative_eval(board);
        if eval - depth as i16 * REV_FUTILITY_MARGIN >= beta {
            return (eval, None);
        }
        skip_quiet = eval + depth as i16 * FUTILITY_MARGIN < alpha;
    }
    let original_alpha = alpha;

    let counter = previous_move.and_then(|pm| state.counter_move[pm.source_square() as usize][pm.target_square() as usize]);
    let mut lazymovelist = generate_movelist(board, false);
    let mut lazy_iter = lazymovelist.lazy_iter(board, predicted_best_move, &state.killers[ply as usize], state.history, counter);

    let (mut value, mut best_move, mut moved)
        = search_moves(stop, board, alpha, beta, depth, ply, &mut lazy_iter, skip_quiet, false, previous_move, state);
    
    if !moved && skip_quiet {
        lazy_iter.reset();
        (value, best_move, moved)
            = search_moves(stop, board, alpha, beta, depth, ply, &mut lazy_iter, false, true, previous_move, state);
    }

    if !moved {
        return if board.in_check(board.side) {
            (-(MATE_VAL - ply as i16), None)
        } else {
            (0, None)
        };
    }

    let ttflag = 
        if value >= beta { TTFlag::Lower }
        else if value <= original_alpha { TTFlag::Upper }
        else { TTFlag::Exact };
    if board.repetitions <= 1 {
        let store_score = store_score(value, ply);
        state.tt.insert(TTEntry::new(board.hash, depth, store_score, ttflag, best_move, board.fullmoves, state.tt.generation));
    }
    (value, best_move)
}

fn quiescence(stop: &Arc<AtomicBool>, board: &mut Board, mut alpha: i16, beta: i16, ply: u8, state: &SearchState) -> i16 {
    if unsafe { NODE_COUNT } & 0b1111111111 == 0 && stop.load(Ordering::Relaxed) {
        return 0
    }
    unsafe { NODE_COUNT += 1; }
    let in_check = board.in_check(board.side);
    let stand_pat = if !in_check {
        let eval = relative_eval(board);
        if eval >= beta {
            return eval;
        }
        alpha = i16::max(alpha, eval);
        eval
    } else {
        -INF
    };

    let mut value = stand_pat;
    let mut moved = false;
    let pawn_endgame = !board.has_non_pawn_pieces();
    let near_prom = (board.pieces[Piece::WhitePawn as usize] & Board::RANK_7 != 0) || (board.pieces[Piece::BlackPawn as usize] & Board::RANK_2 != 0);
    let delta_pruning = !in_check && !pawn_endgame && !near_prom;
    if delta_pruning && stand_pat + PIECE_VALUE[Piece::WhiteQueen as usize].abs() + DELTA_PRUNING_MARGIN <= alpha {
        return stand_pat;
    }
    let mut movelist = generate_movelist(board, !in_check);
    let see_scores = movelist.sort_see_sign(board);
    for (i, mv) in movelist.iter().enumerate() {
        if !in_check && see_scores[i] < 0 {
            break;
        }
        if delta_pruning {
            let mut captured_value = PIECE_VALUE[Piece::WhitePawn as usize].abs();
            if let Some(captured) = board.piece_at(mv.target_square()) {
                captured_value = PIECE_VALUE[captured as usize].abs();
            }
            if stand_pat + captured_value + DELTA_PRUNING_MARGIN <= alpha {
                continue;
            }
        }
        let unmake = make_move(board, mv);
        if board.in_check(board.side.other()) {
            unmake_move(board, mv, &unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() {
            0
        } else {
            -quiescence(stop, board, -beta, -alpha, ply+1, state)
        };
        unmake_move(board, mv, &unmake);
        value = i16::max(value, score);
        alpha = i16::max(alpha, value);
        if alpha >= beta {
            break;
        }
    }
    if !moved && in_check {
        return -(MATE_VAL - ply as i16)
    }
    value
}

#[cfg(test)]
mod tests {
    use super::LMR_TABLE;

    fn colored(r: u8) -> String {
        let code = match r {
            0 => "2",        // dim
            1 => "32",       // green
            2 => "33",       // yellow
            3 => "31",       // red
            _ => "1;31",     // bold red
        };
        format!("\x1b[{}m{:>3}\x1b[0m", code, r)
    }

    #[test]
    fn print_lmr_table() {
        let table = &*LMR_TABLE;
        let depths = [1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 25, 32, 48, 63];
        let moves  = [1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 30, 40, 50, 55, 63];

        print!("{:>5}", "d\\m");
        for m in moves { print!("{:>7}", m); }
        println!();

        print!("{:>5}", "----");
        for _ in moves { print!("{:>7}", "---"); }
        println!();

        for d in depths {
            print!("{:>5}", d);
            for m in moves {
                print!("    {}", colored(table[d][m]));
            }
            println!();
        }
    }
}
