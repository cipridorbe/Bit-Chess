use std::{path::Path, time::Instant, u8};

use crate::{egtb::{KINGS_IDX_PAWNFUL, NUM_KINGS_PAWNFUL, threepiece::{paged::PagedFiles, pos::Pos, reachable_files::three_piece_targets, spillable::{self, SourceBudget, SpillableBucket}}}, repr::{bitboard::BB, board::Board, colour::Colour, piece::Piece, square::Square}, test_assert};

const APPROX_TOTAL_SLOTS: u64 = 34_389_622_399;
const LOG_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

const UNTOUCHED: u8 = 0;
const RESOLVED: u8 = 255;

struct Frontier {
    current: Vec<SpillableBucket>,
    next: Vec<SpillableBucket>,
    active_current: Vec<usize>,
}

impl Frontier {
    // `current`/`next` ping-pong between two fixed on-disk label sets across the whole
    // run (see advance()) rather than being recreated each layer -- by the time a bucket
    // is handed off to become the new `next`, pop() has already fully drained it.
    fn new(dir: &Path, num_files: usize) -> Self {
        Self {
            current: spillable::new_buckets(dir, "a", num_files),
            next: spillable::new_buckets(dir, "b", num_files),
            active_current: Vec::new(),
        }
    }

    fn push(&mut self, file: usize, pos: Pos, status: Status, budget: Option<u64>) {
        self.next[file].push(pos, status, budget);
    }

    // A target may have grown under an earlier, more generous budget and then gone
    // quiet -- push() only re-checks on an actual push, so enforce the incoming budget
    // on everything it governs as soon as a source becomes active.
    fn enter_source(&mut self, budget: &SourceBudget) {
        self.next[budget.file()].enforce_budget(budget.budget_for_target(budget.file()));
        for target in three_piece_targets(budget.file()) {
            self.next[target].enforce_budget(budget.budget_for_target(target));
        }
    }

    fn pop(&mut self) -> Option<(Pos, Status)> {
        while let Some(&file) = self.active_current.last() {
            if let Some(item) = self.current[file].pop() {
                return Some(item);
            }
            self.active_current.pop();
        }
        None
    }

    // Swaps `next` into `current`; returns false once both are empty (generation done).
    fn advance(&mut self) -> bool {
        std::mem::swap(&mut self.current, &mut self.next);
        self.active_current = (0..self.current.len()).filter(|&i| !self.current[i].is_empty()).collect();
        !self.active_current.is_empty()
    }
}

impl Pos {
    // `dir` holds the mmap-backed per-material-file tablebase, persisted directly to
    // disk as it's built (no separate save step, though flush_all() at the end still
    // matters for durability -- writes aren't guaranteed to have hit disk until then).
    pub fn generate(dir: impl AsRef<Path>) {
        let dir = dir.as_ref();
        let mut moves_left = PagedFiles::new(dir.join("moves_left"), "ml", Pos::NUM_FILES);
        let mut status = PagedFiles::new(dir.join("status"), "st", Pos::NUM_FILES);
        let mut frontier = Self::init(dir, &mut moves_left);

        let start = Instant::now();
        let mut last_log = start;
        let mut resolved: u64 = 0;
        let mut layer: u32 = 0;
        // caches the push budget for whichever file is currently being drained --
        // Frontier::pop() drains one file's current bucket fully before moving to the
        // next, so this only needs recomputing when pos.file() actually changes. Seeded
        // with an out-of-range file so the very first iteration always recomputes.
        let mut source_budget = SourceBudget::for_file(Pos::NUM_FILES);

        loop {
            let Some((pos, state)) = frontier.pop() else {
                layer += 1;
                if !frontier.advance() { break; }
                continue;
            };
            *status.get_mut(pos.file(), pos.index()) = state.0 as u8;
            resolved += 1;
            if last_log.elapsed() >= LOG_INTERVAL {
                last_log = Instant::now();
                let elapsed = start.elapsed().as_secs_f64();
                let rate = resolved as f64 / elapsed;
                let frac = resolved as f64 / APPROX_TOTAL_SLOTS as f64;
                let eta_secs = (APPROX_TOTAL_SLOTS as f64 - resolved as f64) / rate.max(1.0);
                eprintln!("generate: layer={layer} resolved={resolved} ({:.4}% of approx total) elapsed={:.0}s rate={:.0}/s eta~={:.0}s",
                    frac * 100.0, elapsed, rate, eta_secs);
            }
            let next_state = state.next();
            #[cfg(feature = "assertions")]
            let pos_debug = format!("{pos:?}");
            let source_file = pos.file();
            if source_budget.file() != source_file {
                source_budget = SourceBudget::for_file(source_file);
                frontier.enter_source(&source_budget);
            }
            for new_pos in pos.predecessors() {
                let (file, index) = (new_pos.file(), new_pos.index());
                let left = moves_left.get_mut(file, index);
                if *left == RESOLVED { continue; }
                test_assert!(*left != UNTOUCHED,
                    "moves_left untouched: predecessor {new_pos:?} (file={file} index={index}) of popped {pos_debug} (state={})",
                    state.0);
                *left -= 1;
                if state.is_loss() || *left == 0 {
                    *left = RESOLVED;
                    let budget = source_budget.budget_for_target(file);
                    frontier.push(file, new_pos, next_state, budget);
                }
            }
        }

        eprintln!("generate: done, {resolved} resolved, {layer} layers, {:.0}s elapsed", start.elapsed().as_secs_f64());
        moves_left.flush_all().expect("flush moves_left");
        status.flush_all().expect("flush status");
    }

