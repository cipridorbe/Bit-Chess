/*
Implementation of negamax, used as the main search algorithm.
*/

use serde::de;

use crate::{bitboard::{Board, Piece}, eval::relative_eval, movegen::{attacks::{all_attacks, is_in_check}, generator::generate_movelist, makemove::{is_in_check_after_move, make_move, unmake_move}, r#move::Move}, search::tt::{TT, TTEntry, TTFlag}};

const MATE_VAL: f32 = 1000000.;

pub fn search(board: &mut Board, depth: u8, tt: &mut TT) -> Option<Move> {
    let tt_best = tt.find(board.hash).and_then(|e| e.best_move);
    let mut best_score = f32::NEG_INFINITY;
    let mut best_move = None;
    let mut alpha = f32::NEG_INFINITY;

    let mut movelist = generate_movelist(&board, false);
    movelist.sort_mvvlva(&board, tt_best);
    for mv in movelist.iter() {
        let unmake = make_move(board, mv);
        if is_in_check_after_move(&board) {
            unmake_move(board, mv, unmake);
            continue;
        }
        let score = if board.is_rule_draw() {
            0.
        } else {
            -negamax(board, f32::NEG_INFINITY, -alpha, depth, tt)
        };
        unmake_move(board, mv, unmake);
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
            alpha = score;
        }
    }

    tt.insert(TTEntry::new(board.hash, depth, best_score, TTFlag::Exact, best_move, board.fullmoves));

    best_move
}

pub fn negamax(board: &mut Board, mut alpha: f32, mut beta: f32, depth: u8, tt: &mut TT) -> f32 {
    let mut predicted_best_move = None;
    if let Some(entry) = tt.find(board.hash) {
        predicted_best_move = entry.best_move;
        if entry.depth >= depth {
            match entry.flag {
                TTFlag::Exact => {
                    if entry.score.abs() > MATE_VAL / 10. {
                        let delta = board.fullmoves as f32 - entry.full_moves as f32;
                        return entry.score - entry.score.signum() * delta;
                    } else {
                        return entry.score;
                    }
                },
                TTFlag::Lower => alpha = f32::max(alpha, entry.score),
                TTFlag::Upper => beta = f32::min(beta, entry.score)
            }
            if alpha >= beta { return entry.score; }
        }
    }
    
    if depth == 0 {
        return quiescence(board, alpha, beta);
    }

    let mut movelist = generate_movelist(board, false);
    movelist.sort_mvvlva(board, predicted_best_move);
    let mut value = f32::MIN;
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
            0.
        } else {
            -negamax(board, -beta, -alpha, depth - 1, tt)
        };
        unmake_move(board, mv, unmake);
        if score > value {
            best_move = Some(mv);
            value = score;
        }
        alpha = f32::max(alpha, value);
        if alpha >= beta {
            break;
        }
    }
    if !moved {
        let attacks = all_attacks(board, board.side.other());
        if is_in_check(attacks, board.pieces[Piece::king(board.side) as usize]) {
            return -(MATE_VAL - board.fullmoves as f32);
        } else {
            return 0.;
        }
    }
    let flag = if original_alpha < value && value < beta {
        TTFlag::Exact
    } else if value >= beta {
        TTFlag::Lower
    } else {
        TTFlag::Upper
    };
    tt.insert(TTEntry::new(board.hash, depth, value, flag, best_move, board.fullmoves));
    value
}

fn quiescence(board: &mut Board, mut alpha: f32, beta: f32) -> f32 {
    let attacks = all_attacks(board, board.side.other());
    let in_check = is_in_check(attacks, board.pieces[Piece::king(board.side) as usize]);
    let stand_pat = if !in_check {
        let eval = relative_eval(board);
        if eval >= beta {
            return eval;
        }
        alpha = f32::max(alpha, eval);
        eval
    } else {
        f32::MIN
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
            0.
        } else {
            -quiescence(board, -beta, -alpha)
        };
        unmake_move(board, mv, unmake);
        value = f32::max(value, score);
        alpha = f32::max(alpha, value);
        if alpha >= beta {
            break;
        }
    }
    if !moved && in_check {
        return -(MATE_VAL - board.fullmoves as f32)
    }
    value
}
