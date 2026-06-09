/*
Implementation of negamax, used as the main search algorithm.
*/

use crate::{bitboard::{Board, Piece}, eval::relative_eval, movegen::{attacks::{all_attacks, is_in_check}, generator::generate_movelist, makemove::{is_in_check_after_move, make_move, unmake_move}, r#move::Move}};

const MATE_VAL: f32 = 1000000.;

pub fn search(board: &mut Board, depth: u8) -> Option<Move> {
    let mut best_score = f32::NEG_INFINITY;
    let mut best_move = None;

    let mut movelist = generate_movelist(&board, false);
    movelist.sort_mvvlva(&board);
    for mv in movelist.iter() {
        let unmake = make_move(board, mv);
        if is_in_check_after_move(&board) {
            unmake_move(board, mv, unmake);
            continue;
        }
        let score = if board.is_rule_draw() {
            0.
        } else {
            -negamax(board, f32::NEG_INFINITY, f32::INFINITY, depth)
        };
        unmake_move(board, mv, unmake);
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move
}

pub fn negamax(board: &mut Board, mut alpha: f32, beta: f32, depth: u8) -> f32 {
    if depth == 0 {
        return quiescence(board, alpha, beta);
    }

    let mut movelist = generate_movelist(board, false);
    movelist.sort_mvvlva(board);
    let mut value = f32::MIN;
    let mut moved = false;
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
            -negamax(board, -beta, -alpha, depth - 1)
        };
        unmake_move(board, mv, unmake);
        value = f32::max(value, score);
        alpha = f32::max(alpha, value);
        if alpha >= beta {
            break;
        }
    }
    if !moved {
        let attacks = all_attacks(board, board.side.other());
        if is_in_check(attacks, board.pieces[Piece::king(board.side) as usize]) {
            return -(MATE_VAL + depth as f32);
        } else {
            return 0.;
        }
    }
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
    movelist.sort_mvvlva(board);
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
        return -(MATE_VAL / 2. - board.fullmoves as f32)
    }
    value
}
