use crate::{eval::{pawnking::pawn_bonus, status::FileStatus}, movegen::attacks::{knight_attacks, pawn_attacks}, repr::{board::Board, colour::Colour, piece::{Piece, PieceType}}, search::{MAX_PLY, state::SearchState}};

pub mod pst;
pub mod pawnking;
pub mod status;

pub type Eval = i16;

pub const INF: Eval = 31000;
pub const MATE: Eval = INF - 1;
pub const MATE_CUTOFF: Eval = MATE - MAX_PLY as Eval * 2;

pub const EVAL_BONUS_DELTA_OUTER: Eval = 115;
pub const EVAL_BONUS_DELTA_INNER: Eval = 60;

const SIDE_TO_MOVE_BONUS: Eval = 10;
const BATTERY_BONUS: Eval = 25;
const OUTPOST_BONUS: Eval = 40;
const MISSING_PAWN_BONUS_KNIGHT: [Eval; 9] = [30, 15, 0, -5, -20, -25, -30, -35, -45];
const KNIGHT_CONTROL_BONUS: Eval = 5;
const BISHOP_PAIR_BONUS: Eval = 50;
const BISHOP_BLOCKER_BONUS: Eval = -15;
const SLIDER_CONTROL_BONUS: Eval = 3;

pub fn partial_relative_eval(board: &Board, search_state: &mut SearchState, alpha: Eval, beta: Eval) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let mut partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    if partial_eval * mult <= alpha - EVAL_BONUS_DELTA_OUTER || partial_eval * mult >= beta + EVAL_BONUS_DELTA_OUTER {
        return mult * partial_eval + SIDE_TO_MOVE_BONUS;
    }
    let entry = pawn_bonus(board, search_state);
    partial_eval += phase_eval(board.state.phase_unbounded, entry.mg_eval, entry.eg_eval);
    if partial_eval * mult <= alpha - EVAL_BONUS_DELTA_INNER || partial_eval * mult >= beta + EVAL_BONUS_DELTA_INNER {
        return mult * partial_eval + SIDE_TO_MOVE_BONUS;
    }
    partial_eval += bonus_eval_non_pawn(board, entry.files);
    mult * partial_eval + SIDE_TO_MOVE_BONUS
}

pub fn relative_eval(board: &Board, search_state: &mut SearchState) -> Eval {
    let mult = if board.colour == Colour::White { 1 } else { -1 };
    let partial_eval = phase_eval(board.state.phase_unbounded, board.state.mg_eval, board.state.eg_eval);
    mult * (partial_eval + bonus_eval(board, search_state)) + SIDE_TO_MOVE_BONUS
}

fn bonus_eval(board: &Board, search_state: &mut SearchState) -> Eval {
    let entry = pawn_bonus(board, search_state);
    let pawn = phase_eval(board.state.phase_unbounded, entry.mg_eval, entry.eg_eval);
    pawn + bonus_eval_non_pawn(board, entry.files)
}

fn bonus_eval_non_pawn(board: &Board, files: [u8; 2]) -> Eval {
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;
    let (mg, eg) = rook_queen_bonus(board, files);
    mg_bonus += mg; eg_bonus += eg;
    let (mg, eg) = knight_bonus(board);
    mg_bonus += mg; eg_bonus += eg;
    let (mg, eg) = bishop_bonus(board);
    mg_bonus += mg; eg_bonus += eg;
    let (mg, eg) = slider_mobility(board);
    mg_bonus += mg; eg_bonus += eg;
    phase_eval(board.state.phase_unbounded, mg_bonus, eg_bonus)
}

#[inline]
fn phase_eval(phase_unbounded: u8, mg_eval: Eval, eg_eval: Eval) -> Eval {
    let phase = phase_unbounded.min(24) as i32;
    ((mg_eval as i32 * phase + eg_eval as i32 * (24 - phase)) / 24) as Eval
}

