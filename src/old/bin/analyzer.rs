use axum::{http::StatusCode, routing::{get, post}, Json, Router, response::Html};
use serde::{Deserialize, Serialize};

use std::sync::{Arc, atomic::AtomicBool};

use bitchess::{
    bitboard::Board,
    movegen::{makemove::make_move, r#move::Move},
    search::{search, tt::{TTFlag, TT}},
};

#[derive(Deserialize)]
struct AnalyzeRequest {
    engine_side: String,
    start_fen: Option<String>,
    moves: Vec<String>,
}

#[derive(Serialize)]
struct TtInfo {
    depth: u8,
    score: i16,
    flag: &'static str,
    best_move: Option<String>,
    generation: u8,
}

#[derive(Serialize)]
struct HistoryEntry {
    from: String,
    to: String,
    score: i16,
}

#[derive(Serialize)]
struct AnalysisFrame {
    halfmove: usize,
    fen: String,
    move_played: Option<String>,
    is_engine_move: bool,
    eval: Option<i16>,
    best_move_found: Option<String>,
    matches_game: Option<bool>,
    tt_before: Option<TtInfo>,
    tt_after: Option<TtInfo>,
    history_top: Vec<HistoryEntry>,
}

fn sq_str(idx: usize) -> String {
    let file = (b'a' + (idx % 8) as u8) as char;
    let rank = (b'1' + (idx / 8) as u8) as char;
    format!("{}{}", file, rank)
}

fn flag_str(f: TTFlag) -> &'static str {
    match f { TTFlag::Exact => "Exact", TTFlag::Lower => "Lower", TTFlag::Upper => "Upper" }
}

fn tt_info(tt: &TT, hash: u64) -> Option<TtInfo> {
    tt.find(hash).map(|e| TtInfo {
        depth: e.depth,
        score: e.score,
        flag: flag_str(e.flag),
        best_move: e.best_move.map(|m| m.to_uci()),
        generation: e.generation,
    })
}

fn history_top(history: &[[i16; 64]; 64]) -> Vec<HistoryEntry> {
    let mut v: Vec<(usize, usize, i16)> = (0..64)
        .flat_map(|f| (0..64).map(move |t| (f, t, history[f][t])))
        .filter(|&(_, _, s)| s > 0)
        .collect();
    v.sort_unstable_by(|a, b| b.2.cmp(&a.2));
    v.truncate(12);
    v.into_iter().map(|(f, t, s)| HistoryEntry { from: sq_str(f), to: sq_str(t), score: s }).collect()
}

fn run_analysis(req: AnalyzeRequest) -> Vec<AnalysisFrame> {
    let mut board = match req.start_fen {
        Some(ref fen) => Board::from_fen(fen),
        None => Board::starting_position(),
    };
    let _engine_side = &req.engine_side;
    let mut tt = Arc::new(TT::new(22, 2));
    let mut history = Box::new([[0i16; 64]; 64]);
    let mut counter_move = Box::new([[None::<Move>; 64]; 64]);
    let mut frames = Vec::new();

    for (i, mv_str) in req.moves.iter().enumerate() {
        let fen = board.to_fen();
        let is_engine = fen.split(' ').nth(1) == Some(&req.engine_side);
        let hash = board.hash();

        let (eval, best_mv, matches, tt_before, tt_after, hist) = if is_engine {
            let before = tt_info(&tt, hash);
            let found = search(&Arc::new(AtomicBool::new(false)), &mut board, 6, &mut tt, &mut history, &mut counter_move);
            let after = tt_info(&tt, board.hash());
            let eval = after.as_ref().map(|e| e.score);
            let uci = found.map(|m| m.to_uci());
            let matches = uci.as_deref().map(|u| u == mv_str.as_str());
            let hist = history_top(&history);
            (eval, uci, matches, before, after, hist)
        } else {
            (None, None, None, None, None, vec![])
        };

        frames.push(AnalysisFrame {
            halfmove: i,
            fen,
            move_played: Some(mv_str.clone()),
            is_engine_move: is_engine,
            eval,
            best_move_found: best_mv,
            matches_game: matches,
            tt_before,
            tt_after,
            history_top: hist,
        });

        let mv = Move::from_uci(&board, mv_str);
        make_move(&mut board, mv);
    }

    frames.push(AnalysisFrame {
        halfmove: req.moves.len(),
        fen: board.to_fen(),
        move_played: None,
        is_engine_move: false,
        eval: None,
        best_move_found: None,
        matches_game: None,
        tt_before: None,
        tt_after: None,
        history_top: vec![],
    });

    frames
}

async fn handle_analyze(
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<Vec<AnalysisFrame>>, (StatusCode, String)> {
    let result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_analysis(req)))
    })
    .await
    .unwrap();

    result.map(Json).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid move in sequence — make sure moves are in UCI format (e.g. e2e4) \
             and match the actual board position."
                .to_string(),
        )
    })
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("../../static/analyzer.html"))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/analyze", post(handle_analyze));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Bitchess analyzer at http://localhost:3001");
    axum::serve(listener, app).await.unwrap();
}
