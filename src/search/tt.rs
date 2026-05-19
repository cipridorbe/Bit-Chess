use serde::de;

use crate::movegen::r#move::Move;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: f32,
    pub flag: TTFlag,
    pub best_move: Option<Move>,
    pub full_moves: u8,
}

impl TTEntry {
    fn empty() -> Self {
        TTEntry {
            hash: 0,
            depth: 0,
            score: 0.,
            flag: TTFlag::Exact,
            best_move: None,
            full_moves: 0,
        }
    }

    pub fn new(hash: u64, depth: u8, score: f32, flag: TTFlag, best_move: Option<Move>, full_moves: u8) -> Self {
        TTEntry {
            hash: hash,
            depth: depth,
            score: score,
            flag: flag,
            best_move: best_move,
            full_moves: full_moves
        }
    }
}

#[derive(Clone, Copy)]
pub enum TTFlag {
    Exact, Lower, Upper
}

pub struct TT {
    table: Vec<TTEntry>,
    mask: u64
}

impl TT {
    pub fn new(bits: u8) -> Self {
        TT {
            table: vec![TTEntry::empty(); 1 << bits],
            mask: (1 << bits) - 1
        }
    }

    pub fn find(&self, hash: u64) -> Option<TTEntry> {
        let entry = self.table[(hash & self.mask) as usize];
        if entry.hash == 0 || entry.hash != hash {
            None
        } else {
            Some(entry)
        }
    }

    pub fn insert(&mut self, entry: TTEntry) {
        let idx = (entry.hash & self.mask) as usize;
        if self.table[idx].hash == 0 || entry.depth >= self.table[idx].depth {
            self.table[idx] = entry;
        }
    }
}