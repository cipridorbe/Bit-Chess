use std::io::{self, BufRead, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use bitchess::{
    bitboard::{Board, Side},
    game::Game,
    movegen::makemove::make_move,
    movegen::r#move::Move,
};

fn parse_depth(tokens: &[&str]) -> u8 {
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == "depth" {
            if let Some(d) = tokens.get(i + 1).and_then(|s| s.parse().ok()) {
                return d;
            }
        }
        i += 1;
    }
    u8::MAX
}

fn parse_time(tokens: &[&str], side: Side) -> Option<u64> {
    let mut movetime: Option<u64> = None;
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: u64 = 0;
    let mut binc: u64 = 0;

    let mut i = 1;
    while i < tokens.len() {
        match tokens[i] {
            "movetime" => { movetime = tokens.get(i + 1).and_then(|s| s.parse().ok()); i += 2; }
            "wtime"    => { wtime    = tokens.get(i + 1).and_then(|s| s.parse().ok()); i += 2; }
            "btime"    => { btime    = tokens.get(i + 1).and_then(|s| s.parse().ok()); i += 2; }
            "winc"     => { winc     = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0); i += 2; }
            "binc"     => { binc     = tokens.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0); i += 2; }
            _ => { i += 1; }
        }
    }

    if let Some(mt) = movetime {
        return Some(mt);
    }

    let (my_time, my_inc) = match side {
        Side::White => (wtime?, winc),
        Side::Black => (btime?, binc),
    };

    let budget = my_time / 20 + my_inc / 2;
    let cap = my_time.saturating_sub(50).max(10);
    Some(budget.clamp(10, cap))
}

fn end_search(stop: &Arc<AtomicBool>, handle: &mut Option<thread::JoinHandle<Game>>, game: &mut Option<Game>) {
    if let Some(h) = handle.take() {
        stop.store(true, Ordering::Relaxed);
        *game = Some(h.join().unwrap());
    }
}

fn main() {
    let stdin = io::stdin();
    let mut game: Option<Game> = Some(Game::new());
    let mut search_handle: Option<thread::JoinHandle<Game>> = None;
    let mut stop = Arc::new(AtomicBool::new(false));

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "uci" => {
                println!("id name Bitchess");
                println!("id author Cipriano Dorbessan");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "ucinewgame" => {
                end_search(&stop, &mut search_handle, &mut game);
                game = Some(Game::new());
            }
            "position" => {
                end_search(&stop, &mut search_handle, &mut game);
                let g = game.as_mut().unwrap();
                let mut i = 1;
                match tokens.get(i).copied() {
                    Some("startpos") => {
                        g.board = Board::starting_position();
                        i += 1;
                    }
                    Some("fen") => {
                        i += 1;
                        if tokens.len() < i + 6 {
                            continue;
                        }
                        g.board = Board::from_fen(&tokens[i..i + 6].join(" "));
                        i += 6;
                    }
                    _ => continue,
                }
                if tokens.get(i).copied() == Some("moves") {
                    i += 1;
                    for mv_str in &tokens[i..] {
                        let mv = Move::from_uci(&g.board, mv_str);
                        make_move(&mut g.board, mv);
                    }
                }
            }
            "go" => {
                end_search(&stop, &mut search_handle, &mut game);

                let side = game.as_ref().unwrap().board.side;
                let time_ms = parse_time(&tokens, side);
                let depth = parse_depth(&tokens);
                let is_infinite = depth == u8::MAX && time_ms.is_none();

                stop = Arc::new(AtomicBool::new(false));

                if let Some(ms) = time_ms {
                    let s = Arc::clone(&stop);
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(ms));
                        s.store(true, Ordering::Relaxed);
                    });
                }

                let stop_search = Arc::clone(&stop);
                let mut g = game.take().unwrap();
                let handle = thread::spawn(move || {
                    let mv = g.find_best_move(depth, &stop_search);
                    println!("bestmove {}", mv.map(|m| m.to_uci()).unwrap_or_else(|| "0000".into()));
                    io::stdout().flush().unwrap();
                    g
                });

                if is_infinite {
                    search_handle = Some(handle);
                } else {
                    game = Some(handle.join().unwrap());
                }
            }
            "stop" => {
                stop.store(true, Ordering::Relaxed);
                end_search(&stop, &mut search_handle, &mut game);
            }
            "ponderhit" => {}
            "quit" => {
                end_search(&stop, &mut search_handle, &mut game);
                break;
            }
            _ => {}
        }
    }
}