    // Only ever writes moves_left: draws need no storage at all (a childless position's
    // own moves_left slot is never read, since nothing ever decrements it -- only a
    // position's children trigger that -- and draws are never popped since they're never
    // pushed), and checkmates' status is deferred to generate()'s dequeue-time write, so
    // init() only needs to seed moves_left and push checkmates into the frontier.
    fn init(dir: &Path, moves_left: &mut PagedFiles) -> Frontier {
        let mut frontier = Frontier::new(&dir.join("frontier"), Pos::NUM_FILES);
        let start = Instant::now();
        let mut last_log = start;
        let mut king_combos: u64 = 0;
        let total_king_combos = 2 * NUM_KINGS_PAWNFUL as u64; // last_moved_iter() x king_iter()
        for last_moved in Self::last_moved_iter() {
            for king in Self::king_iter() {
                king_combos += 1;
                if last_log.elapsed() >= LOG_INTERVAL {
                    last_log = Instant::now();
                    let elapsed = start.elapsed().as_secs_f64();
                    let frac = king_combos as f64 / total_king_combos as f64;
                    let eta_secs = elapsed / frac.max(1e-9) - elapsed;
                    eprintln!("init: {king_combos}/{total_king_combos} king combos ({:.1}%) elapsed={:.0}s eta~={:.0}s",
                        frac * 100.0, elapsed, eta_secs);
                }
                for p1 in Self::p1_iter(king) {
                    for p2 in Self::p2_iter(king, p1) {
                        for p3 in Self::p3_iter(king, p1, p2) {
                            let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() || pos.in_check(pos.last_moved) {
                                continue;
                            }
                            for enpassant in Self::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                let file = pos.file();
                                let index = pos.index();
                                let num_moves = pos.count_distinct_canonical_successors();
                                if num_moves == 0 {
                                    if pos.in_check(!pos.last_moved) {
                                        *moves_left.get_mut(file, index) = RESOLVED;
                                        let budget = spillable::own_file_budget(file);
                                        frontier.push(file, pos.clone(), Status::CHECKMATED, budget);
                                    }
                                    // else: draw, nothing to write (see doc comment above)
                                } else {
                                    test_assert!(num_moves < RESOLVED as usize);
                                    *moves_left.get_mut(file, index) = num_moves as u8;
                                }
                            }
                        }
                    }
                }
            }
        }
        eprintln!("init: done, {:.0}s elapsed", start.elapsed().as_secs_f64());
        frontier
    }

    // iterator over all valid last_moved values
    fn last_moved_iter() -> impl Iterator<Item = Colour> {
        [Colour::White, Colour::Black].into_iter()
    }

    // iterator over all valid king values
    fn king_iter() -> impl Iterator<Item = [Square; 2]> {
        Square::all()
            .filter(|wk| wk.file() < 4)
            .flat_map(|wk| Square::all()
                .filter(move |bk| KINGS_IDX_PAWNFUL[wk as usize][*bk] != u16::MAX)
                .map(move |bk| [wk, bk]))
    }

    #[inline]
    fn smaller_or_equal_pieces(piece: Piece) -> &'static [Piece] {
        const QUEEN: [Piece; 10] = [Piece::WhitePawn, Piece::BlackPawn, Piece::WhiteKnight, Piece::BlackKnight, Piece::WhiteBishop, Piece::BlackBishop, Piece::WhiteRook, Piece::BlackRook, Piece::WhiteQueen, Piece::BlackQueen];
        match piece {
            Piece::WhitePawn | Piece::BlackPawn => &QUEEN[..2],
            Piece::WhiteKnight | Piece::BlackKnight => &QUEEN[..4],
            Piece::WhiteBishop | Piece::BlackBishop => &QUEEN[..6],
            Piece::WhiteRook | Piece::BlackRook => &QUEEN[..8],
            Piece::WhiteQueen | Piece::BlackQueen => &QUEEN[..10],
            Piece::WhiteKing | Piece::BlackKing => panic!("king has no smaller-or-equal-pieces list"),
        }
    }

    #[inline]
    // Squares a pawn can occupy (ranks 2-7) are exactly indices 8..56, since
    // Square::rank() == self as u8 / 8 -- a contiguous slice of ALL_SQUARES.
    fn squares_for(is_pawn: bool) -> &'static [Square] {
        const ALL_SQUARES: [Square; 64] = {
            let mut out = [Square::a1; 64];
            let mut i = 0;
            while i < 64 {
                out[i] = Square::from_u8(i as u8);
                i += 1;
            }
            out
        };
        if is_pawn { &ALL_SQUARES[8..56] } else { &ALL_SQUARES }
    }

    #[inline]
    // iterator over all valid p1 values, dependent on king values
    fn p1_iter(king: [Square; 2]) -> impl Iterator<Item = (Square, Piece)> {
        const P1_KINDS: [Piece; 5] = [Piece::WhitePawn, Piece::WhiteKnight, Piece::WhiteBishop, Piece::WhiteRook, Piece::WhiteQueen];
        let empty = !(king[0].bb() | king[1].bb());
        P1_KINDS.into_iter()
            .flat_map(move |piece| Self::squares_for(piece == Piece::WhitePawn).iter().copied().map(move |square| (square, piece)))
            .filter(move |(square, _)| square.bb() & empty != 0)
    }

    #[inline]
    // iterator over all valid p2 values, dependent on king and p1
    fn p2_iter(king: [Square; 2], p1: (Square, Piece)) -> impl Iterator<Item = Option<(Square, Piece)>> {
        let above_diagonal = { let (rank, file) = king[Colour::White].rank_file(); rank > file };
        let empty = !(king[0].bb() | king[1].bb() | p1.0.bb());
        let some_iter = Self::smaller_or_equal_pieces(p1.1).iter().copied()
            .flat_map(move |piece| Self::squares_for(piece.is_pawn()).iter().copied().map(move |square| (square, piece)))
            .filter(move |(square, _)| square.bb() & empty != 0)
            .map(|p2| Some(p2));
        let none_option = if above_diagonal && p1.1 != Piece::WhitePawn { None } else { Some(None) };
        none_option.into_iter().chain(some_iter)
    }

    #[inline]
    // iterator over all valid p3 values, dependent on king, p1, and p2
    fn p3_iter(king: [Square; 2], p1: (Square, Piece), p2: Option<(Square, Piece)>) -> impl Iterator<Item = Option<(Square, Piece)>> {
        let above_diagonal = { let (rank, file) = king[Colour::White].rank_file(); rank > file };
        let has_pawn = p1.1 == Piece::WhitePawn || p2.is_some_and(|p2| p2.1.is_pawn());
        let must_be_pawn = above_diagonal && !has_pawn;
        let some_iter = p2.into_iter()
            .flat_map(move |p2| {
                let pieces = if must_be_pawn { Self::smaller_or_equal_pieces(Piece::WhitePawn) } else { Self::smaller_or_equal_pieces(p2.1) };
                let empty = !(king[0].bb() | king[1].bb() | p1.0.bb() | p2.0.bb());
                pieces.iter().copied()
                    .flat_map(move |piece| Self::squares_for(piece.is_pawn()).iter().copied().map(move |square| (square, piece)))
                    .filter(move |(square, _)| square.bb() & empty != 0)
            })
            .map(|p3| Some(p3));
        let none_option = if must_be_pawn { None } else { Some(None) };
        none_option.into_iter().chain(some_iter)
    }

    // at most 3 candidate enpassant squares, one per piece that could be the pawn that
    // just double-pushed (right colour, right rank)
    pub(crate) fn enpassant_candidates(pos: &Pos) -> [Option<Square>; 3] {
        let has_both_colours = pos.p2.is_some_and(|p2| p2.1.is_pawn()) && {
            let pieces = || std::iter::once(pos.p1).chain(pos.p2).chain(pos.p3);
            pieces().any(|(_, p)| p == Piece::WhitePawn) && pieces().any(|(_, p)| p == Piece::BlackPawn)
        };
        if !has_both_colours {
            return [None, None, None];
        }
        let (ep_rank, pawn_rank, pawn_piece) = match pos.last_moved {
            Colour::White => (2, 3, Piece::WhitePawn),
            Colour::Black => (5, 4, Piece::BlackPawn),
        };
        let candidate = |p: (Square, Piece)| {
            let (rank, file) = p.0.rank_file();
            (rank == pawn_rank && p.1 == pawn_piece).then(|| Square::from_rank_file(ep_rank, file))
        };
        [candidate(pos.p1), pos.p2.and_then(candidate), pos.p3.and_then(candidate)]
    }

    // iterator over all valid enpassant values given a position
    fn enpassant_iter(pos: Pos) -> impl Iterator<Item = Option<Square>> {
        let some_iter = Self::enpassant_candidates(&pos).into_iter()
            .flatten()
            .filter(move |&sq| pos.enpassant_possible(sq, pos.last_moved))
            .map(Some);
        std::iter::once(None).chain(some_iter)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(i8);

impl Status {
    pub const CHECKMATED: Status = Status(-1);
    pub const DRAW: Status = Status(0);
    pub const UNKOWN: Status = Status(i8::MIN);

    fn next(self) -> Self {
        if self.0.abs() == 127 {
            Self(-self.0)
        } else if self.0 > 0 {
            Self(-self.0 - 1)
        } else if self.0 < 0 && self != Self::UNKOWN {
            Self(-self.0 + 1)
        } else {
            panic!("cannot call next on draw/unkown")
        }
    }

    pub fn is_win(self) -> bool {
        self.0 > 0
    }

    pub fn is_loss(self) -> bool {
        self.0 < 0 && self != Self::UNKOWN
    }

    // For on-disk spilling (see spillable.rs) -- #[repr(transparent)] over i8 already
    // guarantees this is just a reinterpret, these just avoid exposing the field itself.
    pub(crate) fn to_byte(self) -> u8 { self.0 as u8 }
    pub(crate) fn from_byte(b: u8) -> Self { Self(b as i8) }
}

#[cfg(test)]
mod size_estimate {
    use super::*;
    use crate::{egtb::{NUM_KINGS_PAWNFUL, NUM_KINGS_PAWNLESS, KINGS_IDX_PAWNLESS, threepiece::reflection::Reflection}, movegen::tables::KING_ATTACKS};

    // For every valid pawnful (wk,bk) pair, applies every legal single-step white-king
    // and (separately) black-king move, reflects back to canonical form exactly as
    // make_canonical would (horizontal flip if the moved white king crosses file>=4),
    // and measures the resulting king_idx jump -- to size sub-file chunking correctly.
    #[test]
    fn king_idx_jump_from_king_moves() {
        let mut max_white_delta: i64 = 0;
        let mut max_black_delta: i64 = 0;
        let mut white_moves = 0usize;
        let mut black_moves = 0usize;
        let mut white_delta_hist: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();

        for wk_raw in 0..64u8 {
            let wk = Square::from_u8(wk_raw);
            if wk.file() >= 4 { continue; }
            for bk_raw in 0..64u8 {
                let bk = Square::from_u8(bk_raw);
                let old_idx = KINGS_IDX_PAWNFUL[wk][bk];
                if old_idx == u16::MAX { continue; }

                for wk2 in KING_ATTACKS[wk].squares() {
                    if wk2 == bk || KING_ATTACKS[bk] & wk2.bb() != 0 { continue; }
                    let (new_wk, new_bk) = if wk2.file() >= 4 {
                        (Reflection::Horizontal.apply(wk2), Reflection::Horizontal.apply(bk))
                    } else {
                        (wk2, bk)
                    };
                    let new_idx = KINGS_IDX_PAWNFUL[new_wk][new_bk];
                    if new_idx == u16::MAX { continue; }
                    let delta = new_idx as i64 - old_idx as i64;
                    if delta.abs() > max_white_delta { max_white_delta = delta.abs(); }
                    *white_delta_hist.entry(delta.abs() / 100).or_insert(0) += 1;
                    white_moves += 1;
                }

                for bk2 in KING_ATTACKS[bk].squares() {
                    if bk2 == wk || KING_ATTACKS[wk] & bk2.bb() != 0 { continue; }
                    let new_idx = KINGS_IDX_PAWNFUL[wk][bk2];
                    if new_idx == u16::MAX { continue; }
                    let delta = (new_idx as i64 - old_idx as i64).abs();
                    if delta > max_black_delta { max_black_delta = delta; }
                    black_moves += 1;
                }
            }
        }
        println!("NUM_KINGS_PAWNFUL = {NUM_KINGS_PAWNFUL}");
        println!("white king move: max king_idx delta = {max_white_delta}, over {white_moves} moves");
        println!("black king move: max king_idx delta = {max_black_delta}, over {black_moves} moves");
        println!("white delta histogram (bucket=delta/100 -> count): {white_delta_hist:?}");
    }

    // For candidate sub-file counts n=1..16, chunks NUM_KINGS_PAWNFUL into n pieces and
    // finds the worst-case (over every starting (wk,bk)) number of DISTINCT chunks that a
    // single popped pawnful-starting 2-piece position's predecessors could touch -- the
    // union of {no king move} + {every legal white king move} + {every legal black king
    // move}, each of which fans out across the 5 opponent-coloured uncapture piece types
    // (UNCAPTURES is locked to last_moved, not both colours).
    #[test]
    fn subfile_memory_sweep() {
        const PAWNFUL_FILE_SLOTS: u64 = 655_708_032;
        // Predecessor fan-out only ever touches moves_left: status is deferred to dequeue
        // time (carried in the queue entry, moves_left's own 0 doubling as "resolved").
        const BYTES_PER_SLOT: u64 = 1;
        const MATERIAL_TYPES: u64 = 5;
        let slots_per_king_idx = PAWNFUL_FILE_SLOTS / NUM_KINGS_PAWNFUL as u64;

        let mut reachable_sets: Vec<Vec<u16>> = Vec::new();
        for wk_raw in 0..64u8 {
            let wk = Square::from_u8(wk_raw);
            if wk.file() >= 4 { continue; }
            for bk_raw in 0..64u8 {
                let bk = Square::from_u8(bk_raw);
                let old_idx = KINGS_IDX_PAWNFUL[wk][bk];
                if old_idx == u16::MAX { continue; }
                let mut reachable = vec![old_idx];
                for wk2 in KING_ATTACKS[wk].squares() {
                    if wk2 == bk || KING_ATTACKS[bk] & wk2.bb() != 0 { continue; }
                    let (new_wk, new_bk) = if wk2.file() >= 4 {
                        (Reflection::Horizontal.apply(wk2), Reflection::Horizontal.apply(bk))
                    } else { (wk2, bk) };
                    let new_idx = KINGS_IDX_PAWNFUL[new_wk][new_bk];
                    if new_idx != u16::MAX { reachable.push(new_idx); }
                }
                for bk2 in KING_ATTACKS[bk].squares() {
                    if bk2 == wk || KING_ATTACKS[wk] & bk2.bb() != 0 { continue; }
                    let new_idx = KINGS_IDX_PAWNFUL[wk][bk2];
                    if new_idx != u16::MAX { reachable.push(new_idx); }
                }
                reachable_sets.push(reachable);
            }
        }

        println!("n | avg/max distinct_chunks | bytes/chunk (1 material) | avg/worst total (5 materials)");
        for n in 1..=16u64 {
            let chunk_size = (NUM_KINGS_PAWNFUL as u64 + n - 1) / n; // ceil
            let mut max_distinct_chunks = 0usize;
            let mut sum_distinct_chunks = 0u64;
            for reachable in &reachable_sets {
                let mut chunks: Vec<u64> = reachable.iter().map(|&idx| idx as u64 / chunk_size).collect();
                chunks.sort_unstable();
                chunks.dedup();
                max_distinct_chunks = max_distinct_chunks.max(chunks.len());
                sum_distinct_chunks += chunks.len() as u64;
            }
            let avg_distinct_chunks = sum_distinct_chunks as f64 / reachable_sets.len() as f64;
            let bytes_per_chunk = slots_per_king_idx * chunk_size * BYTES_PER_SLOT;
            let worst_case_bytes = bytes_per_chunk * max_distinct_chunks as u64 * MATERIAL_TYPES;
            let avg_case_bytes = bytes_per_chunk as f64 * avg_distinct_chunks * MATERIAL_TYPES as f64;
            println!("n={n:2} | avg_chunks={avg_distinct_chunks:.2} max_chunks={max_distinct_chunks:2} | bytes/chunk={:8.1}MB | avg_total={:.3}GB worst_total={:.3}GB",
                bytes_per_chunk as f64 / 1e6, avg_case_bytes / 1e9, worst_case_bytes as f64 / 1e9);
        }
    }

    // Same analysis as subfile_memory_sweep, but for a PAWNLESS-starting 2-piece position.
    // Of the 5 opponent-coloured uncapture types, 1 (the pawn) produces a pawnful predecessor
    // (indexed via KINGS_IDX_PAWNFUL, horizontal-fold-only canonicalization) and 4 (the
    // non-pawns) stay pawnless (indexed via KINGS_IDX_PAWNLESS, full horizontal+vertical+
    // diagonal-fold canonicalization) -- a king move's raw destination is the same either
    // way, but which canonicalization applies to it differs by which piece got uncaptured.
    #[test]
    fn subfile_memory_sweep_pawnless_start() {
        const PAWNFUL_FILE_SLOTS: u64 = 655_708_032;
        const PAWNLESS_FILE_SLOTS: u64 = 209_674_080;
        // Predecessor fan-out only ever touches moves_left: status is deferred to dequeue
        // time (carried in the queue entry, moves_left's own 0 doubling as "resolved").
        const BYTES_PER_SLOT: u64 = 1;
        let slots_per_king_idx_pawnful = PAWNFUL_FILE_SLOTS / NUM_KINGS_PAWNFUL as u64;
        let slots_per_king_idx_pawnless = PAWNLESS_FILE_SLOTS / NUM_KINGS_PAWNLESS as u64;

        fn canon_pawnful(wk: Square, bk: Square) -> (Square, Square) {
            if wk.file() >= 4 {
                (Reflection::Horizontal.apply(wk), Reflection::Horizontal.apply(bk))
            } else {
                (wk, bk)
            }
        }

        fn canon_pawnless(wk: Square, bk: Square) -> (Square, Square) {
            let (mut wk, mut bk) = (wk, bk);
            if wk.file() >= 4 {
                wk = Reflection::Horizontal.apply(wk);
                bk = Reflection::Horizontal.apply(bk);
            }
            if wk.rank() >= 4 {
                wk = Reflection::Vertical.apply(wk);
                bk = Reflection::Vertical.apply(bk);
            }
            let (wr, wf) = wk.rank_file();
            if wr > wf {
                wk = Reflection::Diagonal.apply(wk);
                bk = Reflection::Diagonal.apply(bk);
            }
            let (wr, wf) = wk.rank_file();
            if wr == wf {
                let (br, bf) = bk.rank_file();
                if br > bf {
                    bk = Reflection::Diagonal.apply(bk);
                }
            }
            (wk, bk)
        }

        // reachable[i] = (pawnful_king_idx, pawnless_king_idx) for each of the {no move} +
        // {white king moves} + {black king moves} raw destinations, each re-canonicalized
        // both ways since which one applies depends on the uncapture choice.
        struct Entry { pawnful: Vec<u16>, pawnless: Vec<u16> }
        let mut entries: Vec<Entry> = Vec::new();

        for wk_raw in 0..64u8 {
            let wk = Square::from_u8(wk_raw);
            let (wr, wf) = wk.rank_file();
            if wf >= 4 || wr >= 4 || wr > wf { continue; }
            for bk_raw in 0..64u8 {
                let bk = Square::from_u8(bk_raw);
                if KINGS_IDX_PAWNLESS[wk_raw as usize][bk] == u16::MAX { continue; }

                let raw_destinations: Vec<(Square, Square)> = std::iter::once((wk, bk))
                    .chain(KING_ATTACKS[wk].squares()
                        .filter(|&wk2| wk2 != bk && KING_ATTACKS[bk] & wk2.bb() == 0)
                        .map(|wk2| (wk2, bk)))
                    .chain(KING_ATTACKS[bk].squares()
                        .filter(|&bk2| bk2 != wk && KING_ATTACKS[wk] & bk2.bb() == 0)
                        .map(|bk2| (wk, bk2)))
                    .collect();

                let mut pawnful = Vec::new();
                let mut pawnless = Vec::new();
                for (rwk, rbk) in raw_destinations {
                    let (pwk, pbk) = canon_pawnful(rwk, rbk);
                    let pf_idx = KINGS_IDX_PAWNFUL[pwk][pbk];
                    if pf_idx != u16::MAX { pawnful.push(pf_idx); }

                    let (lwk, lbk) = canon_pawnless(rwk, rbk);
                    let pl_idx = KINGS_IDX_PAWNLESS[lwk as usize][lbk];
                    if pl_idx != u16::MAX { pawnless.push(pl_idx); }
                }
                entries.push(Entry { pawnful, pawnless });
            }
        }

        println!("n | avg/max chunks (1 pawnful type) | avg/max chunks (4 pawnless types) | avg/worst total");
        for n in 1..=16u64 {
            let chunk_pawnful = (NUM_KINGS_PAWNFUL as u64 + n - 1) / n;
            let chunk_pawnless = (NUM_KINGS_PAWNLESS as u64 + n - 1) / n;
            let bytes_pf = slots_per_king_idx_pawnful * chunk_pawnful * BYTES_PER_SLOT;
            let bytes_pl = slots_per_king_idx_pawnless * chunk_pawnless * BYTES_PER_SLOT;

            let mut max_pf_chunks = 0usize;
            let mut max_pl_chunks = 0usize;
            let mut sum_pf_chunks = 0u64;
            let mut sum_pl_chunks = 0u64;
            let mut max_total = 0f64;
            let mut sum_total = 0f64;
            for e in &entries {
                let mut pf: Vec<u64> = e.pawnful.iter().map(|&i| i as u64 / chunk_pawnful).collect();
                pf.sort_unstable(); pf.dedup();
                let mut pl: Vec<u64> = e.pawnless.iter().map(|&i| i as u64 / chunk_pawnless).collect();
                pl.sort_unstable(); pl.dedup();

                max_pf_chunks = max_pf_chunks.max(pf.len());
                max_pl_chunks = max_pl_chunks.max(pl.len());
                sum_pf_chunks += pf.len() as u64;
                sum_pl_chunks += pl.len() as u64;

                // 1 pawn-uncapture type -> pawnful chunks; 4 non-pawn types -> pawnless chunks
                let total = bytes_pf as f64 * pf.len() as f64 + bytes_pl as f64 * pl.len() as f64 * 4.0;
                max_total = max_total.max(total);
                sum_total += total;
            }
            let avg_pf = sum_pf_chunks as f64 / entries.len() as f64;
            let avg_pl = sum_pl_chunks as f64 / entries.len() as f64;
            let avg_total = sum_total / entries.len() as f64;
            println!("n={n:2} | pawnful avg={avg_pf:.2} max={max_pf_chunks:2} ({:.1}MB/chunk) | pawnless avg={avg_pl:.2} max={max_pl_chunks:2} ({:.1}MB/chunk) | avg_total={:.3}GB worst_total={:.3}GB",
                bytes_pf as f64 / 1e6, bytes_pl as f64 / 1e6, avg_total / 1e9, max_total / 1e9);
        }

        // Exact combined worst/avg case for the specific (n_pawnful=8, n_pawnless=4) pair,
        // computed per-entry (not by summing each column's independent max, since the
        // worst king pair for the pawnful bucket need not be the worst for the pawnless one).
        for (n_pf, n_pl) in [(8u64, 4u64)] {
            let chunk_pawnful = (NUM_KINGS_PAWNFUL as u64 + n_pf - 1) / n_pf;
            let chunk_pawnless = (NUM_KINGS_PAWNLESS as u64 + n_pl - 1) / n_pl;
            let bytes_pf = slots_per_king_idx_pawnful * chunk_pawnful * BYTES_PER_SLOT;
            let bytes_pl = slots_per_king_idx_pawnless * chunk_pawnless * BYTES_PER_SLOT;
            let mut max_total = 0f64;
            let mut sum_total = 0f64;
            for e in &entries {
                let mut pf: Vec<u64> = e.pawnful.iter().map(|&i| i as u64 / chunk_pawnful).collect();
                pf.sort_unstable(); pf.dedup();
                let mut pl: Vec<u64> = e.pawnless.iter().map(|&i| i as u64 / chunk_pawnless).collect();
                pl.sort_unstable(); pl.dedup();
                let total = bytes_pf as f64 * pf.len() as f64 + bytes_pl as f64 * pl.len() as f64 * 4.0;
                max_total = max_total.max(total);
                sum_total += total;
            }
            println!("MIXED n_pawnful={n_pf} n_pawnless={n_pl} | bytes/pf_chunk={:.1}MB bytes/pl_chunk={:.1}MB | avg_total={:.3}GB worst_total={:.3}GB",
                bytes_pf as f64 / 1e6, bytes_pl as f64 / 1e6, sum_total / entries.len() as f64 / 1e9, max_total / 1e9);
        }
    }

    // Walks every canonical position exactly like init() does, but only tracks the
    // largest index() seen per file instead of allocating, to size the full table
    // without needing enough RAM to actually hold it.
    #[test]
    fn max_index_per_file() {
        let mut max_index = vec![0usize; Pos::NUM_FILES];
        let mut count = vec![0usize; Pos::NUM_FILES];
        for last_moved in Pos::last_moved_iter() {
            for king in Pos::king_iter() {
                for p1 in Pos::p1_iter(king) {
                    for p2 in Pos::p2_iter(king, p1) {
                        for p3 in Pos::p3_iter(king, p1, p2) {
                            let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() || pos.in_check(pos.last_moved) {
                                continue;
                            }
                            for enpassant in Pos::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                let file = pos.file();
                                let index = pos.index();
                                if index > max_index[file] {
                                    max_index[file] = index;
                                }
                                count[file] += 1;
                            }
                        }
                    }
                }
            }
        }
        let total_slots: usize = max_index.iter().map(|&m| m + 1).sum();
        let total_bytes = total_slots * 2; // moves_left (u8) + status (Status, 1 byte)
        println!("total slots: {total_slots}, total bytes (moves_left+status): {:.2} GB", total_bytes as f64 / 1e9);

        let mut files_by_size: Vec<usize> = (0..Pos::NUM_FILES).filter(|&f| count[f] > 0).collect();
        files_by_size.sort_by_key(|&f| std::cmp::Reverse(max_index[f]));
        println!("--- largest 20 files ---");
        for &f in files_by_size.iter().take(20) {
            let density = count[f] as f64 / (max_index[f] + 1) as f64;
            println!("file {f}: max_index={} count={} density={:.3}", max_index[f], count[f], density);
        }
        println!("distinct nonempty files: {}", files_by_size.len());
    }

    // Breaks down exactly where file 187's (WN+BN+WP) density gap comes from:
    // p3_iter's own collision filter (vs. the naive P3_RANGE=48) vs. non-canonical
    // duplicates (harmless, counterpart fills the slot) vs. genuine in-check illegality.
    #[test]
    fn file_187_density_breakdown() {
        let mut combo_count = 0usize;
        let mut raw_p3_total = 0usize;
        let mut skipped_noncanonical = 0usize;
        let mut skipped_check = 0usize;
        let mut inserted = 0usize;
        for last_moved in Pos::last_moved_iter() {
            for king in Pos::king_iter() {
                for p1 in Pos::p1_iter(king).filter(|p| p.1 == Piece::WhiteKnight) {
                    for p2 in Pos::p2_iter(king, p1).flatten().filter(|p| p.1 == Piece::BlackKnight) {
                        combo_count += 1;
                        for p3 in Pos::p3_iter(king, p1, Some(p2)).flatten().filter(|p| p.1 == Piece::WhitePawn) {
                            raw_p3_total += 1;
                            let mut pos = Pos { last_moved, king, p1, p2: Some(p2), p3: Some(p3), enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() {
                                skipped_noncanonical += 1;
                                continue;
                            }
                            if pos.in_check(pos.last_moved) {
                                skipped_check += 1;
                                continue;
                            }
                            for enpassant in Pos::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                inserted += 1;
                            }
                        }
                    }
                }
            }
        }
        println!("combo_count (king,p1,p2 triples): {combo_count}");
        println!("raw_p3_total: {raw_p3_total}, avg p3 options per combo: {:.2} (naive P3_RANGE=48)", raw_p3_total as f64 / combo_count as f64);
        println!("skipped_noncanonical: {skipped_noncanonical}");
        println!("skipped_check: {skipped_check}");
        println!("inserted: {inserted}");
        println!("canonical+legal fraction of raw_p3_total: {:.3}", inserted as f64 / raw_p3_total as f64);
    }

    // Same breakdown for file 258 (WB+WN[white]+BP): all three pieces are distinct
    // (no same-type-opposite-colour value tie like file 187's WN/BN), to check whether
    // the 75% non-canonical rate was specific to that tie or a broader pattern.
    #[test]
    fn file_258_density_breakdown() {
        let mut combo_count = 0usize;
        let mut raw_p3_total = 0usize;
        let mut skipped_noncanonical = 0usize;
        let mut skipped_check = 0usize;
        let mut inserted = 0usize;
        for last_moved in Pos::last_moved_iter() {
            for king in Pos::king_iter() {
                for p1 in Pos::p1_iter(king).filter(|p| p.1 == Piece::WhiteBishop) {
                    for p2 in Pos::p2_iter(king, p1).flatten().filter(|p| p.1 == Piece::WhiteKnight) {
                        combo_count += 1;
                        for p3 in Pos::p3_iter(king, p1, Some(p2)).flatten().filter(|p| p.1 == Piece::BlackPawn) {
                            raw_p3_total += 1;
                            let mut pos = Pos { last_moved, king, p1, p2: Some(p2), p3: Some(p3), enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() {
                                skipped_noncanonical += 1;
                                continue;
                            }
                            if pos.in_check(pos.last_moved) {
                                skipped_check += 1;
                                continue;
                            }
                            for enpassant in Pos::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                inserted += 1;
                            }
                        }
                    }
                }
            }
        }
        println!("combo_count (king,p1,p2 triples): {combo_count}");
        println!("raw_p3_total: {raw_p3_total}, avg p3 options per combo: {:.2} (naive P3_RANGE=48)", raw_p3_total as f64 / combo_count as f64);
        println!("skipped_noncanonical: {skipped_noncanonical}");
        println!("skipped_check: {skipped_check}");
        println!("inserted: {inserted}");
        println!("canonical+legal fraction of raw_p3_total: {:.3}", inserted as f64 / raw_p3_total as f64);
    }
}

