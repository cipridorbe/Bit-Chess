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
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
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
    fn tt_cutoff() {
        let depth = 6;
        let iters = 15;
        println!("\ndepth {depth}, {iters} moves per position:");
        for &cutoff in &[0u8, 1, 2, 5, 255] {
            let t = bench(true, depth, cutoff, iters);
            println!("  cutoff={cutoff:3}: {t:.2?}");
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
        let depth = 10;
        let iters = 7;
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
}