impl Board {
    pub fn pst_eval(&self) -> Eval {
        let eval = phase_eval(self.state.phase_unbounded, self.state.mg_eval, self.state.eg_eval);
        if self.colour == Colour::White { eval } else { -eval }
    }
}

#[inline]
fn rook_queen_bonus(board: &Board, files: [u8; 2]) -> (Eval, Eval) {
    let white = board[Piece::WhiteRook] | board[Piece::WhiteQueen];
    let black = board[Piece::BlackRook] | board[Piece::BlackQueen];
    let mut mg_eval = 0;
    let mut eg_eval = 0;

    let mut white_per_file = [0u8; 8];
    for square in white.squares() {
        let file = square.file() as usize;
        if white_per_file[file] > 0 { mg_eval += BATTERY_BONUS; }
        white_per_file[file] += 1;
        let file_status = FileStatus::from_files(files[0], files[1], file as u8);
        mg_eval += file_status.rook_bonus(Colour::White);
    }

    let mut black_per_file = [0u8; 8];
    for square in black.squares() {
        let file = square.file() as usize;
        if black_per_file[file] > 0 { mg_eval -= BATTERY_BONUS; }
        black_per_file[file] += 1;
        let file_status = FileStatus::from_files(files[0], files[1], file as u8);
        mg_eval -= file_status.rook_bonus(Colour::Black);
    }

    (mg_eval, eg_eval)
}

#[inline]
fn knight_bonus(board: &Board) -> (Eval, Eval) {
    let mut bonus = 0;
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;
    let white = board[Piece::WhiteKnight];
    let black = board[Piece::BlackKnight];
    let white_pawns = board[Piece::WhitePawn];
    let black_pawns = board[Piece::BlackPawn];

    let mut white_pawn_cover = pawn_attacks(white_pawns, Colour::White);
    let mut black_pawn_cover = pawn_attacks(black_pawns, Colour::Black);
    let white_control = knight_attacks(white) & !black;
    let black_control = knight_attacks(black) & !white;
    bonus += (white_control.count_ones() as Eval - black_control.count_ones() as Eval) * KNIGHT_CONTROL_BONUS;
    
    black_pawn_cover |= black_pawn_cover >> 8;
    white_pawn_cover |= white_pawn_cover << 8;

    let white_outpost = white & Board::TOP & !black_pawn_cover;
    let black_outpost = black & Board::BOTTOM & !white_pawn_cover;
    mg_bonus += (white_outpost.count_ones() as Eval - black_outpost.count_ones() as Eval) * OUTPOST_BONUS;

    let white_missing = 8 - white_pawns.count_ones();
    let black_missing = 8 - black_pawns.count_ones();
    bonus += MISSING_PAWN_BONUS_KNIGHT[black_missing as usize] * white.count_ones() as Eval;
    bonus -= MISSING_PAWN_BONUS_KNIGHT[white_missing as usize] * black.count_ones() as Eval;

    (bonus + mg_bonus, bonus + eg_bonus)
}

#[inline]
fn bishop_bonus(board: &Board) -> (Eval, Eval) {
    let mut bonus = 0;
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;

    let white = board[Piece::WhiteBishop];
    let black = board[Piece::BlackBishop];

    if white.count_ones() >= 2 { bonus += BISHOP_PAIR_BONUS; }
    if black.count_ones() >= 2 { bonus -= BISHOP_PAIR_BONUS; }

    let white_pawns = board[Piece::WhitePawn];
    let black_pawns = board[Piece::BlackPawn];
    let white_attacks = pawn_attacks(white, Colour::White);
    let black_attacks = pawn_attacks(black, Colour::Black);
    let white_blockers = white_pawns & white_attacks;
    let black_blockers = black_pawns & black_attacks;

    bonus += (white_blockers.count_ones() as Eval - black_blockers.count_ones() as Eval) * BISHOP_BLOCKER_BONUS;

    (bonus + mg_bonus, bonus + eg_bonus)
}

