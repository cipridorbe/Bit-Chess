pub const MAX_PLY: u8 = 64;
pub const NUM_THREADS: u8 = 4;

pub mod negamax;
pub mod state;
pub mod see;
pub mod tt;

#[cfg(test)]
mod test {
    use std::time::{Duration, Instant};
    use crate::repr::game::Game;

    const BENCH_POSITIONS: &[&str] = &[
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "1rbq1rk1/p1b1nppp/1p2p3/8/1B1pN3/P2B4/1P3PPP/2RQ1R1K w - - 0 1",
        "3r2k1/p2r1p1p/1p2p1p1/q4n2/3P4/PQ5P/1P1RNPP1/3R2K1 b - - 0 1",
        "3r2k1/1p3ppp/2pq4/p1n5/P6P/1P6/1PB2QP1/1K2R3 w - - 0 1",
        "r1b1r1k1/1ppn1p1p/3pnqp1/8/p1P1P3/5P2/PbNQNBPP/1R2RB1K w - - 0 1",
        "2r4k/pB4bp/1p4p1/6q1/1P1n4/2N5/P4PPP/2R1Q1K1 b - - 0 1",
        "r5k1/3n1ppp/1p6/3p1p2/3P1B2/r3P2P/PR3PP1/2R3K1 b - - 0 1",
        "2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - 0 1",
        "5r1k/6pp/1n2Q3/4p3/8/7P/PP4PK/R1B1q3 b - - 0 1",
        "r3k2r/pbn2ppp/8/1P1pP3/P1qP4/5B2/3Q1PPP/R3K2R w KQkq - 0 1",
        "3r2k1/ppq2pp1/4p2p/3n3P/3N2P1/2P5/PP2QP2/K2R4 b - - 0 1",
        "q3rn1k/2QR4/pp2pp2/8/P1P5/1P4N1/6n1/6K1 w - - 0 1",
        "6k1/p3q2p/1nr3pB/8/3Q1P2/6P1/PP5P/3R2K1 b - - 0 1",
        "1r4k1/7p/5np1/3p3n/8/2NB4/7P/3N1RK1 w - - 0 1",
        "1r2r1k1/p4p1p/6pB/q7/8/3Q2P1/PbP2PKP/1R3R2 w - - 0 1",
        "r2q1r1k/pb3p1p/2n1p2Q/5p2/8/3B2N1/PP3PPP/R3R1K1 w - - 0 1",
        "8/4p3/p2p4/2pP4/2P1P3/1P4k1/1P1K4/8 w - - 0 1",
        "1r1q1rk1/p1p2pbp/2pp1np1/6B1/4P3/2NQ4/PPP2PPP/3R1RK1 w - - 0 1",
        "q4rk1/1n1Qbppp/2p5/1p2p3/1P2P3/2P4P/6P1/2B1NRK1 b - - 0 1",
        "r2q1r1k/1b1nN2p/pp3pp1/8/Q7/PP5P/1BP2RPN/7K w - - 0 1",
        "8/5p2/pk2p3/4P2p/2b1pP1P/P3P2B/8/7K w - - 0 1",
        "8/2k5/4p3/1nb2p2/2K5/8/6B1/8 w - - 0 1",
        "1B1b4/7K/1p6/1k6/8/8/8/8 w - - 0 1",
        "rn1q1rk1/1b2bppp/1pn1p3/p2pP3/3P4/P2BBN1P/1P1N1PP1/R2Q1RK1 b - - 0 1",
        "8/p1ppk1p1/2n2p2/8/4B3/2P1KPP1/1P5P/8 w - - 0 1",
        "8/3nk3/3pp3/1B6/8/3PPP2/4K3/8 w - - 0 1",
    ];

    #[test]
    fn bench() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        use crate::repr::board::Board;
        use crate::search::{MAX_PLY, negamax::search, state::SearchState};

        const TIME_PER_POSITION: Duration = Duration::from_secs(1);
        const MOVES_PER_POSITION: usize = 4;
        const VERBOSE: bool = true;

        let mut total_nodes: u64 = 0;
        let mut total_depth: u32 = 0;
        let mut total_searches: u32 = 0;
        let mut total_time = Duration::ZERO;

