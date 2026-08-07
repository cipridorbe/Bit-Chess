use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::egtb::threepiece::generator::Status;
use crate::egtb::threepiece::pos::Pos;
use crate::egtb::threepiece::reachable_files::three_piece_targets;
use crate::repr::{colour::Colour, piece::Piece, square::Square};

// Packed field by field rather than transmuted -- p2/p3's unused Option payload byte when
// None is genuinely uninitialized, so a whole-value transmute reads UB. Every byte here is
// something we assign ourselves instead.
const ENTRY_SIZE: u64 = 11;
const NONE_SQUARE: u8 = 255; // Square only uses 0..63

fn encode(pos: Pos, status: Status) -> [u8; 11] {
    let mut buf = [0u8; 11];
    buf[0] = pos.king[0] as u8;
    buf[1] = pos.king[1] as u8;
    buf[2] = pos.p1.0 as u8;
    buf[3] = pos.p1.1 as u8;
    match pos.p2 {
        Some((sq, piece)) => { buf[4] = sq as u8; buf[5] = piece as u8; }
        None => { buf[4] = NONE_SQUARE; buf[5] = 0; }
    }
    match pos.p3 {
        Some((sq, piece)) => { buf[6] = sq as u8; buf[7] = piece as u8; }
        None => { buf[6] = NONE_SQUARE; buf[7] = 0; }
    }
    buf[8] = pos.last_moved as u8;
    buf[9] = pos.enpassant.map_or(NONE_SQUARE, |sq| sq as u8);
    buf[10] = status.to_byte();
    buf
}

fn decode(buf: [u8; 11]) -> (Pos, Status) {
    let piece = |b: u8| unsafe { std::mem::transmute::<u8, Piece>(b) };
    let square_or_none = |b: u8| if b == NONE_SQUARE { None } else { Some(Square::from_u8(b)) };
    let pos = Pos {
        king: [Square::from_u8(buf[0]), Square::from_u8(buf[1])],
        p1: (Square::from_u8(buf[2]), piece(buf[3])),
        p2: square_or_none(buf[4]).map(|sq| (sq, piece(buf[5]))),
        p3: square_or_none(buf[6]).map(|sq| (sq, piece(buf[7]))),
        last_moved: if buf[8] == 0 { Colour::White } else { Colour::Black },
        enpassant: square_or_none(buf[9]),
    };
    (pos, Status::from_byte(buf[10]))
}

// One material file's worth of frontier entries: an in-memory tail (`hot`) plus, once a
// push's budget says `hot` would grow too large, whatever's been spilled to `path` on
// disk. The budget isn't a fixed property of the bucket -- it's supplied fresh on every
// push (see SourceBudget below), since the same target file's bucket can receive pushes
// from several different source files across one layer, each under a different
// allowance. That means spilled chunks can differ in size from each other, so their
// lengths are tracked explicitly (`chunk_lens`) rather than assumed uniform.
//
// Only ever used write-only (during a layer's push phase, while it's `Frontier::next`)
// then read-only (during the following layer's pop phase, after becoming
// `Frontier::current`) -- never both at once.
pub(crate) struct SpillableBucket {
    hot: Vec<(Pos, Status)>,
    path: PathBuf,
    file: Option<File>,
    spilled_bytes: u64,
    chunk_lens: Vec<u64>, // byte length of each spilled chunk, in write order
}

