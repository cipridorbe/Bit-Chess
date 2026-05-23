/*
Implementation of negamax, used as the main search algorithm.
*/

use crate::{bitboard::Board, eval::relative_eval, movegen::{generator::generate_movelist, makemove::{make_move, make_null_move, unmake_move, unmake_null_move}, r#move::{Move, MoveList}}, search::{state::SearchState, tt::{TT, TTEntry, TTFlag}}};

const MATE_VAL: i16 = 30000;
const MATE_CUTOFF: i16 = 29000;
pub const INF: i16 = 31000;
const DELTA_SEARCH: i16 = 50;
const FUTILITY_MARGIN: i16 = 150;

pub static mut NODE_COUNT: u64 = 0;

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

pub fn search(board: &mut Board, mut max_depth: u8, tt: &mut TT, history: &mut [[i16; 64]; 64], counter_move: &mut [[Option<Move>; 64]; 64]) -> Option<Move> {
    let mut state = SearchState::new_search(tt, history, counter_move);
    let mut best_move = None;
    let mut iteration_score = 0;
    max_depth += if board.is_pawn_endgame() { 3 } else { 0 };
    for depth in 1..=max_depth {
        state.max_depth = depth;
        let tt_best = state.tt.find(board.hash).and_then(|e| e.best_move);
        let mut movelist = generate_movelist(&board, false);
        movelist.sort(&board, tt_best, &state.killers[0], state.history, None);

        let mut aspiration_alpha = if depth <= 2 { -INF } else { iteration_score - DELTA_SEARCH };
        let mut aspiration_beta = if depth <= 2 { INF } else { iteration_score + DELTA_SEARCH };

        loop {
            let mut best_score = -INF;
            let mut alpha = aspiration_alpha;

            for mv in movelist.iter() {
                let unmake = make_move(board, mv);
                if board.in_check(board.side.other()) {
                    unmake_move(board, mv, &unmake);
                    continue;
                }
                let score = if board.is_rule_draw() {
                    0
                } else {
                    -negamax(board, -aspiration_beta, -alpha, depth, 0, false, None, &mut state)
                };
                unmake_move(board, mv, &unmake);
                if score > best_score {
                    best_score = score;
                    best_move = Some(mv);
                    alpha = score;
                }
                if alpha >= aspiration_beta {
                    break;
                }
            }
            if best_score <= aspiration_alpha && best_score <= MATE_CUTOFF {
                aspiration_alpha = -INF;
            } else if best_score >= aspiration_beta && best_score <= MATE_CUTOFF {
                aspiration_beta = INF;
            } else {
                state.tt.insert(TTEntry::new(board.hash, depth, best_score, TTFlag::Exact, best_move, board.fullmoves, state.tt.generation));
                iteration_score = best_score;
                if best_score > MATE_CUTOFF {
                    return best_move
                }
                break;
            }
        }
    }
    best_move
}

pub fn search_move(board: &mut Board, alpha: i16, beta: i16, depth: u8, ply: u8, i: usize, mv: Move, captures: usize, state: &mut SearchState) -> i16 {
    let mut full_search_depth = depth - 1;
    if board.in_check(board.side) && ply < 2 * state.max_depth {
        full_search_depth = depth;
    }
    if i == 0 {
        return -negamax(board, -beta, -alpha, full_search_depth, ply + 1, true, Some(mv), state)
    }

    let mut reduced_depth_search = depth - 1;
    if i > captures + 7 && depth >= 3 && !mv.is_promotion() && !board.in_check(board.side) {
        reduced_depth_search = depth - 2;
    }
    let mut score = -negamax(board, -alpha-1, -alpha, reduced_depth_search, ply + 1, true, Some(mv), state);
    if score > alpha || score.abs() > MATE_CUTOFF {
        score = -negamax(board, -beta, -alpha, full_search_depth, ply + 1, true, Some(mv), state);
    } 
    score
}

pub fn search_moves(board: &mut Board, mut alpha: i16, beta: i16, depth: u8, ply: u8, movelist: &MoveList, skip_quiet: bool, skip_tried: bool, previous_move: Option<Move>, state: &mut SearchState) -> (i16, Option<Move>, bool) {
    let mut value = -INF;
    let mut best_move = None;
    let mut moved = false;
    for (i, mv) in movelist.iter().enumerate() {
        if skip_tried && (i == 0 || mv.is_capture()) {
            continue;
        }
        let unmake = make_move(board, mv);
        if board.in_check(board.side.other()) || (skip_quiet && i != 0 && !mv.is_capture() && !board.in_check(board.side)) {
            unmake_move(board, mv, &unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() { 0 } else {
            search_move(board, alpha, beta, depth, ply, i, mv, movelist.captures, state)
        };
        unmake_move(board, mv, &unmake);
        if score > value {
            value = score;
            best_move = Some(mv);
        }
        alpha = i16::max(alpha, score);
        if alpha >= beta {
            state.beta_cutoff(mv, previous_move, depth, ply);
            break;
        }
    }
    (value, best_move, moved)
}

pub fn negamax(board: &mut Board, mut alpha: i16, mut beta: i16, depth: u8, ply: u8, allow_null_move: bool, previous_move: Option<Move>, state: &mut SearchState) -> i16 {
    unsafe { NODE_COUNT += 1; }
    let mut predicted_best_move = None;
    if let Some(entry) = state.tt.find(board.hash) {
        predicted_best_move = entry.best_move;
        if entry.depth >= depth && board.repetitions <= 1 {
            let retrieved_score = retrieve_score(entry.score, ply);
            match entry.flag {
                TTFlag::Exact => return retrieved_score,
                TTFlag::Lower => alpha = i16::max(alpha, retrieved_score),
                TTFlag::Upper => beta = i16::min(beta, retrieved_score)
            }
            if alpha >= beta { return retrieved_score; }
        }
    }
    
    if depth == 0 {
        return quiescence(board, alpha, beta, ply);
    }

    if allow_null_move && board.has_non_pawn_pieces() && beta < INF && depth > 3 && !board.in_check(board.side) {
        let unmake = make_null_move(board);
        let null_move_score = -negamax(board, -beta, -beta + 1, depth - 3, ply + 1, false, None, state);
        unmake_null_move(board, &unmake);
        if null_move_score >= beta {
            let insert_score = store_score(null_move_score, ply);
            state.tt.insert(TTEntry::new(board.hash, depth, insert_score, TTFlag::Lower, None, board.fullmoves, state.tt.generation));
            return null_move_score;
        }
    }

    let original_alpha = alpha;
    let skip_quiet = depth == 1 && !board.in_check(board.side) && relative_eval(board) + FUTILITY_MARGIN < alpha;
    let mut movelist = generate_movelist(board, false);
    let counter = previous_move.and_then(|pm| state.counter_move[pm.source_square() as usize][pm.target_square() as usize]);
    movelist.sort(board, predicted_best_move, &state.killers[ply as usize], state.history, counter);

    let (mut value, mut best_move, mut moved)
        = search_moves(board, alpha, beta, depth, ply, &movelist, skip_quiet, false, previous_move, state);
    
    if !moved && skip_quiet {
        (value, best_move, moved)
            = search_moves(board, alpha, beta, depth, ply, &movelist, false, true, previous_move, state);
    }

    if !moved {
        return if board.in_check(board.side) {
            -(MATE_VAL - ply as i16)
        } else {
            0
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
    return value;


    // loop {    
    //     let mut value = -INF;
    //     let mut moved = false;
    //     let original_alpha = alpha;
    //     let mut best_move = None;
    //     for (i, mv) in movelist.iter().enumerate() {
    //         if futility_prune_failed && (i == 0 || mv.is_capture()) {
    //             continue;
    //         }
    //         let unmake = make_move(board, mv);
    //         if board.in_check(board.side.other()) || (futility_prune && i != 0 && !mv.is_capture() && !board.in_check(board.side)) {
    //             unmake_move(board, mv, &unmake);
    //             continue;
    //         }
    //         moved = true;
    //         let score = if board.is_rule_draw() {
    //             0
    //         } else if i != 0 {
    //             let search_depth = depth - if i > movelist.captures + 7 && depth >= 3 && !mv.is_promotion() && !board.in_check(board.side) {
    //                 2
    //             } else {
    //                 1
    //             };
    //             let low_score = -negamax(board, -alpha-1, -alpha, search_depth, ply + 1, true, state);
    //             if low_score > alpha || low_score.abs() > MATE_CUTOFF {
    //                 let new_depth = if board.in_check(board.side) && ply < 2 * state.max_depth { depth } else { depth - 1 };
    //                 -negamax(board, -beta, -alpha, new_depth, ply + 1, true, state)
    //             } else {
    //                 low_score
    //             }
    //         } else {
    //             let new_depth = if board.in_check(board.side) && ply < 2 * state.max_depth { depth } else { depth - 1 };
    //             -negamax(board, -beta, -alpha, new_depth, ply + 1, true, state)
    //         };
    //         unmake_move(board, mv, &unmake);
    //         if score > value {
    //             best_move = Some(mv);
    //             value = score;
    //         }
    //         alpha = i16::max(alpha, value);
    //         if alpha >= beta {
    //             state.beta_cutoff(mv, depth, ply);
    //             break;
    //         }
    //     }

    //     if !moved && futility_prune {
    //         futility_prune = false;
    //         futility_prune_failed = true;
    //         continue;
    //     }

    //     if !moved {
    //         if board.in_check(board.side) {
    //             return -(MATE_VAL - ply as i16);
    //         } else {
    //             return 0;
    //         }
    //     }

    //     let flag = if original_alpha < value && value < beta {
    //         TTFlag::Exact
    //     } else if value >= beta {
    //         TTFlag::Lower
    //     } else {
    //         TTFlag::Upper
    //     };
    //     if board.repetitions <= 1 {
    //         let store_score = store_score(value, ply);
    //         state.tt.insert(TTEntry::new(board.hash, depth, store_score, flag, best_move, board.fullmoves, state.tt.generation));
    //     }
    //     return value;
    // }
}

fn quiescence(board: &mut Board, mut alpha: i16, beta: i16, ply: u8) -> i16 {
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
    let mut movelist = generate_movelist(board, !in_check);
    let mut moved = false;
    movelist.sort_mvvlva(board, None);
    for mv in movelist.iter() {
        let unmake = make_move(board, mv);
        if board.in_check(board.side.other()) {
            unmake_move(board, mv, &unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() {
            0
        } else {
            -quiescence(board, -beta, -alpha, ply+1)
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
