use std::io::{self, BufRead, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use bitchess::{
    movegen::r#move::Move,
    repr::{board::Board, colour::Colour, game::Game},
};

fn parse_depth(tokens: &[&str]) -> Option<u8> {
    for i in 0..tokens.len().saturating_sub(1) {
        if tokens[i] == "depth" {
            return tokens[i + 1].parse().ok();
        }
    }
    None
}

fn parse_movetime(tokens: &[&str], colour: Colour) -> Option<Duration> {
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
        return Some(Duration::from_millis(mt));
    }

    let (my_time, my_inc) = match colour {
        Colour::White => (wtime?, winc),
        Colour::Black => (btime?, binc),
    };

    let budget = Game::calculate_move_time_basic(
        Duration::from_millis(my_time),
        Duration::from_millis(my_inc),
    );
    let cap = Duration::from_millis(my_time.saturating_sub(50).max(10));
    Some(budget.clamp(Duration::from_millis(10), cap))
}

fn end_search(stop: &Arc<AtomicBool>, handle: &mut Option<thread::JoinHandle<Game>>, game: &mut Option<Game>) {
    stop.store(true, Ordering::Relaxed);
    if let Some(h) = handle.take() {
        *game = Some(h.join().unwrap());
    }
}

fn main() {
    let stdin = io::stdin();
    let mut game: Option<Game> = Some(Game::new_infinite(None, None));
    let mut search_handle: Option<thread::JoinHandle<Game>> = None;
    let mut stop = Arc::new(AtomicBool::new(false));

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        match tokens[0] {
            "uci" => {
                println!("id name Bitchess");
                println!("id author Cipriano Dorbessan");
                println!("uciok");
                io::stdout().flush().unwrap();
            }
            "isready" => {
                println!("readyok");
                io::stdout().flush().unwrap();
            }
            "ucinewgame" => {
                end_search(&stop, &mut search_handle, &mut game);
                game = Some(Game::new_infinite(None, None));
            }
            "position" => {
                end_search(&stop, &mut search_handle, &mut game);
                let g = game.as_mut().unwrap();
                let mut i = 1;
                let mut board = match tokens.get(i).copied() {
                    Some("startpos") => { i += 1; Board::starting_position() }
                    Some("fen") => {
                        i += 1;
                        if tokens.len() < i + 6 { continue; }
                        let b = Board::from_fen(&tokens[i..i + 6].join(" "));
                        i += 6;
                        b
                    }
                    _ => continue,
                };
                if tokens.get(i).copied() == Some("moves") {
                    i += 1;
                    for mv_str in &tokens[i..] {
                        let mv = Move::from_uci(&board, mv_str);
                        let _ = board.makemove(mv);
                    }
                }
                g.set_board(board);
            }
            "go" => {
                end_search(&stop, &mut search_handle, &mut game);
                stop = Arc::new(AtomicBool::new(false));

                let depth = parse_depth(&tokens);
                let is_infinite = tokens.contains(&"infinite") || (depth.is_none() && !tokens.contains(&"movetime") && !tokens.contains(&"wtime"));
                let colour = game.as_ref().unwrap().colour();
                let time = parse_movetime(&tokens, colour);

                let stop_clone = Arc::clone(&stop);
                let mut g = game.take().unwrap();
                let handle = thread::spawn(move || {
                    let (mv, _eval, _reached_depth, _nodes) = g.find_best_move(depth, time, Some(stop_clone));
                    let bestmove = format!("bestmove {}", mv.map(|m| m.to_uci()).unwrap_or_else(|| "0000".to_string()));
                    println!("{}", bestmove);
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
                end_search(&stop, &mut search_handle, &mut game);
            }
            "quit" => {
                end_search(&stop, &mut search_handle, &mut game);
                break;
            }
            _ => {}
        }
    }
}