impl SpillableBucket {
    fn new(path: PathBuf) -> Self {
        Self { hot: Vec::new(), path, file: None, spilled_bytes: 0, chunk_lens: Vec::new() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.hot.is_empty() && self.chunk_lens.is_empty()
    }

    pub(crate) fn spilled_bytes(&self) -> u64 {
        self.spilled_bytes
    }

    // `budget` governs only *this* push -- it's the caller's job (see SourceBudget) to
    // pass the allowance that applies to whichever source file is currently draining.
    pub(crate) fn push(&mut self, pos: Pos, status: Status, budget: Option<u64>) {
        self.enforce_budget(budget);
        self.hot.push((pos, status));
    }

    // Spills now if `hot` is already over budget, without pushing anything. push() calls
    // this itself, but if a target stops getting pushed into, nothing else would ever
    // re-check it -- callers force this once a source is done contributing (see
    // Frontier::retire_source).
    pub(crate) fn enforce_budget(&mut self, budget: Option<u64>) {
        if let Some(budget) = budget {
            if (self.hot.len() as u64 + 1) * ENTRY_SIZE > budget {
                self.spill_hot();
            }
        }
    }

    pub(crate) fn pop(&mut self) -> Option<(Pos, Status)> {
        if let Some(item) = self.hot.pop() {
            return Some(item);
        }
        if let Some(chunk_bytes) = self.chunk_lens.pop() {
            self.load_chunk(chunk_bytes);
            return self.hot.pop();
        }
        None
    }

    fn file(&mut self) -> &mut File {
        if self.file.is_none() {
            self.file = Some(OpenOptions::new().read(true).write(true).create(true).truncate(true)
                .open(&self.path).expect("open spill file"));
        }
        self.file.as_mut().unwrap()
    }

    fn spill_hot(&mut self) {
        if self.hot.is_empty() { return; }
        let mut buf = Vec::with_capacity(self.hot.len() * ENTRY_SIZE as usize);
        for (pos, status) in self.hot.drain(..) {
            buf.extend_from_slice(&encode(pos, status));
        }
        let chunk_bytes = buf.len() as u64;
        let file = self.file();
        file.seek(SeekFrom::End(0)).expect("seek spill file end");
        file.write_all(&buf).expect("write spill chunk");
        self.chunk_lens.push(chunk_bytes);
        self.spilled_bytes += chunk_bytes;
    }

    fn load_chunk(&mut self, chunk_bytes: u64) {
        let new_len = self.spilled_bytes - chunk_bytes;
        let mut buf = vec![0u8; chunk_bytes as usize];
        let file = self.file();
        file.seek(SeekFrom::End(-(chunk_bytes as i64))).expect("seek spill chunk");
        file.read_exact(&mut buf).expect("read spill chunk");
        file.set_len(new_len).expect("truncate spill file");
        self.spilled_bytes = new_len;
        self.hot.extend(buf.chunks_exact(ENTRY_SIZE as usize).map(|c| decode(c.try_into().unwrap())));
    }
}

impl Drop for SpillableBucket {
    fn drop(&mut self) {
        drop(self.file.take());
        // spill files are scratch space, not part of the persisted tablebase -- fine to
        // leave a stray one behind if this doesn't succeed (e.g. never spilled at all).
        let _ = std::fs::remove_file(&self.path);
    }
}

// Scaled down under test so integration tests can trigger real spilling without pushing
// hundreds of MB of data -- same code paths, smaller numbers.
#[cfg(not(test))]
const MB: u64 = 1024 * 1024;
#[cfg(test)]
const MB: u64 = 100;
const OWN_FILE_BUDGET_3PIECE: u64 = 225 * MB;
const POOL_3PIECE_SOURCE: u64 = 25 * MB;
const POOL_LOWPIECE_SOURCE: u64 = 250 * MB;

// Budget for pushing a position directly into its own file's bucket (init()'s checkmate
// seeding, or a source's own-file share within SourceBudget below).
pub(crate) fn own_file_budget(file: usize) -> Option<u64> {
    (Pos::piece_count_for_file(file) == 3).then_some(OWN_FILE_BUDGET_3PIECE)
}

// The push budget that applies while `file` is the source currently being drained:
// unlimited for a target with piece_count <= 2 (own or reachable, always negligible);
// otherwise `file`'s own-file allowance if target == file (225MB for a 3-piece source,
// unlimited for a 1/2-piece one -- too small to matter); otherwise an even share of
// file's pool (25MB for a 3-piece source, 250MB for a 1/2-piece one) split across
// however many 3-piece files `file` can reach.
//
// Recomputed only when the draining source file changes, not per push -- Frontier::pop()
// drains one file's `current` bucket completely before moving to the next, so the
// source stays constant across many consecutive pushes.
pub(crate) struct SourceBudget {
    file: usize,
    own_budget: Option<u64>,
    pool_share: Option<u64>,
}

impl SourceBudget {
    pub(crate) fn for_file(file: usize) -> Self {
        let pc = Pos::piece_count_for_file(file);
        let n3 = three_piece_targets(file).count();
        let pool_share = (n3 > 0).then(|| {
            let pool = if pc == 3 { POOL_3PIECE_SOURCE } else { POOL_LOWPIECE_SOURCE };
            pool / n3 as u64
        });
        Self { file, own_budget: own_file_budget(file), pool_share }
    }

    pub(crate) fn file(&self) -> usize {
        self.file
    }

