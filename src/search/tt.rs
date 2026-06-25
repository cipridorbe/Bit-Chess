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

pub trait Select {
    type Out;
    fn empty() -> Self::Out;
    #[inline]
    fn get<'a>(entry: &'a Self::Out) -> &'a TTEntry;
}
struct If<const shared: bool>();
impl Select for If<true> {
    type Out = UnsafeCell<TTEntry>;
    fn empty() -> Self::Out { UnsafeCell::new(TTEntry::empty()) }
    fn get<'a>(entry: &'a Self::Out) -> &'a TTEntry { unsafe { &*entry.get() }}
}
impl Select for If<false> {
    type Out = TTEntry;
    fn empty() -> Self::Out { TTEntry::empty() }
    fn get<'a>(entry: &'a Self::Out) -> &'a TTEntry { entry }
}

pub struct TT<const shared: bool> where If<shared>: Select {
    table: Vec<<If<shared> as Select>::Out>,
    mask: u64,
    generation: u8,
    generation_cutoff: u8,
}

impl<const shared: bool> TT<shared> where If<shared>: Select {

    /// new table of size 2^bits entries
    pub fn new(bits: u8, generation_cutoff: u8) -> Self {
        let mut table = Vec::with_capacity(1 << bits);
        for _ in 0..(1 << bits) {
            table.push(<If<shared> as Select>::empty());
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
        let entry = <If<shared> as Select>::get(&self.table[idx as usize]);
        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn generation(&self) -> u8 { self.generation }

    pub fn new_search(&mut self) { self.generation += 1 }
}

impl TT<true> {
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
        }
    }
}

impl TT<false> {
    pub fn insert(&mut self, mut entry: TTEntry, ply: u8) {
        let idx = entry.hash.0 & self.mask;
        entry.eval = adjust_insert_eval(entry.eval, ply);
        self.table[idx as usize] = entry;
    }
}

impl Clone for TT<false> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            mask: self.mask,
            generation: self.generation,
            generation_cutoff: self.generation_cutoff
        }
    }
}
unsafe impl<const shared: bool> Send for TT<shared> where If<shared> : Select {}
unsafe impl<const shared: bool> Sync for TT<shared> where If<shared> : Select {}

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