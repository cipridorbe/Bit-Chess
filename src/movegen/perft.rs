use crate::repr::board::Board;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let mut out = 0;
    let mut movelist = board.generate_movelist(false);
    let mut i = 0;
    while i < movelist.length {
        let mv = movelist[i];
        let unmake = board.makemove(mv);
        if !board.other_in_check() {
            out += perft(board, depth - 1);
        }
        board.unmakemove(mv, 0, unmake, &mut movelist);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(fen: &str) -> Board { Board::from_fen(fen) }

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
            let mut board = pos(case.fen);
            for &(depth, expected) in case.depths {
                let got = perft(&mut board, depth);
                assert_eq!(got, expected, "{} depth {}: expected {} got {}", case.name, depth, expected, got);
                passed += 1;
                println!("[{}/{}] {} depth {} = {} ✓", passed, total, case.name, depth, got);
            }
        }
    }

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
            let mut board = pos(case.fen);
            let got = perft(&mut board, case.depth);
            assert_eq!(got, case.nodes, "{} depth {}: expected {} got {}", case.name, case.depth, case.nodes, got);
            passed += 1;
            println!("[{}/{}] {} depth {} = {} ✓", passed, total, case.name, case.depth, got);
        }
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    fn stockfish_nodes(fen: &str, depth: u32) -> u64 {
        let mut sf = Command::new(r"C:\Program Files\stockfish\stockfish-windows-x86-64-avx2.exe")
            .stdin(Stdio::piped()).stdout(Stdio::piped())
            .spawn().expect("stockfish not found");
        {
            let stdin = sf.stdin.as_mut().unwrap();
            writeln!(stdin, "position fen {fen}").unwrap();
            writeln!(stdin, "go perft {depth}").unwrap();
            writeln!(stdin, "quit").unwrap();
        }
        let out = String::from_utf8(sf.wait_with_output().unwrap().stdout).unwrap();
        out.lines()
            .find_map(|l| l.strip_prefix("Nodes searched: ").and_then(|n| n.trim().parse().ok()))
            .unwrap_or_else(|| panic!("no node count from stockfish: {fen} d{depth}"))
    }

    // Run with: cargo test perft_speed -- --ignored --nocapture
    #[test]
    #[ignore]
    fn perft_speed() {
        let cases = [
            ("Start",    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
            ("Kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
            ("Pos 3",    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"),
            ("Pos 4",    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"),
            ("Pos 5",    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"),
            ("Pos 6",    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"),
            ("Promos",   "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1"),
            ("Castling", "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"),
        ];
        const DEPTH: u32 = 5;

        let mut total_nodes = 0u64;
        let mut all_ok = true;
        let wall = Instant::now();
        for (name, fen) in &cases {
            let t0 = Instant::now();
            let mut board = Board::from_fen(fen);
            let nodes = perft(&mut board, DEPTH);
            let secs = t0.elapsed().as_secs_f64();

            let sf = stockfish_nodes(fen, DEPTH);
            let ok = nodes == sf;
            all_ok &= ok;
            let status = if ok { "✓" } else { "✗" };
            let mismatch = if ok { String::new() } else { format!("  (sf={sf})") };
            println!("{status} {name:<10} d{DEPTH}: {nodes:>12} nodes  {secs:>6.3}s  {:>6.1} Mnps{mismatch}",
                nodes as f64 / secs / 1e6);
            total_nodes += nodes;
        }
        let total_secs = wall.elapsed().as_secs_f64();
        println!("{}", "-".repeat(62));
        println!("  {:<12} {:>12} nodes  {:>6.3}s  {:>6.1} Mnps",
            "Total", total_nodes, total_secs, total_nodes as f64 / total_secs / 1e6);
        assert!(all_ok, "one or more positions did not match stockfish");
    }
}

#[cfg(test)]
mod debug {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use std::io::Write;
    use std::process::{Command, Stdio};
    use crate::movegen::r#move::Move;

    fn stockfish_divide(fen: &str, depth: u32) -> HashMap<String, u64> {
        let mut sf = Command::new(r"C:\Program Files\stockfish\stockfish-windows-x86-64-avx2.exe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("stockfish not found");
        {
            let stdin = sf.stdin.as_mut().unwrap();
            writeln!(stdin, "position fen {}", fen).unwrap();
            writeln!(stdin, "go perft {}", depth).unwrap();
            writeln!(stdin, "quit").unwrap();
        }
        let out = sf.wait_with_output().unwrap().stdout;
        let mut map = HashMap::new();
        for line in String::from_utf8(out).unwrap().lines() {
            if let Some((mv, n)) = line.split_once(": ") {
                let mv = mv.trim();
                if mv.len() == 4 || mv.len() == 5 {
                    if let Ok(n) = n.trim().parse::<u64>() {
                        map.insert(mv.to_string(), n);
                    }
                }
            }
        }
        map
    }

    fn our_divide(fen: &str, depth: u32) -> HashMap<String, u64> {
        let board = Board::from_fen(fen);
        let mut map: HashMap<String, u64> = HashMap::new();
        let mut movelist = board.generate_movelist(false);
        let mut i = 0;
        while i < movelist.length {
            let mv = movelist[i];
            let mut clone = board.clone();
            let unmake = clone.makemove(mv);
            if !clone.other_in_check() {
                let n = perft(&mut clone, depth - 1);
                *map.entry(mv.to_uci()).or_insert(0) += n;
            }
            clone.unmakemove(mv, 0, unmake, &mut movelist);
            i += 1;
        }
        map
    }

    fn find_bug(fen: &str, depth: u32, indent: usize) {
        let pad = "  ".repeat(indent);
        let ours = our_divide(fen, depth);
        let sf = stockfish_divide(fen, depth);

        let our_total: u64 = ours.values().sum();
        let sf_total: u64 = sf.values().sum();

        if our_total == sf_total {
            println!("{}OK  depth={} total={} | {}", pad, depth, our_total, fen);
            return;
        }

        println!("{}BAD depth={} ours={} sf={} | {}", pad, depth, our_total, sf_total, fen);

        let all_moves: BTreeMap<String, (u64, u64)> = {
            let mut m = BTreeMap::new();
            for (mv, &n) in &ours { m.entry(mv.clone()).or_insert((0, 0)).0 = n; }
            for (mv, &n) in &sf   { m.entry(mv.clone()).or_insert((0, 0)).1 = n; }
            m
        };

        let mut first_bad: Option<String> = None;
        for (mv, (o, s)) in &all_moves {
            if o != s {
                println!("{}  {} ours={} sf={}", pad, mv, o, s);
                if first_bad.is_none() { first_bad = Some(mv.clone()); }
            }
        }

        if depth > 1 {
            if let Some(mv) = first_bad {
                let mut b = Board::from_fen(fen);
                b.makemove(Move::from_uci(&b, &mv));
                find_bug(&b.to_fen(), depth - 1, indent + 1);
            }
        }
    }

    // Run with: cargo test debug_perft -- --ignored --nocapture
    #[test]
    #[ignore]
    fn debug_perft() {
        find_bug("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 5, 0);
    }
}