    pub(crate) fn budget_for_target(&self, target: usize) -> Option<u64> {
        if Pos::piece_count_for_file(target) <= 2 {
            return None;
        }
        if target == self.file { self.own_budget } else { self.pool_share }
    }
}

pub(crate) fn new_buckets(dir: &std::path::Path, label: &str, num_files: usize) -> Vec<SpillableBucket> {
    std::fs::create_dir_all(dir).expect("create frontier spill directory");
    (0..num_files).map(|file| SpillableBucket::new(dir.join(format!("frontier_{label}_{file}.bin")))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pos(seed: u8) -> Pos {
        Pos {
            king: [Square::from_u8(seed % 64), Square::from_u8((seed + 1) % 64)],
            p1: (Square::from_u8((seed + 2) % 64), Piece::WhiteQueen),
            p2: if seed % 2 == 0 { Some((Square::from_u8((seed + 3) % 64), Piece::BlackKnight)) } else { None },
            p3: None,
            last_moved: if seed % 3 == 0 { Colour::Black } else { Colour::White },
            enpassant: if seed % 5 == 0 { Some(Square::from_u8((seed + 4) % 64)) } else { None },
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        // Square/Piece/Colour/Status don't implement Debug, so compare via the raw bytes
        // (which do -- [u8; 11] implements Debug/Eq) rather than field-by-field assert_eq!.
        for seed in 0..=255u8 {
            let pos = sample_pos(seed);
            let status = Status::from_byte(seed);
            let original_bytes = encode(pos.clone(), status);
            let (decoded_pos, decoded_status) = decode(original_bytes);
            let roundtrip_bytes = encode(decoded_pos, decoded_status);
            assert_eq!(roundtrip_bytes, original_bytes, "seed={seed}");
        }
    }

    #[test]
    fn unlimited_budget_never_spills() {
        let dir = std::env::temp_dir().join("bitchess_spillable_test_unlimited");
        std::fs::create_dir_all(&dir).unwrap();
        let mut bucket = SpillableBucket::new(dir.join("test.bin"));
        for seed in 0..1000u16 {
            bucket.push(sample_pos((seed % 256) as u8), Status::from_byte((seed % 256) as u8), None);
        }
        assert_eq!(bucket.spilled_bytes, 0);
        let mut count = 0;
        while bucket.pop().is_some() { count += 1; }
        assert_eq!(count, 1000);
    }

    #[test]
    fn budgeted_push_spills_and_recovers_everything() {
        let dir = std::env::temp_dir().join("bitchess_spillable_test_budgeted");
        std::fs::create_dir_all(&dir).unwrap();
        // tiny budget so spilling actually triggers within a small test
        let budget = Some(5 * ENTRY_SIZE);
        let mut bucket = SpillableBucket::new(dir.join("test2.bin"));
        let n = 10_000u32;
        let mut expected: Vec<(Pos, Status)> = Vec::new();
        for seed in 0..n {
            let pos = sample_pos((seed % 256) as u8);
            let status = Status::from_byte((seed % 256) as u8);
            expected.push((pos.clone(), status));
            bucket.push(pos, status, budget);
        }
        assert!(bucket.spilled_bytes > 0, "expected spilling to have occurred");

        let mut popped = Vec::new();
        while let Some(item) = bucket.pop() { popped.push(item); }
        assert_eq!(popped.len(), expected.len());
        // pop order isn't guaranteed to match push order (chunk-based), so compare as
        // multisets via the raw encoded bytes as a sort key (Square/Piece/Status don't
        // implement Ord themselves)
        let key = |p: &(Pos, Status)| encode(p.0.clone(), p.1);
        expected.sort_by_key(key);
        popped.sort_by_key(key);
        for (e, p) in expected.iter().zip(popped.iter()) {
            assert_eq!(key(e), key(p));
        }
    }

    #[test]
    fn budgeted_push_can_vary_across_calls() {
        // simulates several different source files pushing into the same target bucket
        // across one layer, each under its own budget -- must not corrupt data even
        // though chunk sizes differ from call to call.
        let dir = std::env::temp_dir().join("bitchess_spillable_test_varying");
        std::fs::create_dir_all(&dir).unwrap();
        let mut bucket = SpillableBucket::new(dir.join("test3.bin"));
        let budgets = [Some(3 * ENTRY_SIZE), Some(37 * ENTRY_SIZE), None, Some(11 * ENTRY_SIZE)];
        let mut expected: Vec<(Pos, Status)> = Vec::new();
        let mut seed = 0u8;
        for &budget in budgets.iter() {
            for _ in 0..50u32 {
                let pos = sample_pos(seed);
                let status = Status::from_byte(seed);
                expected.push((pos.clone(), status));
                bucket.push(pos, status, budget);
                seed = seed.wrapping_add(1);
            }
        }
        let mut popped = Vec::new();
        while let Some(item) = bucket.pop() { popped.push(item); }
        assert_eq!(popped.len(), expected.len());
        let key = |p: &(Pos, Status)| encode(p.0.clone(), p.1);
        expected.sort_by_key(key);
        popped.sort_by_key(key);
        for (e, p) in expected.iter().zip(popped.iter()) {
            assert_eq!(key(e), key(p));
        }
    }

    #[test]
    fn source_budget_matches_spec() {
        // file 131 is 2-piece with 14 3-piece uncapture targets + 1 (file 10, itself
        // 2-piece) low-piece target -- own file unlimited, low-piece target unlimited,
        // 3-piece targets share the 250MB low-piece-source pool across 14 files.
        let sb = SourceBudget::for_file(131);
        assert_eq!(sb.budget_for_target(131), None); // own file, low-piece source
        assert_eq!(sb.budget_for_target(10), None);  // reachable but itself low-piece
        assert_eq!(sb.budget_for_target(121), Some(250 * MB / 14)); // 3-piece target

        // file 132 is 3-piece with exactly one target (121) -- own file 225MB, that one
        // target shares the 25MB 3-piece-source pool alone.
        let sb = SourceBudget::for_file(132);
        assert_eq!(sb.budget_for_target(132), Some(225 * MB));
        assert_eq!(sb.budget_for_target(121), Some(25 * MB));

        // file 604 (lone queen) has zero 3-piece targets at all -- pool_share unused.
        let sb = SourceBudget::for_file(604);
        assert_eq!(sb.budget_for_target(604), None);
        assert_eq!(sb.budget_for_target(120), None); // 1-piece target, unlimited
    }
}