        for &fen in BENCH_POSITIONS {
            let mut board = Board::from_fen(fen);
            let mut state = SearchState::new_default();
            if VERBOSE { eprintln!("\n{}", fen); }
            for i in 0..MOVES_PER_POSITION {
                let stop = Arc::new(AtomicBool::new(false));
                let stop_timer = Arc::clone(&stop);
                std::thread::spawn(move || {
                    std::thread::sleep(TIME_PER_POSITION);
                    stop_timer.store(true, Ordering::Relaxed);
                });
                let start = Instant::now();
                let (mv, _eval, depth, nodes) = search(&mut board, &mut state, MAX_PLY, &stop, Some(start + TIME_PER_POSITION));
                let elapsed = start.elapsed();
                let knps = nodes as f64 / elapsed.as_secs_f64() / 1000.0;
                total_nodes += nodes;
                total_depth += depth as u32;
                total_searches += 1;
                total_time += elapsed;
                if VERBOSE {
                    eprintln!("  move {}: depth={:>2}  knps={:>8.0}  time={:.2?}", i + 1, depth, knps, elapsed);
                }
                match mv {
                    Some(mv) => { let _ = board.makemove(mv); }
                    None => break,
                }
            }
        }

        let total_knps = total_nodes as f64 / total_time.as_secs_f64() / 1000.0;
        let avg_depth = total_depth as f64 / total_searches as f64;
        eprintln!("\n  positions : {}", BENCH_POSITIONS.len());
        eprintln!("  searches  : {}", total_searches);
        eprintln!("  nodes     : {} ({:.0} knps)", total_nodes, total_knps);
        eprintln!("  time      : {:.2?}", total_time);
        eprintln!("  avg depth : {:.1}", avg_depth);
    }

    #[test]
    fn test_find_best_move_respects_time_limit() {
        let mut game = Game::new_infinite(None, None, None);
        let time_limit = Duration::from_millis(500);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("500ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(2000), "search took {:?}, way over the 500ms limit", elapsed);
    }

    #[test]
    fn test_find_best_move_depth15() {
        let mut game = Game::new_infinite(None, None, None);
        let time_limit = Duration::from_millis(6000);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("5s test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(10000), "search took {:?}, way over the 5s limit", elapsed);
    }

    #[test]
    fn test_find_best_move_depth15_with_stop() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        let mut game = Game::new_infinite(None, None, None);
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(2000));
            stop2.store(true, Ordering::Relaxed);
        });
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, None, Some(stop));
        let elapsed = start.elapsed();
        eprintln!("stop-flag test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(5000), "search took {:?}, should have stopped within ~2s", elapsed);
    }

    // Regression: helper threads must stop within ~2x time limit
    #[test]
    fn test_short_time_limit_stops_quickly() {
        let mut game = Game::new_infinite(None, None, None);
        let time_limit = Duration::from_millis(100);
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
        let elapsed = start.elapsed();
        eprintln!("100ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(500), "search took {:?}, helper threads did not stop in time", elapsed);
    }

    // Regression: back-to-back searches must not panic on Arc::get_mut
    #[test]
    fn test_back_to_back_searches() {
        let mut game = Game::new_infinite(None, None, None);
        for i in 0..3 {
            let time_limit = Duration::from_millis(200);
            let start = Instant::now();
            let (mv, _eval, depth, nodes) = game.find_best_move(None, Some(time_limit), None);
            let elapsed = start.elapsed();
            eprintln!("search {}: elapsed={:?}, depth={}, nodes={}", i, elapsed, depth, nodes);
            assert!(mv.is_some(), "expected a move for search {}", i);
            assert!(elapsed < Duration::from_millis(1000), "search {} took {:?}", i, elapsed);
        }
    }

    // Regression: external stop flag must be respected promptly
    #[test]
    fn test_external_stop_100ms() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        let mut game = Game::new_infinite(None, None, None);
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            stop2.store(true, Ordering::Relaxed);
        });
        let start = Instant::now();
        let (mv, _eval, depth, nodes) = game.find_best_move(None, None, Some(stop));
        let elapsed = start.elapsed();
        eprintln!("ext-stop 100ms test: elapsed={:?}, depth={}, nodes={}", elapsed, depth, nodes);
        assert!(mv.is_some(), "expected a move");
        assert!(elapsed < Duration::from_millis(500), "search took {:?}, did not stop promptly", elapsed);
    }

    // Measures score variance between depth and depth/2 across bench positions to calibrate
    // the singular extension margin. Run with: cargo test --release test_singular_margin -- --nocapture
    #[test]
    fn test_singular_margin() {
        use std::sync::{Arc, atomic::AtomicBool};
        use crate::eval::INF;
        use crate::repr::board::Board;
        use crate::search::{negamax::negamax, state::SearchState};

        let test_depths: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15, 16];
        let stop = Arc::new(AtomicBool::new(false));

        let positions = crate::eval::tests::TEST_POSITIONS;
        eprintln!("\n|score(depth) - score(depth/2)| across {} positions", positions.len());
        eprintln!("{:<8} {:>8} {:>8} {:>8} {:>8} {:>8}", "depth", "mean", "p50", "p75", "p90", "max");

        for &depth in test_depths {
            let half = depth / 2;
            let mut diffs: Vec<i16> = Vec::new();

            for &(_name, fen) in positions {
                let mut board = Board::from_fen(fen);

                let mut state_half = SearchState::new_default();
                state_half.new_search();
                state_half.max_depth = half + half / 2;
                let (_, score_half) = negamax(&stop, &mut board, &mut state_half, half, 0, -INF, INF, false);

                let mut state_full = SearchState::new_default();
                state_full.new_search();
                state_full.max_depth = depth + depth / 2;
                let (_, score_full) = negamax(&stop, &mut board, &mut state_full, depth, 0, -INF, INF, false);

                if score_full.abs() < crate::eval::MATE_CUTOFF && score_half.abs() < crate::eval::MATE_CUTOFF {
                    diffs.push((score_full - score_half).abs());
                }
            }

            diffs.sort();
            let n = diffs.len();
            let mean = diffs.iter().map(|&d| d as f64).sum::<f64>() / n as f64;
            let p50 = diffs[n * 50 / 100];
            let p75 = diffs[n * 75 / 100];
            let p90 = diffs[n * 90 / 100];
            let max = diffs[n - 1];
            eprintln!("{}/{:<4}   {:>8.0} {:>8} {:>8} {:>8} {:>8}   (cur margin: {})",
                depth, half, mean, p50, p75, p90, max, 2 * depth as i16);
        }
    }

    // Tests singular extension effectiveness on positions with known forcing lines.
    // Run twice — once with singular extensions enabled and once with depth >= 99 to disable —
    // and compare depth reached and nodes searched at equal time.
    // Run with: cargo test --release test_singular_effectiveness -- --nocapture
    #[test]
    fn test_singular_effectiveness() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        use crate::repr::board::Board;
        use crate::search::{MAX_PLY, negamax::search, state::SearchState};

        // Positions chosen for long forcing lines (sacrifices, forced exchanges, mating attacks)
        let positions: &[(&str, &str)] = &[
            ("Pos 1",   "6k1/1q4p1/5p1p/2Pn3Q/3p4/6P1/2r2PNP/5RK1 w - - 0 1"),
            ("Pos 2",  "8/1B2r1p1/5k1p/8/5P2/PR3K1b/8/8 b - - 0 1"),
            ("Rook vs passed pawns",   "5r1k/6pp/1n2Q3/4p3/8/7P/PP4PK/R1B1q3 b - - 0 1"),
            ("Endgame breakthrough",   "8/4p3/p2p4/2pP4/2P1P3/1P4k1/1P1K4/8 w - - 0 1"),
            ("Exchange sacrifice",     "2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - 0 1"),
        ];

        const TIME_PER_MOVE: Duration = Duration::from_millis(1000);
        const MOVES: usize = 4;

        eprintln!("\nSingular extension effectiveness (500ms/move, {} moves each)", MOVES);
        eprintln!("Disable SE by setting `depth >= 99` in negamax.rs and re-run to compare.\n");

        let mut total_nodes = 0u64;
        let mut total_depth = 0u32;

        for (name, fen) in positions {
            eprintln!("{}  [{}]", name, fen);
            eprintln!("  {:<5} {:>7} {:>5} {:>12} {:>8}  best", "move", "score", "depth", "nodes", "knps");

            let mut board = Board::from_fen(fen);
            let mut state = SearchState::new_default();

            for i in 0..MOVES {
                let stop = Arc::new(AtomicBool::new(false));
                let stop2 = Arc::clone(&stop);
                std::thread::spawn(move || {
                    std::thread::sleep(TIME_PER_MOVE);
                    stop2.store(true, Ordering::Relaxed);
                });
                let start = Instant::now();
                let (mv, score, depth, nodes) = search(&mut board, &mut state, MAX_PLY, &stop, Some(start + TIME_PER_MOVE));
                let elapsed = start.elapsed();
                let knps = nodes as f64 / elapsed.as_secs_f64() / 1000.0;
                total_nodes += nodes;
                total_depth += depth as u32;
                let mv_str = mv.map(|m| m.to_uci()).unwrap_or_else(|| "none".to_string());
                eprintln!("  {:<5} {:>7} {:>5} {:>12} {:>8.0}  {}", i + 1, score, depth, nodes, knps, mv_str);
                match mv {
                    Some(mv) => { let _ = board.makemove(mv); }
                    None => break,
                }
            }
            eprintln!();
        }

        let n = positions.len() as u32 * MOVES as u32;
        eprintln!("totals: {} nodes, avg depth {:.1}", total_nodes, total_depth as f64 / n as f64);
    }

    // Prints best move per depth from starting position to diagnose bad opening play.
    // Run with: cargo test test_opening_moves -- --nocapture
    #[test]
    fn test_opening_moves() {
        use std::sync::{Arc, atomic::AtomicBool};
        use crate::repr::board::Board;
        use crate::search::{negamax::search, state::SearchState};

        let mut board = Board::starting_position();
        let stop = Arc::new(AtomicBool::new(false));
        let mut state = SearchState::new_default();

        eprintln!("multi-threaded search:");
        eprintln!("{:<6} {:>8} {:>12}  best", "depth", "score", "nodes");
        for depth in 1..=12u8 {
            let start = Instant::now();
            let (mv, score, reached, nodes) = search(&mut board, &mut state, depth, &stop, None);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let best = mv.map(|m| m.to_uci()).unwrap_or_else(|| "none".to_string());
            eprintln!("{:<6} {:>8} {:>12}  {}  ({:.1}ms)", reached, score, nodes, best, ms);
        }
    }

    // Prints per-depth timing to calibrate the iterative deepening early-exit factor.
    // Run with: cargo test test_depth_timing -- --nocapture
    #[test]
    fn test_depth_timing() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        use crate::eval::INF;
        use crate::movegen::r#move::Move;
        use crate::repr::board::Board;
        use crate::search::{NUM_THREADS, negamax::{negamax, iterative_deepening}, state::SearchState};

        const MAX_TEST_DEPTH: u8 = 15;

        let positions: &[(&str, &[&str])] = &[
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &[]),
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &["e2e4", "e7e5", "g1f3", "b8c6"]),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", &[]),
            ("3r2k1/p2r1p1p/1p2p1p1/q4n2/3P4/PQ5P/1P1RNPP1/3R2K1 b - - 0 1", &[]),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", &[]),
            ("2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - 0 1", &[]),
            ("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3", &["g8f6", "d2d3"]),
        ];

        for (fen, moves) in positions {
            let mut board = Board::from_fen(fen);
            for mv_str in *moves {
                let mv = Move::from_uci(&board, mv_str);
                let _ = board.makemove(mv);
            }

            let stop = Arc::new(AtomicBool::new(false));
            let fake_stop = Arc::new(AtomicBool::new(false));
            let mut state = SearchState::new_default();
            state.new_search();

            // Spawn helper threads running full iterative deepening, same as real search()
            let mut threads = Vec::new();
            for _ in 1..NUM_THREADS {
                let mut helper_state = state.new_helper_thread();
                let mut helper_board = board.clone();
                let helper_stop = Arc::clone(&fake_stop);
                threads.push(std::thread::spawn(move || {
                    iterative_deepening(&mut helper_board, &mut helper_state, MAX_TEST_DEPTH, &helper_stop, None)
                }));
            }

            let label = if moves.is_empty() {
                fen[..fen.find(' ').unwrap_or(fen.len())].to_string()
            } else {
                format!("{} ..{}", &fen[..fen.find(' ').unwrap_or(fen.len())], moves.last().unwrap())
            };
            eprintln!("\n{}", label);
            eprintln!("{:<6} {:>10} {:>7} {:>7} {:>12}  best", "depth", "ms", "r(i-1)", "r(i-2)", "nodes");

            // Main thread does its own depth loop with per-depth timing, sharing TT with helpers
            let mut prev_ms = [0.0f64; 2];
            let mut prev_nodes = 0u64;
            for depth in 1..=MAX_TEST_DEPTH {
                state.max_depth = depth + depth / 2;
                let start = Instant::now();
                let (mv, score) = negamax(&stop, &mut board, &mut state, depth, 0, -INF, INF, false);
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                let delta_nodes = state.node_count - prev_nodes;
                let r1 = if prev_ms[0] > 0.5 { ms / prev_ms[0] } else { 0.0 };
                let r2 = if prev_ms[1] > 0.5 { ms / prev_ms[1] } else { 0.0 };
                let best = mv.map(|m| m.to_uci()).unwrap_or_else(|| "none".to_string());
                eprintln!("{:<6} {:>10.1} {:>7.2} {:>7.2}  {:>12}  {} ({})", depth, ms, r1, r2, delta_nodes, best, score);
                prev_ms[1] = prev_ms[0];
                prev_ms[0] = ms;
                prev_nodes = state.node_count;
            }

            fake_stop.store(true, Ordering::Relaxed);
            for t in threads { let _ = t.join(); }
        }
    }
}
