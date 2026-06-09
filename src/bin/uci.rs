use std::io::{self, BufRead, Write};

use bitchess::{
    bitboard::Board,
    movegen::makemove::make_move,
    movegen::r#move::Move,
    search::search,
};

fn main() {
    let stdin = io::stdin();
    let mut board = Board::starting_position();

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
                board = Board::starting_position();
            }
            "position" => {
                let mut i = 1;
                match tokens.get(i).copied() {
                    Some("startpos") => {
                        board = Board::starting_position();
                        i += 1;
                    }
                    Some("fen") => {
                        i += 1;
                        let fen = tokens[i..i + 6].join(" ");
                        board = Board::from_fen(&fen);
                        i += 6;
                    }
                    _ => continue,
                }
                if tokens.get(i).copied() == Some("moves") {
                    i += 1;
                    for mv_str in &tokens[i..] {
                        let mv = Move::from_uci(&board, mv_str);
                        make_move(&mut board, mv);
                    }
                }
            }
            "go" => {
                let mut depth = 6u8;
                let mut i = 1;
                while i < tokens.len() {
                    if tokens[i] == "depth" {
                        if let Some(d) = tokens.get(i + 1).and_then(|s| s.parse().ok()) {
                            depth = d;
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                match search(&mut board, depth) {
                    Some(mv) => print!("bestmove {}\n", mv.to_uci()),
                    None => print!("bestmove 0000\n"),
                }
                io::stdout().flush().unwrap();
            }
            "stop" | "ponderhit" => {}
            "quit" => break,
            _ => {}
        }
    }
}