#[inline]
fn slider_mobility(board: &Board) -> (Eval, Eval) {
    let mut bonus = 0;
    let mut mg_bonus = 0;
    let mut eg_bonus = 0;
    let white = board.state.attacks[Colour::White as usize][PieceType::Slider as usize];
    let black = board.state.attacks[Colour::Black as usize][PieceType::Slider as usize];
    let white_pawns = board[Piece::WhitePawn];
    let black_pawns = board[Piece::BlackPawn];
    let white_control = white & !pawn_attacks(black_pawns, Colour::Black);
    let black_control = black & !pawn_attacks(white_pawns, Colour::White);
    bonus += (white_control.count_ones() as Eval - black_control.count_ones() as Eval) * SLIDER_CONTROL_BONUS;

    (bonus + mg_bonus, bonus + eg_bonus)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cmp::Reverse;
    use crate::{eval::bonus_eval, movegen::r#move::Move, repr::board::Board, search::state::SearchState};

    // Depth for the bonus distribution test. Depth 4 gives ~10M positions
    // across the 8 perft positions and finishes in a few seconds in release mode.
    const BONUS_DIST_DEPTH: u32 = 4;

    fn collect_bonus(board: &mut Board, state: &mut SearchState, depth: u32, bonus: &mut Vec<i32>, pawn_vec: &mut Vec<i32>, phase: &mut Vec<i32>) {
        if depth == 0 {
            let pawn_entry = crate::eval::pawnking::pawn_bonus(board, state);
            let pawn_part = super::phase_eval(board.state.phase_unbounded, pawn_entry.mg_eval, pawn_entry.eg_eval);
            bonus.push(bonus_eval(board, state) as i32);
            pawn_vec.push(pawn_part as i32);
            phase.push(board.state.phase_unbounded as i32);
            return;
        }
        let mut movelist = board.generate_movelist(false);
        let mut i = 0;
        while i < movelist.length {
            let mv = movelist[i];
            let unmake = board.makemove(mv);
            collect_bonus(board, state, depth - 1, bonus, pawn_vec, phase);
            board.unmakemove(mv, unmake);
            movelist.maybe_add_proms(1, Some(mv), i);
            i += 1;
        }
    }

    fn print_dist_pair(full: &[i32], pawn: &[i32], rest: &[i32], label: &str, phase_avg: i32, phase_root: Option<i32>) {
        if full.is_empty() { eprintln!("{label}: no samples"); return; }
        let n = full.len();
        let avg = |v: &[i32]| v.iter().map(|&x| x as i64).sum::<i64>() / n as i64;
        let ph_rt_str = match phase_root {
            Some(root) => format!("{:>5}", root),
            None       => "   --".to_string(),
        };
        eprint!("{label:<14}  {n:>10}  {ph_rt_str}  {:>6}  {:>3}/{:<3}/{:<3}", phase_avg, avg(full), avg(pawn), avg(rest));
        for p in 0..=10 {
            let idx = (p * (n - 1)) / 10;
            eprint!("  {:>3}/{:<3}/{:<3}", full[idx], pawn[idx], rest[idx]);
        }
        eprintln!();
    }

    pub(crate) const TEST_POSITIONS: &[(&str, &str)] = &[
            ("Start",    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
            ("Kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
            ("Pos3",     "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"),
            ("Pos4",     "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"),
            ("Pos6",     "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"),
            ("G1p13ph24", "r1b1kb1r/pp3ppp/2n1pn2/2pq4/3P4/P1P1BN2/1P3PPP/RN1QKB1R b KQkq - 0 7"),  // phase~24
            ("G1p26ph24", "r3k2r/ppqbbpp1/2n1p2p/1B6/3P2nB/P4N2/1P1N1PPP/2RQK2R w Kkq - 2 14"),  // phase~24
            ("G3p16ph24", "rn2k1nr/pp1b1ppp/8/1Bb1p3/3q4/2N5/PP1B1PPP/R2QK1NR w KQkq - 4 9"),  // phase~24
            ("G4p14ph24", "rn2kbnr/pp3ppp/4b3/3qp3/8/2P2N2/P2N1PPP/R1BQKB1R w KQkq - 1 8"),  // phase~24
            ("G5p14ph24", "r1bqk1nr/1p2bppp/p1np4/1N2p3/2P1P3/2N5/PP3PPP/R1BQKB1R w KQkq - 0 8"),  // phase~24
            ("G6p18ph24", "r2qkb1r/1p3ppp/p1npbn2/8/4PB2/1NN5/PPP3PP/R2QKB1R w KQkq - 1 10"),  // phase~24
            ("G8p15ph24", "r1bqk2r/pp2bppp/2np1n2/4p3/4P3/1NN1BP2/PPP3PP/R2QKB1R b KQkq - 3 8"),  // phase~24
            ("G8p32ph24", "2rq1rk1/1p1n1pp1/2npbb1p/3Np3/p1P1P3/4BPP1/PP1QBK1P/2NR3R w - - 0 17"),  // phase~24
            ("G9p17ph24", "rn1qkb1r/3b1ppp/p2ppn2/1p4B1/3NP3/P1N3Q1/1PP2PPP/R3KB1R b KQkq - 0 9"),  // phase~24
            ("G10p15ph24", "rn1qk2r/ppp1bppp/8/3p1b2/3Pn3/3B1N2/PPP2PPP/RNBQR1K1 b kq - 3 8"),  // phase~24
            ("G11p13ph24", "r1bqk2r/ppp1bppp/2np1n2/1B6/2Q1P3/2N2N2/PPP2PPP/R1B1K2R b KQkq - 6 7"),  // phase~24
            ("G12p14ph24", "r1bq1rk1/pp2bppp/2np1n2/2p1p1B1/P1B1P3/2NP1N2/1PP2PPP/R2QK2R w KQ - 3 8"),  // phase~24
            ("G15p12ph24", "r1bqkb1r/2pp1ppp/p1n5/1p2p3/B2Pn3/5N2/PPP2PPP/RNBQ1RK1 w kq - 0 7"),  // phase~24
            ("G17p10ph24", "r1bqkb1r/1ppp1ppp/p1n5/4p3/B3n3/5N2/PPPP1PPP/RNBQ1RK1 w kq - 0 6"),  // phase~24
            ("G19p14ph24", "r1bqkb1r/2p2ppp/p1n5/1p1pp3/3Pn3/1B3N2/PPP2PPP/RNBQ1RK1 w kq - 0 8"),  // phase~24
            ("G20p18ph24", "r2qk2r/2p1bppp/p1n1b3/1p1pP3/4n3/1BP2N2/PP3PPP/RNBQ1RK1 w kq - 1 10"),  // phase~24
            ("G21p20ph24", "rn1q1rk1/pp3ppp/2pb4/3p1b2/2PPn3/3B1N2/PP3PPP/RNBQR1K1 w - - 2 11"),  // phase~24
            ("G22p15ph24", "r1bqkb1r/pp3ppp/2np1n2/1N2p1B1/4P3/2N5/PPP2PPP/R2QKB1R b KQkq - 1 8"),  // phase~24
            ("G24p11ph24", "r1bqkb1r/ppp2ppp/5n2/nB1Pp1N1/8/8/PPPP1PPP/RNBQK2R b KQkq - 2 6"),  // phase~24
            ("G24p28ph24", "r1bq1rk1/p4ppp/1bp5/n3p3/4N3/3P2Pn/PPP2P1P/RNBQKB1R w KQ - 5 15"),  // phase~24
            ("G26p12ph24", "rnbqkb1r/1p3ppp/p2p1n2/4p3/3NP3/2N1B3/PPP2PPP/R2QKB1R w KQkq - 0 7"),  // phase~24
            ("G27p22ph24", "r1bqr1k1/pp2bppp/3p1n2/2pP4/1n6/2NB1N1P/PPP2PP1/R1BQR1K1 w - - 1 12"),  // phase~24
            ("G29p12ph24", "r1bqkb1r/pp3ppp/2nppn2/8/3NPB2/2N5/PPP2PPP/R2QKB1R w KQkq - 0 7"),  // phase~24
            ("G5p34ph22", "r3r1k1/1p1qb2p/p2p1npB/4p3/1nP2P2/N1N5/PP2Q1PP/R4RK1 w - - 1 18"),  // phase~22
            ("G13p24ph22", "r1b1r1k1/p2nppbp/2pp2p1/2q3B1/4P3/N1P2N1P/PP3PP1/R2QR1K1 w - - 5 13"),  // phase~22
            ("G14p10ph22", "r1bqkb1r/p1pp1ppp/2p2n2/8/4P3/8/PPP2PPP/RNBQKB1R w KQkq - 0 6"),  // phase~22
            ("G14p31ph22", "r3r1k1/p1qn1p1p/2pbb1p1/3p2B1/7Q/3B3P/PPP1NPP1/R3R1K1 b - - 3 16"),  // phase~22
            ("G15p31ph22", "r2q1rk1/2p2ppp/2n3b1/1p1pP3/p2Pn3/4B3/PPB2PPP/R1NQ1RK1 b - - 1 16"),  // phase~22
            ("G16p12ph22", "r2qkb1r/pppb1ppp/8/3p4/3Pn3/3B4/PPP2PPP/RNBQK2R w KQkq - 0 7"),  // phase~22
            ("G23p22ph22", "r1bq1rk1/5ppp/p1np1b2/1p1Np3/4P3/N1P5/PP3PPP/R2QKB1R w KQ - 1 12"),  // phase~22
            ("G27p36ph22", "r2qrbk1/p4pp1/p2B1n1p/2pP4/8/P1N2Q1P/1PbN1PP1/R3R1K1 w - - 0 19"),  // phase~22
            ("G21p47ph21", "r1r3k1/pp1q1bp1/5p1p/3p1n2/1Q1PBN2/P6P/1P3PP1/2R1RNK1 b - - 0 24"),  // phase~21
            ("G28p25ph21", "r3k2r/3q1ppp/p1pbN1n1/2pp4/8/2PP4/PP2QPPP/RNB1R1K1 b kq - 0 13"),  // phase~21
            ("G2p14ph20", "rnb1kbnr/pp3ppp/8/8/3q4/5N2/PP3PPP/RNB1KB1R w KQkq - 0 8"),  // phase~20
            ("G8p64ph20", "2r3k1/5pp1/3p1n2/2r4p/1q2PP2/1P2QBPb/4N2P/1R2R1K1 w - - 6 33"),  // phase~20
            ("G22p39ph20", "2rq1rk1/1b3p1p/p2p2p1/3Bp1b1/RP2P3/N1P5/5PPP/3Q1RK1 b - - 0 20"),  // phase~20
            ("G23p38ph20", "r4rk1/4np1p/2bp1qp1/4p3/p1B1P3/R1P1N3/1P3PPP/3Q1RK1 w - - 2 20"),  // phase~20
            ("G25p33ph20", "r4r1k/2pqb1pp/p4p2/1p2Pb2/3PpP2/4B2P/PPB3P1/R2Q1RK1 b - - 0 17"),  // phase~20
            ("G26p34ph20", "r1b1k2r/4qpp1/5n1p/p1p1p3/Pp2P1P1/4B2P/1PPQ1PB1/R3K2R w KQkq - 0 18"),  // phase~20
            ("G28p46ph20", "4rnkr/3q2p1/2pb4/p2pp3/3P4/4BNP1/PP2Q2P/R3R1K1 w - - 0 24"),  // phase~20
            ("G30p23ph20", "r1b1k2r/p2p2pp/1qpQpn2/8/8/3B4/PPP2PPP/R1B1K2R b KQkq - 2 12"),  // phase~20
            ("G19p42ph19", "1r3rk1/2p3pp/p3bq2/3p4/P2N4/2B2n1P/2B2QP1/R5K1 w - - 0 22"),  // phase~19
            ("G12p39ph18", "1rbq2rk/5p1p/p2p1p1Q/1p1Pp3/2P2P1N/2P5/1P4PP/R4RK1 b - - 0 20"),  // phase~18
            ("G13p46ph18", "r1r2k2/4pp1p/b2p1qp1/N7/1p6/2P4P/P2Q1PP1/R3R1K1 w - - 0 24"),  // phase~18
            ("G13p52ph18", "r1r3k1/4pp1p/3p2pQ/Nb6/1P6/P1q4P/5PP1/R3R1K1 w - - 1 27"),  // phase~18
            ("G22p62ph18", "1r4k1/R4p1p/1P4p1/1r1q2b1/2N1p3/6P1/Q4P1P/5RK1 w - - 4 32"),  // phase~18
            ("G1p68ph16", "1q6/p2brp1k/1p4np/5p2/1PNP4/P2B1Q1P/3K1P2/6R1 w - - 2 35"),  // phase~16
            ("G10p36ph16", "r3q1k1/p4ppp/pnpb4/8/3P4/2PQ1N2/5PPP/R1B3K1 w - - 0 19"),  // phase~16
            ("G12p57ph16", "2b3rk/R3qp1p/3p3Q/3P1p2/2pR1P1N/2P4P/4p1PK/8 b - - 1 29"),  // phase~16
            ("G24p63ph16", "5rk1/p5pp/3q1r2/2p1p3/8/1PQP2P1/P1PR2KP/4R3 b - - 0 32"),  // phase~16
            ("G9p46ph15", "4k2r/5p1p/4pp1b/1p1p1P2/4P3/5KQ1/2q3PP/3R3R w k - 0 24"),  // phase~15
            ("G9p53ph15", "4k2r/5p1p/4pp1b/1p1P4/4q3/6Q1/5KPP/3R3R b k - 5 27"),  // phase~15
            ("G7p18ph14", "r1b1k1nr/ppp2ppp/8/4n3/2B1p3/2N5/PPP2PPP/R1B1K2R w KQkq - 0 10"),  // phase~14
            ("G11p42ph14", "1r2qbk1/Q4pp1/3p4/2p5/5P1p/2N3P1/PPP2P2/2KR4 w - - 0 22"),  // phase~14
            ("G14p60ph14", "4r1k1/5b1p/5p2/p2p4/7p/1P1R2N1/P2Q1PPK/q7 w - - 0 31"),  // phase~14
            ("G20p45ph14", "5rk1/2p2p1p/5p2/1p2nq2/1P1P1P2/3p4/1P4PP/1B1Q1RK1 b - - 0 23"),  // phase~14
            ("G25p63ph14", "3q4/2p3pk/p1B3bp/1p1PPr2/8/P3Q3/1P4P1/4R1K1 b - - 0 32"),  // phase~14
            ("G2p42ph12", "r5k1/pN4pp/2n1pn2/8/P7/B1r5/5PPP/R4RK1 w - - 0 22"),  // phase~12
            ("G4p32ph12", "2kr3r/1p4pp/p3Rn2/2b5/8/2P5/P2N1PPP/R1B3K1 w - - 1 17"),  // phase~12
            ("G6p37ph12", "2r1r1k1/1p2bppp/p2p4/3RnP2/5B2/P5P1/1PP4P/2K2B1R b - - 0 19"),  // phase~12
            ("G16p36ph12", "2kr3r/2pb2p1/1p1b1p2/p7/3RB2p/2B4P/PPP2PP1/3R2K1 w - - 0 19"),  // phase~12
            ("G17p39ph12", "1r5r/R1pkbppp/8/1p2P1B1/4N3/8/1P2nPPP/5R1K b - - 5 20"),  // phase~12
            ("G23p56ph12", "5r2/r3np2/2b2kp1/4p2p/1R2P3/1BP1N3/5PP1/5RK1 w - - 0 29"),  // phase~12
            ("G30p48ph12", "r2r4/1R4p1/3pbk1p/p2np3/P6P/3BB3/2P2PP1/3R2K1 w - - 0 25"),  // phase~12
            ("G3p43ph10", "3rr1k1/1p4pp/p4p2/2N1n3/8/6P1/PP3PP1/1K1RR3 b - - 3 22"),  // phase~10
            ("G18p49ph10", "4k2r/2p1b2N/p4pB1/1pn5/8/1b2B3/1P3PPP/3R2K1 b - - 7 25"),  // phase~10
            ("G18p57ph10", "4k2r/2p1bb1N/p3np2/1p6/6P1/4B3/1P3P1P/1B1R2K1 b - - 4 29"),  // phase~10
            ("G28p85ph10", "4R3/6pk/1bp5/8/P6P/3r2B1/P3R1KP/3r4 b - - 0 43"),  // phase~10
            ("G11p80ph8", "5k2/6p1/8/8/3Q3P/q3K3/2P2P2/8 w - - 12 41"),  // phase~8
            ("G26p60ph8", "4r3/3n1kp1/2b4p/p7/1pP1P1P1/4B2P/1KP3B1/3R4 w - - 2 31"),  // phase~8
            ("G4p69ph6", "8/8/1k5p/1p1r2pn/4N3/4KPPP/P1R5/8 b - - 0 35"),  // phase~6
            ("G7p39ph6", "2kr4/ppp2Bpp/2b5/8/5P2/8/PPP3PP/2K1R3 b - - 0 20"),  // phase~6
            ("G15p71ph6", "3B3k/5bpp/8/8/1P6/p3p1n1/P1Bp2PP/R5K1 b - - 9 36"),  // phase~6
            ("G16p57ph6", "4R3/2p4r/1p1b1p2/p4kp1/P6p/2B4P/1PP1KPP1/8 b - - 1 29"),  // phase~6
            ("G19p74ph6", "7k/1B1b2pp/8/r2p4/2pN4/B1K5/6P1/8 w - - 0 38"),  // phase~6
            ("G29p42ph6", "2r3k1/p4ppp/b3p3/2p5/4BP2/2P5/PP4PP/3K3R w - - 0 22"),  // phase~6
            ("G29p51ph6", "1r6/p3kpp1/b3p2p/2p5/P3BP1P/1PP5/2K3P1/1R6 b - - 0 26"),  // phase~6
            ("G30p76ph6", "8/2R3p1/5k1p/1B2p3/P6P/1b6/r4PP1/6K1 w - - 9 39"),  // phase~6
            ("G10p77ph5", "6k1/6p1/2R2p1p/2B5/8/3K2P1/1r5P/8 b - - 0 39"),  // phase~5
            ("G2p82ph4", "8/3n2p1/1p2k2p/n3p2P/P7/B5P1/3N1PK1/8 w - - 5 42"),  // phase~4
            ("G3p55ph4", "2R5/1p1r1k1p/p4pp1/8/8/6P1/PPK2PP1/8 b - - 1 28"),  // phase~4
            ("G5p69ph4", "6k1/8/p5p1/7p/r7/7P/PR4PK/8 b - - 2 35"),  // phase~4
            ("G6p83ph4", "8/5k2/B2n1pp1/2b5/pp3BPP/PP6/8/1K6 b - - 0 42"),  // phase~4
            ("G7p82ph4", "8/8/5k2/8/2P4K/R7/6r1/8 w - - 0 42"),  // phase~4
            ("G20p90ph4", "8/2p1kp2/7p/1P2Kp2/2P2R1P/8/7r/8 w - - 10 46"),  // phase~4
            ("G21p81ph4", "5k2/5R2/1p2P2p/p3p3/P3p2P/4P1K1/4r1P1/8 b - - 2 41"),  // phase~4
            ("G17p54ph2", "8/2p2Npp/8/1p1kP3/3n4/6P1/1P3PKP/8 w - - 2 28"),  // phase~2
            ("G27p69ph2", "4k3/p2n4/p2P4/N1p2pp1/5P2/P5K1/1P4P1/8 b - - 0 35"),  // phase~2
    ];

    #[test]
    fn test_bonus_distribution() {
        let positions = TEST_POSITIONS;
        let mut state = SearchState::new(16, 2, 18);
        let mut global_bonus: Vec<i32> = Vec::new();
        let mut global_pawn:  Vec<i32> = Vec::new();
        let mut global_rest:  Vec<i32> = Vec::new();
        let mut global_phase: Vec<i32> = Vec::new();
        // (name, full_abs_sorted, pawn_abs_sorted, rest_abs_sorted, phase_avg, phase_root)
        let mut per_pos: Vec<(&str, Vec<i32>, Vec<i32>, Vec<i32>, i32, i32)> = Vec::new();

        for (name, fen) in positions {
            let mut board = Board::from_fen(fen);
            let phase_root = board.state.phase_unbounded as i32;
            let mut bonus_samples: Vec<i32> = Vec::new();
            let mut pawn_samples:  Vec<i32> = Vec::new();
            let mut phase_samples: Vec<i32> = Vec::new();
            collect_bonus(&mut board, &mut state, BONUS_DIST_DEPTH, &mut bonus_samples, &mut pawn_samples, &mut phase_samples);
            let mut rest_abs: Vec<i32> = bonus_samples.iter().zip(pawn_samples.iter()).map(|(&b, &p)| (b - p).abs()).collect();
            global_bonus.extend(bonus_samples.iter().map(|&x| x.abs()));
            global_pawn.extend(pawn_samples.iter().map(|&x| x.abs()));
            global_rest.extend_from_slice(&rest_abs);
            global_phase.extend_from_slice(&phase_samples);
            let phase_avg = if phase_samples.is_empty() { 0 } else {
                (phase_samples.iter().map(|&x| x as i64).sum::<i64>() / phase_samples.len() as i64) as i32
            };
            let mut full_abs: Vec<i32> = bonus_samples.iter().map(|&x| x.abs()).collect();
            let mut pawn_abs: Vec<i32> = pawn_samples.iter().map(|&x| x.abs()).collect();
            full_abs.sort_unstable();
            pawn_abs.sort_unstable();
            rest_abs.sort_unstable();
            per_pos.push((name, full_abs, pawn_abs, rest_abs, phase_avg, phase_root));
        }

        global_bonus.sort_unstable();
        global_pawn.sort_unstable();
        global_rest.sort_unstable();

        let global_phase_avg = if global_phase.is_empty() { 0 } else {
            (global_phase.iter().map(|&x| x as i64).sum::<i64>() / global_phase.len() as i64) as i32
        };

        let hdr = format!("{:<14}  {:>10}  {:>5}  {:>6}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}  {:>11}",
            "position", "n", "ph_rt", "ph_avg", "avg", "p0", "p10", "p20", "p30", "p40", "p50", "p60", "p70", "p80", "p90", "p100");

        per_pos.sort_by_key(|(_, full, _, _, _, _)| {
            let avg = full.iter().map(|&x| x as i64).sum::<i64>() / full.len().max(1) as i64;
            Reverse(avg)
        });

        eprintln!("\n|bonus_eval| full/pawn/rest distribution at depth {BONUS_DIST_DEPTH}:");
        eprintln!("{hdr}");
        for (name, full, pawn, rest, phase_avg, phase_root) in &per_pos {
            print_dist_pair(full, pawn, rest, name, *phase_avg, Some(*phase_root));
        }
        eprintln!();
        print_dist_pair(&global_bonus, &global_pawn, &global_rest, "GLOBAL", global_phase_avg, None);
    }

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