/*
Implementation of negamax, used as the main search algorithm.
*/

use serde::de;

use crate::{bitboard::{Board, Piece}, eval::relative_eval, movegen::{attacks::{all_attacks, is_in_check}, generator::generate_movelist, makemove::{is_in_check_after_move, make_move, unmake_move}, r#move::Move}, search::{state::SearchState, tt::{TT, TTEntry, TTFlag}}};

const MATE_VAL: i16 = 30000;
const MATE_CUTOFF: i16 = 29000;
pub const INF: i16 = 31000;

pub static mut NODE_COUNT: u64 = 0;

pub fn search(board: &mut Board, max_depth: u8, tt: &mut TT, history: &mut [[i16; 64]; 64]) -> Option<Move> {
    let mut state = SearchState::new_search(tt, history);
    let mut best_move = None;
    for depth in 1..=max_depth {
        let tt_best = state.tt.find(board.hash).and_then(|e| e.best_move);
        let mut best_score = -INF;
        let mut alpha = -INF;

        let mut movelist = generate_movelist(&board, false);
        movelist.sort(&board, tt_best, &state.killers[0], state.history);
        for mv in movelist.iter() {
            let unmake = make_move(board, mv);
            if is_in_check_after_move(&board) {
                unmake_move(board, mv, unmake);
                continue;
            }
            let score = if board.is_rule_draw() {
                0
            } else {
                -negamax(board, -INF, -alpha, depth, 0, &mut state)
            };
            unmake_move(board, mv, unmake);
            if score > best_score {
                best_score = score;
                best_move = Some(mv);
                alpha = score;
            }
        }

        state.tt.insert(TTEntry::new(board.hash, depth, best_score, TTFlag::Exact, best_move, board.fullmoves, state.tt.generation));
    }
    best_move
}

pub fn negamax(board: &mut Board, mut alpha: i16, mut beta: i16, depth: u8, ply: u8, state: &mut SearchState) -> i16 {
    unsafe { NODE_COUNT += 1; }
    let mut predicted_best_move = None;
    if let Some(entry) = state.tt.find(board.hash) {
        predicted_best_move = entry.best_move;
        if entry.depth >= depth {
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

    let mut movelist = generate_movelist(board, false);
    movelist.sort(board, predicted_best_move, &state.killers[ply as usize], state.history);
    let mut value = -INF;
    let mut moved = false;
    let original_alpha = alpha;
    let mut best_move = None;
    for mv in movelist.iter() {
        let unmake = make_move(board, mv);
        if is_in_check_after_move(board) {
            unmake_move(board, mv, unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() {
            0
        } else {
            -negamax(board, -beta, -alpha, depth - 1, ply + 1, state)
        };
        unmake_move(board, mv, unmake);
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
        let attacks = all_attacks(board, board.side.other());
        if is_in_check(attacks, board.pieces[Piece::king(board.side) as usize]) {
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
    state.tt.insert(TTEntry::new(board.hash, depth, value, flag, best_move, board.fullmoves, state.tt.generation));
    value
}

fn quiescence(board: &mut Board, mut alpha: i16, beta: i16) -> i16 {
    unsafe { NODE_COUNT += 1; }
    let attacks = all_attacks(board, board.side.other());
    let in_check = is_in_check(attacks, board.pieces[Piece::king(board.side) as usize]);
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
        if is_in_check_after_move(board) {
            unmake_move(board, mv, unmake);
            continue;
        }
        moved = true;
        let score = if board.is_rule_draw() {
            0
        } else {
            -quiescence(board, -beta, -alpha)
        };
        unmake_move(board, mv, unmake);
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
