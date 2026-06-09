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
    search::search,
};

struct AppState {
    board: Board,
    player_side: Side,
}

type SharedState = Arc<Mutex<AppState>>;

#[derive(Serialize)]
struct GameState {
    fen: String,
    legal_moves: Vec<String>,
    game_over: bool,
    status_message: String,
    player_side: String,
}

#[derive(Deserialize)]
struct MoveRequest {
    uci: String,
}

#[derive(Deserialize)]
struct NewGameRequest {
    side: String,
}

async fn get_state(State(state): State<SharedState>) -> Json<GameState> {
    let state = state.lock().unwrap();
    Json(compute_state(&state.board, state.player_side))
}

async fn post_move(
    State(state): State<SharedState>,
    Json(req): Json<MoveRequest>,
) -> Json<GameState> {
    let mut state = state.lock().unwrap();
    let mv = Move::from_uci(&state.board, &req.uci);
    make_move(&mut state.board, mv);

    // Run bot if it's its turn (search returns None when no legal moves)
    if state.board.side != state.player_side {
        if let Some(bot_mv) = search(&state.board, 4) {
            make_move(&mut state.board, bot_mv);
        }
    }

    Json(compute_state(&state.board, state.player_side))
}

fn compute_state(board: &Board, player_side: Side) -> GameState {
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

    let player_side = if player_side == Side::White { "w" } else { "b" }.to_string();
    GameState { fen, legal_moves, game_over, status_message, player_side }
}

async fn new_game(
    State(state): State<SharedState>,
    Json(req): Json<NewGameRequest>,
) -> Json<GameState> {
    let mut state = state.lock().unwrap();
    state.board = Board::starting_position();
    state.player_side = if req.side == "w" { Side::White } else { Side::Black };

    // If player chose black, bot plays first as white
    if state.player_side == Side::Black {
        if let Some(bot_mv) = search(&state.board, 4) {
            make_move(&mut state.board, bot_mv);
        }
    }

    Json(compute_state(&state.board, state.player_side))
}

pub async fn run() {
    let state = Arc::new(Mutex::new(AppState {
        board: Board::starting_position(),
        player_side: Side::White,
    }));

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/state", get(get_state))
        .route("/api/move", post(post_move))
        .route("/api/new", post(new_game))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Bitchess running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}
