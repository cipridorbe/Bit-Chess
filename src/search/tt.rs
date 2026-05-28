use crate::movegen::r#move::Move;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: i16,
    pub flag: TTFlag,
    pub best_move: Option<Move>,
    pub full_moves: u8,
    pub generation: u8,
}

impl TTEntry {
    fn empty() -> Self {
        TTEntry {
            hash: 0,
            depth: 0,
            score: 0,
            flag: TTFlag::Exact,
            best_move: None,
            full_moves: 0,
            generation: 0
        }
    }

    pub fn new(hash: u64, depth: u8, score: i16, flag: TTFlag, best_move: Option<Move>, full_moves: u8, generation: u8) -> Self {
        TTEntry {
            hash: hash,
            depth: depth,
            score: score,
            flag: flag,
            best_move: best_move,
            full_moves: full_moves,
            generation: generation
        }
    }
}

#[derive(Clone, Copy)]
pub enum TTFlag {
    Exact, Lower, Upper
}

pub struct TT {
    pub(crate) table: Vec<TTEntry>,
    pub(crate) mask: u64,
    pub(crate) generation: u8,
    pub(crate) generation_cutoff: u8,
    pub(crate) enabled: bool,
}

impl TT {
    pub fn new(bits: u8, generation_cutoff: u8) -> Self {
        TT {
            table: vec![TTEntry::empty(); 1 << bits],
            mask: (1 << bits) - 1,
            generation: 0,
            generation_cutoff: generation_cutoff,
            enabled: true,
        }
    }

    pub fn new_search(&mut self) {
        self.generation += 1;
    }

    pub fn new_disabled() -> Self {
        TT { table: vec![], mask: 0, generation: 0, generation_cutoff: 0, enabled: false }
    }

    pub fn find(&self, hash: u64) -> Option<TTEntry> {
        if !self.enabled { return None; }
        let entry = self.table[(hash & self.mask) as usize];
        if entry.hash == 0 || entry.hash != hash {
            None
        } else {
            Some(entry)
        }
    }

