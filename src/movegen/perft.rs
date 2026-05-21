/*
Counts the number of moves at any given position. Used for testing purposes.
*/

use crate::{bitboard::Board, movegen::{generator::generate_movelist, makemove::{make_move, unmake_move}}};

pub fn perft(board: &Board, depth: u32) -> u64 {
    let mut board= board.clone();
    if depth == 0 { return 1; }
    let mut out = 0;
    for mv in generate_movelist(&board, false).iter() {
        let unmake = make_move(&mut board, mv);
        if !board.in_check(board.side.other()) {
            out += perft(&board, depth - 1);
        }
        unmake_move(&mut board, mv, &unmake);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(fen: &str) -> Board { Board::from_fen(fen) }

    // Perft divide: prints per-move counts, useful for pinpointing discrepancies
    #[allow(dead_code)]
    fn perft_divide(board: &Board, depth: u32) {
        use crate::movegen::{generator::generate_movelist, makemove::{make_move}};
        let mut total = 0u64;
        for mv in generate_movelist(board, false).iter() {
            let mut copy = board.clone();
            make_move(&mut copy, mv);
            if !copy.in_check(copy.side.other()) {
                let count = perft(&copy, depth - 1);
                println!("{}: {}", mv.to_uci(), count);
                total += count;
            }
        }
        println!("Total: {total}");
    }

    // ── Comprehensive: all positions, all depths, with progress output ────────
    // Run with: cargo test perft_all -- --nocapture
    #[test]
    fn perft_all() {
        struct Case { name: &'static str, fen: &'static str, depths: &'static [(u32, u64)] }
        let cases = [
            Case { name: "Start",      fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",                            depths: &[(1,20),(2,400),(3,8902),(4,197281)] },
            Case { name: "Kiwipete",   fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",               depths: &[(1,48),(2,2039),(3,97862),(4,4085603)] },
            Case { name: "Pos 3",      fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",                                           depths: &[(1,14),(2,191),(3,2812),(4,43238)] },
            Case { name: "Pos 4",      fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",                    depths: &[(1,6),(2,264),(3,9467),(4,422333)] },
            Case { name: "Pos 5",      fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",                          depths: &[(1,44),(2,1486),(3,62379),(4,2103487)] },
            Case { name: "Pos 6",      fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",           depths: &[(1,46),(2,2079),(3,89890),(4,3894594)] },
            Case { name: "Promos",     fen: "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1",                                             depths: &[(1,24),(2,496),(3,9483),(4,182838)] },
            Case { name: "Castling",   fen: "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",                                               depths: &[(1,26),(2,568),(3,13744),(4,314346)] },
            // Additional positions from external test suite
            Case { name: "Extra 1",    fen: "r6r/1b2k1bq/8/8/7B/8/8/R3K2R b KQ - 3 2",                                            depths: &[(1,8)] },
            Case { name: "Extra 2",    fen: "8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 3",                                                  depths: &[(1,8)] },
            Case { name: "Extra 3",    fen: "r1bqkbnr/pppppppp/n7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 2 2",                          depths: &[(1,19)] },
            Case { name: "Extra 4",    fen: "r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2",                 depths: &[(1,5)] },
            Case { name: "Extra 5",    fen: "2kr3r/p1ppqpb1/bn2Qnp1/3PN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQ - 3 2",                    depths: &[(1,44)] },
            Case { name: "Extra 6",    fen: "rnb2k1r/pp1Pbppp/2p5/q7/2B5/8/PPPQNnPP/RNB1K2R w KQ - 3 9",                          depths: &[(1,39)] },
            Case { name: "Extra 7",    fen: "2r5/3pk3/8/2P5/8/2K5/8/8 w - - 5 4",                                                  depths: &[(1,9)] },
            Case { name: "Extra 15",   fen: "r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1",                                          depths: &[(4,1274206)] },
            Case { name: "Extra 16",   fen: "r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1",                                           depths: &[(4,1720476)] },
            Case { name: "Extra 23",   fen: "8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1",                                                   depths: &[(4,23527)] },
        ];
        let mut passed = 0;
        let total: usize = cases.iter().map(|c| c.depths.len()).sum();
        for case in &cases {
            let board = pos(case.fen);
            for &(depth, expected) in case.depths {
                let got = perft(&board, depth);
                assert_eq!(got, expected, "{} depth {}: expected {} got {}", case.name, depth, expected, got);
                passed += 1;
                println!("[{}/{}] {} depth {} = {} ✓", passed, total, case.name, depth, got);
            }
        }
    }

    // ── Position 1: starting position ────────────────────────────────────────
    // https://www.chessprogramming.org/Perft_Results#Initial_Position
    #[test]
    fn start_d1() { assert_eq!(perft(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), 1), 20); }
    #[test]
    fn start_d2() { assert_eq!(perft(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), 2), 400); }
    #[test]
    fn start_d3() { assert_eq!(perft(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), 3), 8902); }
    #[test]
    fn start_d4() { assert_eq!(perft(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), 4), 197281); }
    #[test] #[ignore]
    fn start_d5() { assert_eq!(perft(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"), 5), 4865609); }

    // ── Position 2: Kiwipete — stresses castling, en passant, promotions ─────
    // https://www.chessprogramming.org/Perft_Results#Position_2
    #[test]
    fn kiwipete_d1() { assert_eq!(perft(&pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"), 1), 48); }
    #[test]
    fn kiwipete_d2() { assert_eq!(perft(&pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"), 2), 2039); }
    #[test]
    fn kiwipete_d3() { assert_eq!(perft(&pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"), 3), 97862); }
    #[test]
    fn kiwipete_d4() { assert_eq!(perft(&pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"), 4), 4085603); }
    #[test] #[ignore]
    fn kiwipete_d5() { assert_eq!(perft(&pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"), 5), 193690690); }

    // ── Position 3 ───────────────────────────────────────────────────────────
    // https://www.chessprogramming.org/Perft_Results#Position_3
    #[test]
    fn pos3_d1() { assert_eq!(perft(&pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"), 1), 14); }
    #[test]
    fn pos3_d2() { assert_eq!(perft(&pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"), 2), 191); }
    #[test]
    fn pos3_d3() { assert_eq!(perft(&pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"), 3), 2812); }
    #[test]
    fn pos3_d4() { assert_eq!(perft(&pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"), 4), 43238); }
    #[test] #[ignore]
    fn pos3_d5() { assert_eq!(perft(&pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"), 5), 674624); }

    // ── Position 4 ───────────────────────────────────────────────────────────
    // https://www.chessprogramming.org/Perft_Results#Position_4
    #[test]
    fn pos4_d1() { assert_eq!(perft(&pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 1), 6); }
    #[test]
    fn pos4_d2() { assert_eq!(perft(&pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 2), 264); }
    #[test]
    fn pos4_d3() { assert_eq!(perft(&pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 3), 9467); }
    #[test]
    fn pos4_d4() { assert_eq!(perft(&pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 4), 422333); }
    #[test] #[ignore]
    fn pos4_d5() { assert_eq!(perft(&pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 5), 15833292); }

    // ── Position 5 ───────────────────────────────────────────────────────────
    // https://www.chessprogramming.org/Perft_Results#Position_5
    #[test]
    fn pos5_d1() { assert_eq!(perft(&pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 1), 44); }
    #[test]
    fn pos5_d2() { assert_eq!(perft(&pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 2), 1486); }
    #[test]
    fn pos5_d3() { assert_eq!(perft(&pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 3), 62379); }
    #[test]
    fn pos5_d4() { assert_eq!(perft(&pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 4), 2103487); }
    #[test] #[ignore]
    fn pos5_d5() { assert_eq!(perft(&pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 5), 89941194); }

    // ── Position 6 ───────────────────────────────────────────────────────────
    // https://www.chessprogramming.org/Perft_Results#Position_6
    #[test]
    fn pos6_d1() { assert_eq!(perft(&pos("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 1), 46); }
    #[test]
    fn pos6_d2() { assert_eq!(perft(&pos("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 2), 2079); }
    #[test]
    fn pos6_d3() { assert_eq!(perft(&pos("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 3), 89890); }
    #[test]
    fn pos6_d4() { assert_eq!(perft(&pos("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 4), 3894594); }
    #[test] #[ignore]
    fn pos6_d5() { assert_eq!(perft(&pos("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 5), 164075551); }

    // ── Position 7: promotion stress test ────────────────────────────────────
    // Black has three pawns on rank 2 and white has three on rank 7 — almost
    // every move is a promotion or promotion capture.
    #[test]
    fn promos_d1() { assert_eq!(perft(&pos("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"), 1), 24); }
    #[test]
    fn promos_d2() { assert_eq!(perft(&pos("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"), 2), 496); }
    #[test]
    fn promos_d3() { assert_eq!(perft(&pos("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"), 3), 9483); }
    #[test]
    fn promos_d4() { assert_eq!(perft(&pos("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"), 4), 182838); }
    #[test] #[ignore]
    fn promos_d5() { assert_eq!(perft(&pos("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"), 5), 3605103); }

    // ── Position 8: rook endgame with full castling rights ───────────────────
    // Only rooks and kings — stresses castling legality and rook mobility.
    #[test]
    fn castling_d1() { assert_eq!(perft(&pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"), 1), 26); }
    #[test]
    fn castling_d2() { assert_eq!(perft(&pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"), 2), 568); }
    #[test]
    fn castling_d3() { assert_eq!(perft(&pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"), 3), 13744); }
    #[test]
    fn castling_d4() { assert_eq!(perft(&pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"), 4), 314346); }
    #[test] #[ignore]
    fn castling_d5() { assert_eq!(perft(&pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"), 5), 7594526); }

    // ── Extra positions from external test suite ──────────────────────────────

    // Stresses castling rights when in check / bishop covers castling path
    #[test]
    fn extra1_d1() { assert_eq!(perft(&pos("r6r/1b2k1bq/8/8/7B/8/8/R3K2R b KQ - 3 2"), 1), 8); }

    // En passant square present, stresses correct capture rules
    #[test]
    fn extra2_d1() { assert_eq!(perft(&pos("8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 3"), 1), 8); }

    // Knight on a6 restricts white development
    #[test]
    fn extra3_d1() { assert_eq!(perft(&pos("r1bqkbnr/pppppppp/n7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 2 2"), 1), 19); }

    // King in check from queen — very few legal replies
    #[test]
    fn extra4_d1() { assert_eq!(perft(&pos("r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2"), 1), 5); }

    // Kiwipete variant — black king already castled queen-side
    #[test]
    fn extra5_d1() { assert_eq!(perft(&pos("2kr3r/p1ppqpb1/bn2Qnp1/3PN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQ - 3 2"), 1), 44); }

    // Pos5 variant with queen + extra pressure
    #[test]
    fn extra6_d1() { assert_eq!(perft(&pos("rnb2k1r/pp1Pbppp/2p5/q7/2B5/8/PPPQNnPP/RNB1K2R w KQ - 3 9"), 1), 39); }

    // Pawn + rook endgame, minimal branching
    #[test]
    fn extra7_d1() { assert_eq!(perft(&pos("2r5/3pk3/8/2P5/8/2K5/8/8 w - - 5 4"), 1), 9); }

    // Castling under bishop attack
    #[test]
    fn extra15_d4() { assert_eq!(perft(&pos("r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1"), 4), 1274206); }

    // Queen opposition across open board
    #[test]
    fn extra16_d4() { assert_eq!(perft(&pos("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1"), 4), 1720476); }

    // King+queen+knight endgame
    #[test]
    fn extra23_d4() { assert_eq!(perft(&pos("8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1"), 4), 23527); }

    // ── Deep positions (D5–D7) — run with: cargo test perft_deep -- --ignored --nocapture
    #[test] #[ignore]
    fn perft_deep() {
        struct Case { name: &'static str, fen: &'static str, depth: u32, nodes: u64 }
        let cases = [
            Case { name: "En passant avoid check",  fen: "3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1",          depth: 6, nodes: 1134888 },
            Case { name: "Bishop pawn endgame",      fen: "8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1",          depth: 6, nodes: 1015133 },
            Case { name: "En passant + bishop",      fen: "8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1",         depth: 6, nodes: 1440467 },
            Case { name: "Rook K-side castle only",  fen: "5k2/8/8/8/8/8/8/4K2R w K - 0 1",              depth: 6, nodes: 661072 },
            Case { name: "Rook Q-side castle only",  fen: "3k4/8/8/8/8/8/8/R3K3 w Q - 0 1",              depth: 6, nodes: 803711 },
            Case { name: "Pawn promotion race",      fen: "2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1",           depth: 6, nodes: 3821001 },
            Case { name: "Knight+queen vs pawn",     fen: "8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1",         depth: 5, nodes: 1004658 },
            Case { name: "King+pawn endgame 1",      fen: "4k3/1P6/8/8/8/8/K7/8 w - - 0 1",              depth: 6, nodes: 217342 },
            Case { name: "King+pawn endgame 2",      fen: "8/P1k5/K7/8/8/8/8/8 w - - 0 1",               depth: 6, nodes: 92683 },
            Case { name: "King+pawn endgame 3",      fen: "K1k5/8/P7/8/8/8/8/8 w - - 0 1",               depth: 6, nodes: 2217 },
            Case { name: "King+pawn endgame 4",      fen: "8/k1P5/8/1K6/8/8/8/8 w - - 0 1",              depth: 7, nodes: 567584 },
        ];
        let mut passed = 0;
        let total = cases.len();
        for case in &cases {
            let board = pos(case.fen);
            let got = perft(&board, case.depth);
            assert_eq!(got, case.nodes, "{} depth {}: expected {} got {}", case.name, case.depth, case.nodes, got);
            passed += 1;
            println!("[{}/{}] {} depth {} = {} ✓", passed, total, case.name, case.depth, got);
        }
    }
}