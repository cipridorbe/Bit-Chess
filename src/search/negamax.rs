/*
Implementation of negamax, used as the main search algorithm.
*/

use serde::de;

use crate::{bitboard::{Board, Piece}, eval::relative_eval, movegen::{attacks::{all_attacks, is_in_check}, generator::generate_movelist, makemove::{make_move, make_null_move, unmake_move, unmake_null_move}, r#move::Move}, search::{state::SearchState, tt::{TT, TTEntry, TTFlag}}};

const MATE_VAL: i16 = 30000;
const MATE_CUTOFF: i16 = 29000;
pub const INF: i16 = 31000;
const DELTA_SEARCH: i16 = 50;

pub static mut NODE_COUNT: u64 = 0;

pub fn search(board: &mut Board, max_depth: u8, tt: &mut TT, history: &mut [[i16; 64]; 64]) -> Option<Move> {
    let mut state = SearchState::new_search(tt, history);
    let mut best_move = None;
    let mut iteration_score = 0;
    for depth in 1..=max_depth {
        state.max_depth = depth;
        let tt_best = state.tt.find(board.hash).and_then(|e| e.best_move);
        let mut movelist = generate_movelist(&board, false);
        movelist.sort(&board, tt_best, &state.killers[0], state.history);

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
                    -negamax(board, -aspiration_beta, -alpha, depth, 0, false, &mut state)
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
            if best_score <= aspiration_alpha {
                aspiration_alpha = -INF;
            } else if best_score >= aspiration_beta {
                aspiration_beta = INF;
            } else {
                state.tt.insert(TTEntry::new(board.hash, depth, best_score, TTFlag::Exact, best_move, board.fullmoves, state.tt.generation));
                iteration_score = best_score;
                break;
            }
        }
    }
    best_move
}

pub fn negamax(board: &mut Board, mut alpha: i16, mut beta: i16, depth: u8, ply: u8, allow_null_move: bool, state: &mut SearchState) -> i16 {
    unsafe { NODE_COUNT += 1; }
    let mut predicted_best_move = None;
    if let Some(entry) = state.tt.find(board.hash) {
        predicted_best_move = entry.best_move;
        if entry.depth >= depth && board.repetitions <= 1 {
            match entry.flag {
                TTFlag::Exact => {
                    if entry.score.abs() > MATE_CUTOFF {
                        let delta = board.fullmoves as i16 - entry.full_moves as i16;
                        return entry.score - entry.score.signum() * delta;
                    } else {
                        return entry.score;
                    }
                },
                TTFlag::Lower => alpha = i16::max(alpha, entry.score),
                TTFlag::Upper => beta = i16::min(beta, entry.score)
            }
            if alpha >= beta { return entry.score; }
        }
    }
    
    if depth == 0 {
        return quiescence(board, alpha, beta);
    }

    if  allow_null_move && board.has_non_pawn_pieces() && beta < INF && depth > 3 && !board.in_check(board.side) {
        let unmake = make_null_move(board);
        let null_move_score = -negamax(board, -beta, -beta + 1, depth - 3, ply + 1, false, state);
        unmake_null_move(board, &unmake);
        if null_move_score >= beta {
            state.tt.insert(TTEntry::new(board.hash, depth, null_move_score, TTFlag::Lower, None, board.fullmoves, state.tt.generation));
            return null_move_score;
        }
    }

    let mut movelist = generate_movelist(board, false);
    movelist.sort(board, predicted_best_move, &state.killers[ply as usize], state.history);
    let mut value = -INF;
    let mut moved = false;
    let original_alpha = alpha;
    let mut best_move = None;
    for (i, mv) in movelist.iter().enumerate() {
        let unmake = make_move(board, mv);
        if board.in_check(board.side.other()) {
            unmake_move(board, mv, &unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() {
            0
        } else if i > movelist.captures + 7 && depth >= 3 && !mv.is_promotion() && !board.in_check(board.side) {
            let low_score = -negamax(board, -alpha-1, -alpha, depth - 2, ply + 1, true, state);
            if low_score > alpha {
                -negamax(board, -beta, -alpha, depth - 1, ply + 1, true, state)
            } else {
                low_score
            }
        } else {
            let new_depth = if board.in_check(board.side) && ply < 2 * state.max_depth { depth } else { depth - 1 };
            -negamax(board, -beta, -alpha, new_depth, ply + 1, true, state)
        };
        unmake_move(board, mv, &unmake);
        if score > value {
            best_move = Some(mv);
            value = score;
        }
        alpha = i16::max(alpha, value);
        if alpha >= beta {
            state.beta_cutoff(mv, depth, ply);
            break;
        }
    }

    if !moved {
        if board.in_check(board.side) {
            return -(MATE_VAL - board.fullmoves as i16);
        } else {
            return 0;
        }
    }

    let flag = if original_alpha < value && value < beta {
        TTFlag::Exact
    } else if value >= beta {
        TTFlag::Lower
    } else {
        TTFlag::Upper
    };
    if board.repetitions <= 1 {
        state.tt.insert(TTEntry::new(board.hash, depth, value, flag, best_move, board.fullmoves, state.tt.generation));
    }
    value
}

fn quiescence(board: &mut Board, mut alpha: i16, beta: i16) -> i16 {
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
            -quiescence(board, -beta, -alpha)
        };
        unmake_move(board, mv, &unmake);
        value = i16::max(value, score);
        alpha = i16::max(alpha, value);
        if alpha >= beta {
            break;
        }
    }
    if !moved && in_check {
        return -(MATE_VAL - board.fullmoves as i16)
    }
    value
}