    pub fn insert(&mut self, entry: TTEntry) {
        if !self.enabled { return; }
        let idx = (entry.hash & self.mask) as usize;
        if self.table[idx].hash == 0 || entry.depth >= self.table[idx].depth || entry.generation - self.table[idx].generation > self.generation_cutoff {
            self.table[idx] = entry;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bitboard::Board, search::search};
    use std::time::Instant;

    const POSITIONS: &[&str] = &[
        // general
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

    fn bench(with_tt: bool, depth: u8, cutoff: u8, iters: i32) -> std::time::Duration {
        use crate::movegen::makemove::make_move;
        use crate::movegen::r#move::Move;
        let mut tt = if with_tt { TT::new(23, cutoff) } else { TT::new_disabled() };
        let mut history = [[0; 64]; 64];
        let mut counter_move = [[None::<Move>; 64]; 64];
        let start = Instant::now();
        for &fen in POSITIONS {
            let mut board = Board::from_fen(fen);
            for _ in 0..iters {
                if let Some(mv) = search(&mut board, depth, &mut tt, &mut history, &mut counter_move) {
                    make_move(&mut board, mv);
                }
            }
        }
        start.elapsed()
    }

    #[test]
    fn tt_speedup() {
        let depth = 6;
        let without = bench(false, depth, 0, 15);
        let with_tt = bench(true, depth, 2, 15);
        println!(
            "\ndepth {depth}: no TT = {without:.2?}, TT = {with_tt:.2?}, speedup = {:.1}x",
            without.as_secs_f64() / with_tt.as_secs_f64()
        );
        assert!(with_tt < without, "TT should be faster than no TT");
    }

    #[test]
    fn tt_age_weight() {
        // Compare quality-based eviction at different age penalty weights.
        // weight=0: depth-only (never evict a deeper entry regardless of age)
        // weight=N: an entry N generations old can be replaced by one N plies shallower
        // weight=255: effectively always-replace
        use crate::search::negamax::NODE_COUNT;
        use crate::movegen::makemove::make_move;
        use crate::movegen::r#move::Move;
        let depth = 7;
        let iters = 8;
        println!("\ndepth {depth}, {iters} moves per position ({} positions):", POSITIONS.len());
        println!("  {:>8}  {:>10}  {:>10}", "weight", "time", "nodes");
        for &weight in &[0u8, 1, 2, 4, 8, 255] {
            unsafe { NODE_COUNT = 0; }
            let mut tt = TT::new(22, weight);
            let mut history = [[0i16; 64]; 64];
            let mut counter_move = [[None::<Move>; 64]; 64];
            let start = Instant::now();
            for &fen in POSITIONS {
                let mut board = Board::from_fen(fen);
                for _ in 0..iters {
                    if let Some(mv) = search(&mut board, depth, &mut tt, &mut history, &mut counter_move) {
                        make_move(&mut board, mv);
                    }
                }
            }
            let elapsed = start.elapsed();
            let nodes = unsafe { NODE_COUNT };
            println!("  {:>8}  {:>10.2?}  {:>10}", weight, elapsed, nodes);
        }
    }

    #[test]
    fn tt_entry_size() {
        println!("Size of TT Entry: {} bytes", std::mem::size_of::<TTEntry>());
    }

    #[test]
    fn node_count() {
        use crate::search::negamax::NODE_COUNT;
        use crate::movegen::makemove::make_move;
        use crate::movegen::r#move::Move;
        let depth = 9;
        let iters = 4;
        // let mut tt = TT::new_disabled();
        let mut tt = TT::new(22, 2);
        let mut history = [[0; 64]; 64];
        let mut counter_move = [[None::<Move>; 64]; 64];
        unsafe { NODE_COUNT = 0; }
        let start = Instant::now();
        for &fen in POSITIONS {
            let mut board = Board::from_fen(fen);
            for _ in 0..iters {
                if let Some(mv) = search(&mut board, depth, &mut tt, &mut history, &mut counter_move) {
                    make_move(&mut board, mv);
                }
            }
        }
        let elapsed = start.elapsed();
        let nodes = unsafe { NODE_COUNT };
        println!(
            "\ndepth {depth}, {iters} moves × {} positions: {} nodes in {elapsed:.2?} ({:.0} knps)",
            POSITIONS.len(),
            nodes,
            nodes as f64 / elapsed.as_secs_f64() / 1000.0,
        );
    }

    #[test]
    fn tt_hit_rate() {
        use crate::search::negamax::{
            NODE_COUNT,
            TT_LOOKUPS_DEPTH, TT_LOOKUPS_DEPTH_SUCCESS,
            TT_LOOKUPS_PLY,   TT_LOOKUPS_PLY_SUCESS,
        };
        use crate::movegen::makemove::make_move;
        use crate::movegen::r#move::Move;

        fn hit_color(pct: f64) -> &'static str {
            if pct >= 75.0 { "\x1b[32m" }       // green
            else if pct >= 40.0 { "\x1b[33m" }  // yellow
            else { "\x1b[31m" }                  // red
        }
        const RESET: &str = "\x1b[0m";

        let search_depth = 9;
        let iters = 4;
        let mut tt = TT::new(22, 0);
        let mut history = [[0i16; 64]; 64];
        let mut counter_move = [[None::<Move>; 64]; 64];

        unsafe {
            NODE_COUNT = 0;
            TT_LOOKUPS_DEPTH = [0; 64];
            TT_LOOKUPS_PLY   = [0; 64];
            TT_LOOKUPS_DEPTH_SUCCESS = [0; 64];
            TT_LOOKUPS_PLY_SUCESS    = [0; 64];
        }

        let start = Instant::now();
        for &fen in POSITIONS {
            let mut board = Board::from_fen(fen);
            for _ in 0..iters {
                if let Some(mv) = search(&mut board, search_depth, &mut tt, &mut history, &mut counter_move) {
                    make_move(&mut board, mv);
                }
            }
        }
        let elapsed = start.elapsed();

        let (nodes, by_depth_l, by_depth_h, by_ply_l, by_ply_h) = unsafe {(
            NODE_COUNT,
            TT_LOOKUPS_DEPTH,
            TT_LOOKUPS_DEPTH_SUCCESS,
            TT_LOOKUPS_PLY,
            TT_LOOKUPS_PLY_SUCESS,
        )};

        println!(
            "\ndepth {search_depth}, {iters} moves × {} positions: {} nodes in {elapsed:.2?} ({:.0} knps)",
            POSITIONS.len(), nodes,
            nodes as f64 / elapsed.as_secs_f64() / 1000.0,
        );

        // ── By depth ──────────────────────────────────────────────────────────
        println!("\n  TT lookups by depth (remaining depth):");
        println!("  {:>5}  {:>10}  {:>10}  {:>8}  {:>10}", "depth", "lookups", "hits", "hit%", "misses");
        println!("  {:>5}  {:>10}  {:>10}  {:>8}  {:>10}", "-----", "-------", "----", "----", "------");
        for d in 0..64usize {
            let l = by_depth_l[d];
            if l == 0 { continue; }
            let h = by_depth_h[d];
            let pct = 100.0 * h as f64 / l as f64;
            let col = hit_color(pct);
            println!(
                "  {:>5}  {:>10}  {:>10}  {col}{:>7.1}%{RESET}  {:>10}",
                d, l, h, pct, l - h,
            );
        }

        // ── By ply ────────────────────────────────────────────────────────────
        println!("\n  TT lookups by ply (distance from root):");
        println!("  {:>5}  {:>10}  {:>10}  {:>8}  {:>10}", "ply", "lookups", "hits", "hit%", "misses");
        println!("  {:>5}  {:>10}  {:>10}  {:>8}  {:>10}", "---", "-------", "----", "----", "------");
        for p in 0..64usize {
            let l = by_ply_l[p];
            if l == 0 { continue; }
            let h = by_ply_h[p];
            let pct = 100.0 * h as f64 / l as f64;
            let col = hit_color(pct);
            println!(
                "  {:>5}  {:>10}  {:>10}  {col}{:>7.1}%{RESET}  {:>10}",
                p, l, h, pct, l - h,
            );
        }
    }
}