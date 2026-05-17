use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::{
    bitboard::{Board, Piece, Side},
    movegen::{
        attacks::all_attacks,
        generator::generate_movelist,
        makemove::{is_in_check_after_move, make_move},
        r#move::Move,
    },
};

type SharedBoard = Arc<Mutex<Board>>;

#[derive(Serialize)]
struct GameState {
    fen: String,
    legal_moves: Vec<String>,
    game_over: bool,
    status_message: String,
}

#[derive(Deserialize)]
struct MoveRequest {
    uci: String,
}

async fn get_state(State(board): State<SharedBoard>) -> Json<GameState> {
    let board = board.lock().unwrap();
    Json(compute_state(&board))
}

async fn post_move(
    State(board): State<SharedBoard>,
    Json(req): Json<MoveRequest>,
) -> Json<GameState> {
    let mut board = board.lock().unwrap();
    let mv = Move::from_uci(&board, &req.uci);
    make_move(&mut board, mv);
    Json(compute_state(&board))
}

fn compute_state(board: &Board) -> GameState {
    let fen = board.to_fen();
    let pseudo_legal = generate_movelist(board);
    let legal_moves: Vec<String> = pseudo_legal
        .iter()
        .filter(|mv| {
            let mut copy = board.clone();
            make_move(&mut copy, *mv);
            !is_in_check_after_move(&copy)
        })
        .map(|mv| mv.to_uci())
        .collect();

    let game_over = legal_moves.is_empty();
    let status_message = if game_over {
        let king_bb = board.pieces[Piece::king(board.side) as usize];
        let in_check = all_attacks(board, board.side.other()) & king_bb != 0;
        if in_check {
            let winner = if board.side == Side::White { "Black" } else { "White" };
            format!("{} wins by checkmate!", winner)
        } else {
            "Stalemate! It's a draw.".to_string()
        }
    } else {
        String::new()
    };

    GameState { fen, legal_moves, game_over, status_message }
}

async fn new_game(State(board): State<SharedBoard>) -> Json<GameState> {
    let mut board = board.lock().unwrap();
    *board = Board::starting_position();
    Json(compute_state(&board))
}

pub async fn run() {
    let board = Arc::new(Mutex::new(Board::starting_position()));
    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/state", get(get_state))
        .route("/api/move", post(post_move))
        .route("/api/new", post(new_game))
        .with_state(board);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Bitchess running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}
