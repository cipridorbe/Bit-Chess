/*
Implementation of negamax, used as the main search algorithm.
*/

use crate::{bitboard::{Board, Piece}, eval::relative_eval, movegen::{attacks::{all_attacks, is_in_check}, generator::generate_movelist, makemove::{is_in_check_after_move, make_move, unmake_move}, r#move::Move}};

pub fn search(board: &Board) -> Option<Move> {
    let mut board = board.clone();
    let mut best_score = f32::NEG_INFINITY;
    let mut best_move = None;

    for mv in generate_movelist(&board).iter() {
        let unmake = make_move(&mut board, mv);
        if is_in_check_after_move(&board) {
            unmake_move(&mut board, mv, unmake);
            continue;
        }
        let score = -negamax(&mut board, f32::NEG_INFINITY, f32::INFINITY, 4);
        unmake_move(&mut board, mv, unmake);
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move
}

pub fn negamax(board: &mut Board, mut alpha: f32, beta: f32, depth: u8) -> f32 {
    if depth == 0 {
        return relative_eval(board);
    }

    let movelist = generate_movelist(board);
    let mut value = f32::MIN;
    let mut moved = false;
    for mv in movelist.iter() {
        let unmake = make_move(board, mv);
        if is_in_check_after_move(board) {
            unmake_move(board, mv, unmake);
            continue;
        }
        moved = true;
        let score = -negamax(board, -beta, -alpha, depth - 1);
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
            // checkmate
            return -(1000000. + depth as f32);
        } else {
            // stalemate
            return 0.;
        }
    }
    return value;
}