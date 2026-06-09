use crate::{bitboard::Board, movegen::{generator::generate_movelist, makemove::{make_move, unmake_move}, r#move::Move}, search::{search, tt::TT}};

pub struct Game {
    pub board: Board,
    pub tt: TT,
    pub history: [[i16; 64]; 64],
    pub counter_move: [[Option<Move>; 64]; 64],
}

impl Game {
    pub fn new() -> Self {
        Game {
            board: Board::starting_position(),
            tt: TT::new(23, 2),
            history: [[0; 64]; 64],
            counter_move: [[None; 64]; 64]
        }
    }

    pub fn from_fen(fen: &str) -> Self {
        Game {
            board: Board::from_fen(fen),
            tt: TT::new(23, 2),
            history: [[0; 64]; 64],
            counter_move: [[None; 64]; 64]
        }
    }

    pub fn find_best_move(&mut self, depth: u8) -> Option<Move> {
        search(&mut self.board, depth, &mut self.tt, &mut self.history, &mut self.counter_move)
    }

    pub fn is_game_over(&mut self) -> bool {
        let movelist = generate_movelist(&self.board, false);
        for mv in movelist.iter() {
            let unmake = make_move(&mut self.board, mv);
            if !self.board.in_check(self.board.side.other()) {
                unmake_move(&mut self.board, mv, &unmake);
                return false;
            }
            unmake_move(&mut self.board, mv, &unmake);
        }
        true
    }
}