#[cfg(test)]
mod predecessor_coverage_check {
    use super::*;

    // Direct coverage check (not just canonicalization idempotency): for a sample of
    // king configurations with p1 = WhiteQueen (matching the crash's material context),
    // computes every predecessor and verifies it's actually producible via
    // p1_iter/p2_iter/p3_iter for its OWN (king,p1,p2) -- i.e. that init()'s enumeration
    // would really visit it, not just that it's already in canonical form. Samples a
    // handful of kings (including above-diagonal ones) instead of sweeping all 1806 to
    // stay fast.
    #[test]
    fn predecessors_are_reachable_via_iterators_p1_queen() {
        let sample_kings: Vec<[Square; 2]> = Pos::king_iter()
            .filter(|k| {
                let (r, f) = k[Colour::White].rank_file();
                r != f // skip on-diagonal, sample both above and below
            })
            .step_by(137) // spread across the 1806 range, cheap sample
            .take(30)
            .collect();
        eprintln!("sampling {} king configs", sample_kings.len());

        let mut checked: u64 = 0;
        let mut failures: u64 = 0;
        for last_moved in Pos::last_moved_iter() {
            for &king in &sample_kings {
                for p1 in Pos::p1_iter(king).filter(|p| p.1 == Piece::WhiteQueen) {
                    for p2 in Pos::p2_iter(king, p1) {
                        for p3 in Pos::p3_iter(king, p1, p2) {
                            let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() || pos.in_check(pos.last_moved) {
                                continue;
                            }
                            for enpassant in Pos::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                for pred in pos.clone().predecessors() {
                                    checked += 1;
                                    let p1_ok = Pos::p1_iter(pred.king).any(|p| p == pred.p1);
                                    let p2_ok = Pos::p2_iter(pred.king, pred.p1).any(|p| p == pred.p2);
                                    let p3_ok = Pos::p3_iter(pred.king, pred.p1, pred.p2).any(|p| p == pred.p3);
                                    if !p1_ok || !p2_ok || !p3_ok {
                                        failures += 1;
                                        eprintln!("UNREACHABLE: pred={pred:?} file={} index={} <- popped {pos:?} (p1_ok={p1_ok} p2_ok={p2_ok} p3_ok={p3_ok})",
                                            pred.file(), pred.index());
                                        if failures >= 20 { return; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("checked {checked} predecessors, {failures} failures");
        assert_eq!(failures, 0);
    }
}

#[cfg(test)]
mod predecessor_canonical_check {
    use super::*;

    // Frontier::pop() reads active_current.last(), and active_current is built in
    // ASCENDING file order -- so .last() actually drains the HIGHEST file index first.
    // The very first checkmates generate() pops (the ones that triggered the
    // "moves_left untouched" crash) therefore come from p1 = WhiteQueen (file 484..605),
    // not WhitePawn. make_revmove()'s last step is always make_canonical(), so any
    // predecessor that isn't already a fixed point of make_canonical() reveals a
    // canonicalization bug that would make init() (which only keeps already-canonical
    // raw combos) skip that exact slot -- without needing a full init() run to find it.
    #[test]
    fn predecessors_are_already_canonical_for_p1_queen() {
        let mut checked: u64 = 0;
        let mut failures: u64 = 0;
        for last_moved in Pos::last_moved_iter() {
            for king in Pos::king_iter() {
                for p1 in Pos::p1_iter(king).filter(|p| p.1 == Piece::WhiteQueen) {
                    for p2 in Pos::p2_iter(king, p1) {
                        for p3 in Pos::p3_iter(king, p1, p2) {
                            let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() || pos.in_check(pos.last_moved) {
                                continue;
                            }
                            for enpassant in Pos::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                for pred in pos.clone().predecessors() {
                                    checked += 1;
                                    let mut reclone = pred.clone();
                                    reclone.make_canonical();
                                    if reclone != pred {
                                        failures += 1;
                                        eprintln!("NOT ALREADY CANONICAL: pred={pred:?} file={} index={} <- popped {pos:?}",
                                            pred.file(), pred.index());
                                        eprintln!("  after re-canonicalizing: {reclone:?} file={} index={}",
                                            reclone.file(), reclone.index());
                                        if failures >= 20 { return; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("checked {checked} predecessors, {failures} failures");
        assert_eq!(failures, 0);
    }
}

#[cfg(test)]
mod iter_bench {
    use super::*;
    use std::hint::black_box;

    // Isolates just the p1_iter/p2_iter/p3_iter enumeration cost (no make_canonical,
    // no in_check, no count_distinct_canonical_successors mixed in), to measure the
    // iterator restructuring itself rather than the whole init() pipeline.
    #[test]
    fn bench_enumeration() {
        let start = Instant::now();
        let mut count: u64 = 0;
        for king in Pos::king_iter() {
            for p1 in Pos::p1_iter(king) {
                for p2 in Pos::p2_iter(king, p1) {
                    for p3 in Pos::p3_iter(king, p1, p2) {
                        count += black_box(1u64);
                        black_box((p1, p2, p3));
                    }
                }
            }
        }
        let elapsed = start.elapsed();
        println!("enumerated {count} (king,p1,p2,p3) tuples in {:.3}s ({:.0}/s)",
            elapsed.as_secs_f64(), count as f64 / elapsed.as_secs_f64());
    }

    #[test]
    fn bench_enumeration_with_enpassant() {
        let start = Instant::now();
        let mut count: u64 = 0;
        for last_moved in Pos::last_moved_iter() {
            for king in Pos::king_iter() {
                for p1 in Pos::p1_iter(king) {
                    for p2 in Pos::p2_iter(king, p1) {
                        for p3 in Pos::p3_iter(king, p1, p2) {
                            let pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            for ep in Pos::enpassant_iter(pos.clone()) {
                                count += black_box(1u64);
                                black_box(ep);
                            }
                        }
                    }
                }
            }
        }
        let elapsed = start.elapsed();
        println!("enumerated {count} (king,p1,p2,p3,ep) tuples in {:.3}s ({:.0}/s)",
            elapsed.as_secs_f64(), count as f64 / elapsed.as_secs_f64());
    }
}

#[cfg(test)]
mod reachability {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    // File-level reachability (via predecessors -- i.e. unpromotions from 3-piece positions
    // and uncaptures from 1/2-piece positions) doesn't depend on king position at all -- only
    // on piece types and whether a piece can reach an edge square, which is unaffected by
    // where the kings sit as long as they don't occupy an edge square themselves. So a single
    // off-edge king pair (rather than all 1806 from king_iter()) gives the same answer without
    // the ~1806x cost of sweeping every king position -- white king strictly below the diagonal
    // (rank < file) so it stays put under make_canonical, and off the diagonal so the "both
    // kings on diagonal" tie-break never triggers.
    //
    // A single sweep over p1 x p2 x p3 (each possibly None, per the iterators) covers every
    // piece count (1, 2, and 3 non-king pieces) in one pass, so one table serves both the
    // 3-piece unpromotion-target budget and the 1/2-piece uncapture-target budget.
    #[test]
    fn compute() {
        let king = [Square::c2, Square::f6];
        assert_ne!(KINGS_IDX_PAWNFUL[king[0]][king[1]], u16::MAX, "king pair must be valid");

        let mut reachable: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut checked = 0u64;
        for last_moved in Pos::last_moved_iter() {
            for p1 in Pos::p1_iter(king) {
                for p2 in Pos::p2_iter(king, p1) {
                    for p3 in Pos::p3_iter(king, p1, p2) {
                        let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                        let hash = pos.unique_hash();
                        pos.make_canonical();
                        if hash != pos.unique_hash() || pos.in_check(pos.last_moved) { continue; }
                        checked += 1;
                        let file = pos.file();
                        for pred in pos.clone().predecessors() {
                            let pf = pred.file();
                            if pf != file {
                                reachable.entry(file).or_default().insert(pf);
                            }
                        }
                    }
                }
            }
        }
        println!("checked {checked} canonical positions");
        let mut max_targets = 0;
        for (file, targets) in &reachable {
            max_targets = max_targets.max(targets.len());
            println!("file {file}: reachable -> {targets:?}");
        }
        println!("files with reachable targets: {}, max targets from one file: {max_targets}", reachable.len());
    }

    // Cross-check for Pos::piece_count_for_file(): its decode-from-file-index logic must
    // agree with directly counting how many of p1/p2/p3 are present, for every canonical
    // position this generator can actually produce.
    #[test]
    fn piece_count_for_file_matches_actual_positions() {
        let king = [Square::c2, Square::f6];
        for last_moved in Pos::last_moved_iter() {
            for p1 in Pos::p1_iter(king) {
                for p2 in Pos::p2_iter(king, p1) {
                    for p3 in Pos::p3_iter(king, p1, p2) {
                        let pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                        let expected = 1 + p2.is_some() as u8 + p3.is_some() as u8;
                        assert_eq!(Pos::piece_count_for_file(pos.file()), expected,
                            "file={} has_p2={} has_p3={}", pos.file(), p2.is_some(), p3.is_some());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod enpassant_crash_repro {
    use super::*;

    #[test]
    fn trace_enpassant_uncapture_predecessor() {
        let mut c = Pos {
            king: [Square::a1, Square::h8],
            p1: (Square::a8, Piece::WhiteQueen),
            p2: Some((Square::d6, Piece::WhitePawn)),
            p3: None,
            last_moved: Colour::White,
            enpassant: None,
        };
        c.make_canonical();
        let dump = |p: &Pos| format!(
            "fen={p:?} king=[{},{}] p1=({},{}) p2={:?} p3={:?} last_moved={} enpassant={:?}",
            p.king[0] as u8, p.king[1] as u8, p.p1.0 as u8, p.p1.1 as u8,
            p.p2.map(|(sq, pc)| (sq as u8, pc as u8)),
            p.p3.map(|(sq, pc)| (sq as u8, pc as u8)),
            p.last_moved as u8, p.enpassant.map(|sq| sq as u8));
        println!("c: {}", dump(&c));
        for pred in c.clone().predecessors() {
            println!("pred: {}", dump(&pred));
        }
    }

    // Forces the enpassant-candidate INJECTION path (not the direct en-passant-uncapture
    // path): C has a queen (so plenty of quiet reverse moves) plus two already-placed
    // opposite-colour pawns positioned so that, after undoing a quiet queen move, the
    // resulting predecessor's own pawns satisfy enpassant_possible(). The white king sits
    // on a file >= 4, so reaching canonical form requires a Horizontal reflection --
    // exactly the condition needed to expose the double-transform bug.
    #[test]
    fn injected_enpassant_predecessors_are_self_consistent() {
        let mut c = Pos {
            king: [Square::e1, Square::e8],
            p1: (Square::h1, Piece::WhiteQueen),
            p2: Some((Square::d5, Piece::WhitePawn)),
            p3: Some((Square::e5, Piece::BlackPawn)),
            last_moved: Colour::White,
            enpassant: None,
        };
        c.make_canonical();
        println!("c: {c:?} last_moved={} king=[{},{}]", c.last_moved as u8, c.king[0] as u8, c.king[1] as u8);

        let mut found_ep = 0;
        for pred in c.clone().predecessors() {
            if pred.enpassant.is_none() { continue; }
            found_ep += 1;
            println!("pred: {pred:?} last_moved={} enpassant={:?}", pred.last_moved as u8, pred.enpassant.map(|s| s as u8));

            // Re-derive what generate_revmovelist()'s ep branch would look for, and check
            // that piece actually exists and is the correct colour's pawn -- this is
            // exactly the invariant that was violated in the original crash.
            let ep = pred.enpassant.unwrap() as u8;
            let source = match pred.last_moved {
                Colour::White => ep + 8,
                Colour::Black => ep - 8,
            };
            let pawn = Piece::pawn(pred.last_moved);
            let pieces = [Some(pred.p1), pred.p2, pred.p3];
            let piece_at_source = pieces.into_iter().flatten().find(|(sq, _)| *sq as u8 == source);
            assert_eq!(piece_at_source.map(|(_, p)| p), Some(pawn),
                "ep={ep} source={source} should hold {pawn:?} but found {:?} in {pred:?}",
                piece_at_source.map(|(sq, p)| (sq as u8, p as u8)));
        }
        assert!(found_ep > 0, "test didn't actually exercise the injection path");
    }
}

// spillable::MB is scaled down under cfg(test), so these budgets (computed the same way
// as SourceBudget does internally) are small enough to force real spilling here.
#[cfg(test)]
mod frontier_integration {
    use super::*;

    fn junk_pos(seed: u32) -> Pos {
        Pos {
            king: [Square::c2, Square::f6],
            p1: (Square::from_u8((seed % 64) as u8), Piece::WhiteQueen),
            p2: None,
            p3: None,
            last_moved: if seed % 2 == 0 { Colour::White } else { Colour::Black },
            enpassant: None,
        }
    }

    #[test]
    fn routes_and_recovers_pushes_per_bucket_under_real_budgets() {
        let dir = std::env::temp_dir().join("bitchess_frontier_integration_test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut frontier = Frontier::new(&dir, Pos::NUM_FILES);

        // 132: 3-piece, one target (121). 131: 2-piece, several targets including one
        // low-piece (10, unlimited) and several 3-piece ones. 604: 1-piece, zero 3-piece
        // targets -- own file only.
        let sources = [132usize, 131, 604];
        let mut pushed_per_file: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
        let mut source_budget = SourceBudget::for_file(Pos::NUM_FILES);

        for &source in &sources {
            source_budget = SourceBudget::for_file(source);
            frontier.enter_source(&source_budget);

            let mut push_to = |frontier: &mut Frontier, file: usize, n: u32, pushed: &mut std::collections::HashMap<usize, u32>| {
                let budget = source_budget.budget_for_target(file);
                for i in 0..n {
                    frontier.push(file, junk_pos(i), Status::from_byte((i % 200) as u8), budget);
                }
                *pushed.entry(file).or_default() += n;
            };
            push_to(&mut frontier, source, 3000, &mut pushed_per_file);
            for target in three_piece_targets(source) {
                push_to(&mut frontier, target, 3000, &mut pushed_per_file);
            }
        }

        // at least one budgeted bucket must have actually spilled to disk -- otherwise
        // this test isn't exercising the thing it's meant to.
        let any_spilled = pushed_per_file.keys().any(|&f| frontier.next[f].spilled_bytes() > 0);
        assert!(any_spilled, "expected at least one bucket to spill under test-scaled budgets");

        assert!(frontier.advance(), "expected non-empty frontier after pushing");

        for (&file, &expected) in &pushed_per_file {
            let mut count = 0u32;
            while frontier.current[file].pop().is_some() { count += 1; }
            assert_eq!(count, expected, "file {file}: pushed {expected}, popped {count}");
        }
        assert!(!frontier.advance(), "expected frontier to be empty after draining everything pushed");
    }
}