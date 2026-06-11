use crate::{repr::{board::Board, colour::Colour}, search::MAX_PLY};

pub mod pst;
pub mod pawn;
pub mod king;

pub type Eval = i16;

pub const INF: Eval = 31000;
pub const MATE: Eval = INF - 1;
pub const MATE_CUTOFF: Eval = MATE - MAX_PLY as Eval * 2;

pub const EVAL_BONUS_DELTA: Eval = 200;

pub fn partial_relative_eval(board: &Board, alpha: Eval, beta: Eval) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    if partial_eval <= alpha - EVAL_BONUS_DELTA || partial_eval >= beta + EVAL_BONUS_DELTA {
        mult * partial_eval
    } else {
        mult * (partial_eval + bonus_eval(board))
    }
}

pub fn relative_eval(board: &Board) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    mult * (partial_eval + bonus_eval(board))
}

pub fn bonus_eval(board: &Board) -> Eval {
    0
}

fn phase_eval(phase_unbounded: u8, mg_eval: Eval, eg_eval: Eval) -> Eval {
    let phase = phase_unbounded.min(24) as i32;
    ((mg_eval as i32 * phase + eg_eval as i32 * (24 - phase)) / 24) as Eval
}

#[cfg(test)]
mod tests {
    use crate::{movegen::r#move::Move, repr::board::Board};

    /// Asserts that all incrementally-maintained fields on `board` match
    /// a fresh parse of its own FEN. Tests hash, pawn_hash, mg_eval,
    /// eg_eval, and phase_unbounded.
    fn assert_board_consistent(board: &Board) {
        let fen = board.to_fen();
        let fresh = Board::from_fen(&fen);
        assert_eq!(board.state.hash, fresh.state.hash,
            "hash mismatch after sequence ending in {fen}");
        assert_eq!(board.state.pawn_hash, fresh.state.pawn_hash,
            "pawn_hash mismatch after sequence ending in {fen}");
        assert_eq!(board.state.mg_eval, fresh.state.mg_eval,
            "mg_eval mismatch after sequence ending in {fen}");
        assert_eq!(board.state.eg_eval, fresh.state.eg_eval,
            "eg_eval mismatch after sequence ending in {fen}");
        assert_eq!(board.state.phase_unbounded, fresh.state.phase_unbounded,
            "phase mismatch after sequence ending in {fen}");
    }

    fn play(board: &mut Board, uci: &str) {
        let mv = Move::from_uci(board, uci);
        board.makemove(mv);
        assert_board_consistent(board);
    }

    #[test]
    fn test_eval_starting_position() {
        let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_board_consistent(&board);
        assert_eq!(board.state.mg_eval, 0, "starting position should be symmetric");
        assert_eq!(board.state.eg_eval, 0, "starting position should be symmetric");
    }

    #[test]
    fn test_eval_quiet_moves() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        play(&mut board, "e2e4");
        play(&mut board, "e7e5");
        play(&mut board, "g1f3");
        play(&mut board, "b8c6");
    }

    #[test]
    fn test_eval_pawn_capture() {
        let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2");
        play(&mut board, "e4d5");
        play(&mut board, "c7c6");
        play(&mut board, "d5c6");
    }

    #[test]
    fn test_eval_enpassant() {
        let mut board = Board::from_fen("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3");
        play(&mut board, "e5f6");
    }

    #[test]
    fn test_eval_promotion() {
        let mut board = Board::from_fen("8/P7/8/8/8/8/8/4K1k1 w - - 0 1");
        play(&mut board, "a7a8q");
    }

    #[test]
    fn test_eval_promotion_capture() {
        let mut board = Board::from_fen("1n6/P7/8/8/8/8/8/4K1k1 w - - 0 1");
        play(&mut board, "a7b8q");
    }

    #[test]
    fn test_eval_castling() {
        let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
        play(&mut board, "e1g1");
        play(&mut board, "e8c8");
    }

    #[test]
    fn test_eval_sequence_with_captures() {
        // Italian Game: Nxe5 Nxe5 Qh5 h6 Qxe5+
        let mut board = Board::from_fen("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4");
        play(&mut board, "f3e5");  // Nxe5
        play(&mut board, "c6e5");  // Nxe5
        play(&mut board, "d1h5");  // Qh5
        play(&mut board, "h7h6");  // h6
        play(&mut board, "h5e5");  // Qxe5+
    }

    #[test]
    fn test_move_sort_order() {
        use crate::search::state::SearchState;
        let board = Board::from_fen("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4");
        let state = SearchState::new(16, 2, 14);
        let movelist = board.generate_movelist(false);
        let mut scores = movelist.score(&board, &state, None, 0);
        let mut movelist = movelist;
        movelist.sort(&mut scores);
        for i in 1..movelist.length {
            assert!(scores[i - 1] >= scores[i],
                "moves not sorted at index {i}: score[{}]={} < score[{}]={}",
                i - 1, scores[i - 1], i, scores[i]);
        }
    }
}