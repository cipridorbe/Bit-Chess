use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use memmap2::MmapMut;

use crate::egtb::{NUM_KINGS_PAWNFUL, NUM_KINGS_PAWNLESS};

fn tetra(x: u64) -> u64 { x * (x + 1) * (x + 2) / 6 }

// Theoretical (upper-bound) slot count for a given file() value, mirroring index()'s
// branch structure exactly -- decodes file() = v1*121 + v2*11 + v3 (v in 0..4 = White
// P/N/B/R/Q, 5..9 = Black P/N/B/R/Q, 10 = None) back into the piece shape, without
// needing an actual Pos. This sizes the file's backing store; it need not be tight,
// since the on-disk file is only ever touched (and only consumes disk space) at the
// specific indices index() actually produces.
pub(crate) fn theoretical_file_slots(file_idx: usize) -> u64 {
    let v1 = file_idx / 121;
    let rem = file_idx % 121;
    let v2 = rem / 11;
    let v3 = rem % 11;
    if v1 >= 5 { return 0; } // p1 is always White; unreachable file slot

    let is_pawn = |v: usize| v == 0 || v == 5;
    let is_none = |v: usize| v == 10;
    let pf = NUM_KINGS_PAWNFUL as u64;
    let pl = NUM_KINGS_PAWNLESS as u64;

    if is_none(v2) {
        return if is_pawn(v1) { 2 * pf * 48 } else { 2 * pl * 62 };
    }
    if is_none(v3) {
        let (p1_pawn, p2_pawn) = (is_pawn(v1), is_pawn(v2));
        if p1_pawn && p2_pawn {
            return if v1 == v2 { 2 * pf * (47 * 48 / 2) } else { 2 * pf * (48 * 47 + 16) };
        } else if p2_pawn {
            return 2 * pf * (62 * 48);
        } else {
            return if v1 == v2 { 2 * pl * (61 * 62 / 2) } else { 2 * pl * (62 * 61) };
        }
    }
    let (p1_pawn, p2_pawn, p3_pawn) = (is_pawn(v1), is_pawn(v2), is_pawn(v3));

    if !p1_pawn && !p2_pawn && !p3_pawn {
        if v1 == v2 && v2 == v3 {
            return 2 * pl * tetra(60);
        } else if v1 == v2 {
            return 2 * pl * (61 * 62 / 2) * 60;
        } else if v2 == v3 {
            return 2 * pl * 62 * (60 * 61 / 2);
        } else {
            return 2 * pl * 62 * 61 * 60;
        }
    }
    if !p1_pawn && !p2_pawn && p3_pawn {
        return if v1 == v2 { 2 * pf * (61 * 62 / 2) * 48 } else { 2 * pf * 62 * 61 * 48 };
    }
    if !p1_pawn && p2_pawn && p3_pawn && v2 == v3 {
        return 2 * pf * (47 * 48 / 2) * 62;
    }
    if !p1_pawn && p2_pawn && p3_pawn {
        return 2 * pf * 62 * (48 * 47 + 16);
    }
    // p1_pawn: all three pawns (pawns are always a value-suffix)
    if v1 == v2 && v2 == v3 {
        return 2 * pf * tetra(46);
    }
    // "PPvP": p1==p2 (contiguous-identical invariant rules out any other split)
    2 * pf * ((47 * 48 / 2) * 46 + 46 * 32)
}

// One memory-mapped, OS-page-cached backing file per material file index. Reads and
// writes go straight through `mmap` -- the OS transparently pages data in from disk on
// first touch and writes dirty pages back under memory pressure; there is no explicit
// load/save step here.
pub(crate) struct PagedFile {
    mmap: MmapMut,
}

impl PagedFile {
    fn open_or_create(path: &Path, num_slots: u64) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        file.set_len(num_slots.max(1))?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self { mmap })
    }

    #[inline]
    pub(crate) fn flush(&self) -> std::io::Result<()> {
        self.mmap.flush()
    }
}

impl std::ops::Index<usize> for PagedFile {
    type Output = u8;
    #[inline]
    fn index(&self, i: usize) -> &u8 { &self.mmap[i] }
}

impl std::ops::IndexMut<usize> for PagedFile {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut u8 { &mut self.mmap[i] }
}

// Replaces `Files<T> = [Vec<T>; NUM_FILES]`. Each backing file is created and mapped
// lazily on first touch; which of its pages are actually resident in physical RAM at
// any moment is entirely the OS's concern from then on.
pub(crate) struct PagedFiles {
    files: Vec<Option<PagedFile>>,
    dir: PathBuf,
    prefix: &'static str,
}

impl PagedFiles {
    pub(crate) fn new(dir: PathBuf, prefix: &'static str, num_files: usize) -> Self {
        std::fs::create_dir_all(&dir).expect("create paged-file directory");
        Self { files: (0..num_files).map(|_| None).collect(), dir, prefix }
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, file: usize, index: usize) -> &mut u8 {
        if self.files[file].is_none() {
            let path = self.dir.join(format!("{}_{file}.bin", self.prefix));
            let slots = theoretical_file_slots(file);
            self.files[file] = Some(PagedFile::open_or_create(&path, slots).expect("mmap open"));
        }
        &mut self.files[file].as_mut().unwrap()[index]
    }

    pub(crate) fn flush_all(&self) -> std::io::Result<()> {
        for f in self.files.iter().flatten() {
            f.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cross-checked against the empirically measured max_index (+1) for these exact
    // files from the size_estimate tests earlier in this project's history.
    #[test]
    fn matches_measured_sizes() {
        assert_eq!(theoretical_file_slots(187), 655_708_032); // KNP v KN
        assert_eq!(theoretical_file_slots(253), 655_708_032); // KBNP v K
        assert_eq!(theoretical_file_slots(258), 655_708_032); // KBN v KP
    }
}
