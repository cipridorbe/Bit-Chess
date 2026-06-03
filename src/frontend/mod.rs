use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::{
    bitboard::{Board, Piece, Side},
    movegen::{
        attacks::all_attacks,
        generator::generate_movelist,
        makemove::make_move,
        r#move::Move,
    },
    search::{search, tt::TT},
};

struct AppState {
    board: Board,
    player_side: Side,
    tt: Arc<TT>,
    history: Box<[[i16; 64]; 64]>,
    counter_move: Box<[[Option<Move>; 64]; 64]>,
    think_time_ms: Option<u64>,
}

type SharedState = Arc<Mutex<AppState>>;

#[derive(Serialize)]
struct GameState {
    fen: String,
    legal_moves: Vec<String>,
    game_over: bool,
    status_message: String,
    player_side: String,
    last_move: Option<String>,
}

#[derive(Deserialize)]
struct MoveRequest {
    uci: String,
}

#[derive(Deserialize)]
struct NewGameRequest {
    side: String,
}

#[derive(Serialize)]
struct SettingsState {
    think_time_ms: Option<u64>,
}

#[derive(Deserialize)]
struct SettingsRequest {
    think_time_ms: Option<u64>,
}

fn bot_search(s: &mut AppState) -> Option<Move> {
    let stop = Arc::new(AtomicBool::new(false));
    let depth = match s.think_time_ms {
        Some(ms) => {
            let stop_timer = Arc::clone(&stop);
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(ms));
                stop_timer.store(true, Ordering::Relaxed);
            });
            64u8
        }
        None => 10u8,
    };
    search(&stop, &mut s.board, depth, &mut s.tt, &mut s.history, &mut s.counter_move)
}

async fn get_state(State(state): State<SharedState>) -> Json<GameState> {
    let guard = state.lock().unwrap();
    Json(compute_state(&guard.board, guard.player_side, None))
}

async fn post_move(
    State(state): State<SharedState>,
    Json(req): Json<MoveRequest>,
) -> Json<GameState> {
    let mut guard = state.lock().unwrap();
    let mv = Move::from_uci(&guard.board, &req.uci);
    make_move(&mut guard.board, mv);

    let mut last_move = None;
    if guard.board.side != guard.player_side {
        let s = &mut *guard;
        if let Some(bot_mv) = bot_search(s) {
            last_move = Some(bot_mv.to_uci());
            make_move(&mut guard.board, bot_mv);
        }
    }

    Json(compute_state(&guard.board, guard.player_side, last_move))
}

fn compute_state(board: &Board, player_side: Side, last_move: Option<String>) -> GameState {
    let fen = board.to_fen();
    let pseudo_legal = generate_movelist(board, false);
    let legal_moves: Vec<String> = pseudo_legal
        .iter()
        .filter(|mv| {
            let mut copy = board.clone();
            make_move(&mut copy, *mv);
            !copy.in_check(copy.side.other())
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
    GameState { fen, legal_moves, game_over, status_message, player_side, last_move }
}

async fn new_game(
    State(state): State<SharedState>,
    Json(req): Json<NewGameRequest>,
) -> Json<GameState> {
    let mut guard = state.lock().unwrap();
    guard.board = Board::starting_position();
    guard.player_side = if req.side == "w" { Side::White } else { Side::Black };

    let mut last_move = None;
    if guard.player_side == Side::Black {
        let s = &mut *guard;
        if let Some(bot_mv) = bot_search(s) {
            last_move = Some(bot_mv.to_uci());
            make_move(&mut guard.board, bot_mv);
        }
    }

    Json(compute_state(&guard.board, guard.player_side, last_move))
}

async fn get_settings(State(state): State<SharedState>) -> Json<SettingsState> {
    let guard = state.lock().unwrap();
    Json(SettingsState { think_time_ms: guard.think_time_ms })
}

async fn post_settings(
    State(state): State<SharedState>,
    Json(req): Json<SettingsRequest>,
) -> Json<SettingsState> {
    let mut guard = state.lock().unwrap();
    guard.think_time_ms = req.think_time_ms;
    Json(SettingsState { think_time_ms: guard.think_time_ms })
}

pub async fn run() {
    let state = Arc::new(Mutex::new(AppState {
        board: Board::starting_position(),
        player_side: Side::White,
        tt: Arc::new(TT::new(22, 2)),
        history: Box::new([[0; 64]; 64]),
        counter_move: Box::new([[None; 64]; 64]),
        think_time_ms: None,
    }));

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/state", get(get_state))
        .route("/api/move", post(post_move))
        .route("/api/new", post(new_game))
        .route("/api/settings", get(get_settings).post(post_settings))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Bitchess running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}
