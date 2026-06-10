use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

use crate::{eval::Eval, movegen::r#move::Move, repr::{board::Board, colour::Colour, piece::Piece, square::Square}, search::{MAX_PLY, negamax::search, state::SearchState}};

pub struct Game {
    board: Board,
    // If None, game has no time limit
    total_time: Option<Duration>,
    time_increment: Duration,
    time_left: [Duration; 2],
    search_state: SearchState,
}

impl Game {
    /// Creates a new game from the starting position with no time limit
    pub fn new_infinite(tt_bits: Option<u8>, tt_generation_cutoff: Option<u8>, pawn_table_bits: Option<u8>) -> Self {
        let tt_bits = tt_bits.unwrap_or(23);
        let tt_generation_cutoff = tt_generation_cutoff.unwrap_or(2);
        let pawn_table_bits = pawn_table_bits.unwrap_or(18);
        Game {
            board: Board::starting_position(),
            total_time: None,
            time_increment: Duration::new(0, 0),
            time_left: [Duration::new(0, 0); 2],
            search_state: SearchState::new(tt_bits, tt_generation_cutoff, pawn_table_bits)
        }
    }

    pub fn new_finite(total_time: Duration, time_increment: Duration, tt_bits: Option<u8>, tt_generation_cutoff: Option<u8>, pawn_table_bits: Option<u8>) -> Self {
        if total_time <= Duration::new(0, 0) || time_increment < Duration::new(0, 0) {
            panic!("Cannot have negative time");
        }
        let tt_bits = tt_bits.unwrap_or(23);
        let tt_generation_cutoff = tt_generation_cutoff.unwrap_or(2);
        let pawn_table_bits = pawn_table_bits.unwrap_or(18);
        Game {
            board: Board::starting_position(),
            total_time: Some(total_time),
            time_increment: time_increment,
            time_left: [total_time; 2],
            search_state: SearchState::new(tt_bits, tt_generation_cutoff, pawn_table_bits)
        }
    }

    pub fn new_10min() -> Self {
        Game::new_finite(Duration::from_secs(60*10), Duration::from_secs(0), None, None, None)
    }

    pub fn new_3min() -> Self {
        Game::new_finite(Duration::from_secs(60*3), Duration::from_secs(1), None, None, None)
    }

    pub fn new_1min() -> Self {
        Game::new_finite(Duration::from_secs(60*1), Duration::from_secs(2), None, None, None)
    }

    pub fn game_over_score(&mut self) -> Option<Eval> {
        if self.board.is_rule_draw() {
            return Some(0);
        }
        let num_pieces = self.board.occupied().count_ones();
        let bishops = self.board[Piece::WhiteBishop] | self.board[Piece::BlackBishop];
        if bishops.count_ones() == num_pieces - 2 {
            if bishops & Square::DARK_SQUARES == bishops {
                return Some(0);
            }
            if bishops & Square::LIGHT_SQUARES == bishops {
                return Some(0);
            }
        }

        let mut movelist = self.board.generate_movelist(false);
        let mut i = 0;
        while i < movelist.length {
            let mv = movelist[i];
            if self.board.is_legal(mv) {
                return None;
            }
            // have to make and unmake to prevent queen promotion stalemates
            let unmake = self.board.makemove(mv);
            self.board.unmakemove(mv, 0, unmake, Some(&mut movelist));
            i += 1;
        }
        if self.board.in_check() {
            return match self.board.colour {
                Colour::White => Some(-(1 + self.board.fullmoves as i16)),
                Colour::Black => Some(1 + self.board.fullmoves as i16 ),
            }
        }
        Some(0)
    }

    /// Finds the best move to make.
    /// - `depth`: max search depth; `None` uses MAX_PLY
    /// - `time`: explicit time budget; `None` uses the game's own time tracking (panics for infinite games)
    /// - `stop_flag`: optional external flag that can interrupt the search early (e.g. UCI stop)
    pub fn find_best_move(&mut self, depth: Option<u8>, time: Option<Duration>, stop_flag: Option<Arc<AtomicBool>>) -> (Option<Move>, Eval, u8, u64) {
        let depth = depth.unwrap_or(MAX_PLY);
        let internal_stop = Arc::new(AtomicBool::new(false));

        // Spawn timer if a time budget is available
        let timer_duration = time.map(|t| t.mul_f32(0.95))
            .or_else(|| self.total_time.map(|_| self.calculate_move_time()));
        if let Some(duration) = timer_duration {
            let flag = Arc::clone(&internal_stop);
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                flag.store(true, Ordering::Relaxed);
            });
        }

        // Bridge external stop flag to internal one
        if let Some(ext) = stop_flag {
            let flag = Arc::clone(&internal_stop);
            std::thread::spawn(move || {
                while !ext.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                flag.store(true, Ordering::Relaxed);
            });
        }

        search(&mut self.board, &mut self.search_state, depth, &internal_stop)
    }

    fn calculate_move_time(&self) -> Duration {
        self.time_left[self.board.colour as usize] / 20 + self.time_increment / 2
    }

    pub fn calculate_move_time_basic(time_left: Duration, increment: Duration) -> Duration {
        time_left / 20 + increment / 2
    }

    pub fn set_board(&mut self, board: Board) {
        self.board = board;
    }

    pub fn colour(&self) -> Colour {
        self.board.colour
    }
}