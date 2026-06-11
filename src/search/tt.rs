use std::cell::UnsafeCell;

use crate::{eval::{Eval, MATE_CUTOFF}, movegen::r#move::Move, repr::hash::Hash};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone)]
pub struct TTEntry {
    pub hash: Hash,
    pub eval: Eval,
    pub flag: TTFlag,
    pub best_move: Option<Move>,
    pub depth: u8,
    pub generation: u8,
}

impl TTEntry {
    pub const fn empty() -> Self {
        TTEntry {
            hash: unsafe { std::mem::transmute(0u64) },
            eval: 0,
            flag: TTFlag::Exact,
            best_move: None,
            depth: 0,
            generation: 0,
        }
    }
    pub fn new(hash: Hash, eval: Eval, flag: TTFlag, best_move: Option<Move>, depth: u8, generation: u8) -> Self {
        TTEntry { hash, eval, flag, best_move, depth, generation }
    }
}

pub struct TT {
    table: Vec<UnsafeCell<TTEntry>>,
    mask: u64,
    generation: u8,
    generation_cutoff: u8,
}

impl TT {
    /// new table of size 2^bits entries
    pub fn new(bits: u8, generation_cutoff: u8) -> Self {
        let mut table = Vec::with_capacity(1 << bits);
        for _ in 0..(1 << bits) {
            table.push(UnsafeCell::new(TTEntry::empty()));
        }
        TT {
            table: table,
            mask: (1 << bits) - 1,
            generation: 0,
            generation_cutoff: generation_cutoff
        }
    }

    pub fn find(&self, hash: Hash) -> Option<&TTEntry> {
        let idx = hash.0 & self.mask;
        let entry = unsafe { &*self.table[idx as usize].get() };
        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn insert(&self, mut entry: TTEntry, ply: u8) {
        let idx = entry.hash.0 & self.mask;
        let current_entry = unsafe { &mut *self.table[idx as usize].get() };
        if entry.generation.wrapping_sub(current_entry.generation) >= self.generation_cutoff
            || entry.depth >= current_entry.depth || current_entry.hash.0 == 0
        {
            entry.eval = adjust_insert_eval(entry.eval, ply);
            let hash = entry.hash;
            entry.hash = Hash(0);
            *current_entry = entry;
            current_entry.hash = hash;
            // let p = current_entry as *mut TTEntry;
            // unsafe { std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).hash), Hash(0)) };
            // current_entry.eval = entry.eval;
            // current_entry.flag = entry.flag;
            // current_entry.best_move = entry.best_move;
            // current_entry.depth = entry.depth;
            // current_entry.generation = entry.generation;
            // std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
            // unsafe { std::ptr::write_volatile(std::ptr::addr_of_mut!((*p).hash), entry.hash) };
        }
    }

    pub fn generation(&self) -> u8 { self.generation }

    pub fn new_search(&mut self) { self.generation += 1 }
}

unsafe impl Send for TT {}
unsafe impl Sync for TT {}

fn adjust_insert_eval(eval: Eval, ply: u8) -> Eval {
    if eval.abs() < MATE_CUTOFF {
        eval
    } else {
        eval + ply as Eval * eval.signum()
    }
}

pub fn adjust_retrieve_eval(eval: Eval, ply: u8) -> Eval {
    if eval.abs() < MATE_CUTOFF {
        eval
    } else {
        eval - ply as Eval * eval.signum()
    }
}