// =====================================================================================
// Compression research notes (kept here so findings survive across sessions)
// =====================================================================================
//
// GOAL: the raw 4-piece EGTB started at ~171.27MB. Windows Explorer's "compress to 7z"
// gets it to ~14.85MB in ~2min. The goal has been to match or beat that with a
// specialized/understood scheme (not necessarily hand-rolled -- a real backend counts).
// All numbers below use unknowns-converted-to-draws (Status(0)), as always.
//
// -------------------------------------------------------------------------------------
// 1) THE index() REDESIGN (this was a real, shipped change, not just a diagnostic)
// -------------------------------------------------------------------------------------
// Pos::index() was redesigned from "[kings][p1][p2][side]" to "[side][kings][p1][p2]"
// with coordinate compaction: p1's square range excludes squares already occupied by
// the kings (64 -> 62 for non-pawns; pawns stay at 48, since kings aren't guaranteed to
// land in ranks 1-6 so nothing can safely be subtracted), and p2's range additionally
// excludes p1's square when that's always safe to do (non-pawn p2 can always subtract
// kings+p1 -> 61 of 64, or 47 of 48 if p1 is also a pawn; a pawn p2 behind a non-pawn p1
// can't safely subtract p1, since p1 isn't guaranteed to be in the pawn zone -- stays at
// 48). The en-passant sparse tail (extra 16 slots per side, was 32 shared) moved inside
// each side's own partition to match side being outermost now.
//
// Effect: raw file 171.27MB -> 159.38MB (~6.9% smaller), confirmed by regenerating from
// scratch and passing test_syzygy_comparison (no collisions, no correctness regression).
// Compression got *equal or slightly better* too, not worse:
//   zstd level 19, current order, single stream:  17.81MB (old layout) -> 17.71MB (new)
// So this was a clean win on every axis. One side effect: the `reorder_side_outer`
// diagnostic (even/odd interleave split), which used to *help* zstd on the old
// side-innermost layout, now *hurts* (19.79MB) because the new index() already puts
// side outermost -- re-applying that reorder just scrambles an already-good layout.
// Don't reach for it post-redesign; it's now stale.
//
// -------------------------------------------------------------------------------------
// 2) BASELINE STATS on the current (post-redesign) tablebase, 167,118,196 bytes (159.38MB)
// -------------------------------------------------------------------------------------
// Overall distribution:
//   Zeros:   79,230,690 (47.41%)   Wins: 43,826,611 (26.22%)   Losses: 44,060,895 (26.37%)
// Top non-zero |value| buckets are roughly flat/wide (typical DTM spread), e.g. top 5:
//   -19: 2.36%   18: 2.32%   16: 2.30%   -17: 2.28%   -15: 2.22%   (each roughly ~2%, no
//   single dominant non-zero value -- the only real skew in the byte distribution is
//   "zero vs everything else", not within the non-zero values themselves).
//
// Simple/baseline size estimates:
//   Raw:                                    159.38 MB  (1.00x)
//   RLE (count,val) pairs:                  120.67 MB  (1.32x)
//   Zero-RLE (only zero runs collapsed):    115.30 MB  (1.38x)
//   Sparse (idx+val) encoding:              419.08 MB  (0.38x, i.e. much worse -- too
//                                            many non-zero entries for sparse to make sense)
//   Global order-0 (single-byte) entropy *:  75.70 MB  (2.11x)
//   WDL-only entropy * (3-way win/draw/loss over all entries): 30.36 MB
//   DTZ-only entropy * (magnitude, non-zero entries only):     55.82 MB
//   WDL+DTZ combined *:                      86.18 MB  (1.85x)
//   Global order-1 (byte-pair) entropy *:    58.58 MB  (2.72x; "2x single" would be 151.40MB,
//                                             so adjacent values are meaningfully correlated)
//   Delta entropy * (vs prev entry, per file): 55.48 MB (2.87x)
//   Stride-2 entropy * (vs same pos, other side): 60.59 MB (2.63x)
//   Per-file entropy sum * (own distribution per file): 61.09 MB (2.61x; beats the 75.70MB
//                                             global estimate -- confirms per-file/regional
//                                             models matter, motivating everything below)
// (* = theoretical minimum with an ideal Huffman/arithmetic coder on top; no such coder
// is actually implemented for these estimates, they're information-theoretic lower bounds.)
//
// Block-adaptive single-byte entropy sweep (independent frequency model per block,
// block never crosses a file boundary; "reorder" = reorder_side_outer, now stale/harmful
// post-redesign so only "current order" numbers matter going forward):
//   block   flat/current   per-file/current
//     256      45.08 MB        45.07 MB
//     512      43.96 MB        43.95 MB
//    1024      43.67 MB        43.66 MB   <- best plain block-adaptive result
//    4096      44.43 MB        44.42 MB
//   65536      47.83 MB        47.80 MB
// (flat vs per-file barely differ -- file boundaries rarely land mid-block at these
// sizes relative to file sizes, so this axis doesn't matter much either way.)
//
// LZ match coverage (fraction of bytes inside a >=8-byte repeat, a rough proxy for what
// dictionary/LZ-style matching -- i.e. what 7z/zstd actually do -- could exploit):
//   current order: 89.67%   reordered: 89.80%   (both very high; most of the file *is*
//   redundant in an LZ sense, our hand-rolled schemes just can't reach most of it)
//
// Real backends (ground truth):
//   zstd level 19, 128MiB window, long-distance-matching, current order, single stream: 17.71MB (9.00x)
//   zstd level 19, ..., per-file (separate streams per file, no cross-file dictionary):  17.84MB (8.93x)
//   zstd on reorder_side_outer'd data (now counterproductive post-redesign):             19.79MB
//   7z (Windows Explorer default, ~2min):                                                14.85MB (~10.73x)
//
// -------------------------------------------------------------------------------------
// 3) N-GRAM / RUN-TOKEN STRUCTURAL FINDINGS
// -------------------------------------------------------------------------------------
// Global top-10 n-grams by length (data.windows(n), i.e. OVERLAPPING/unaligned counting
// -- every starting offset counted separately). Values shown as i8 (Status encoding).
//   len=2:  [0,0]=68,224,971; then uniform pairs -13:1,843,603  -15:1,812,567  -17:1,728,032
//           -11:1,721,719  -19:1,706,586  -21:1,596,013  16:1,595,411  18:1,570,566  14:1,521,843
//   len=4:  [0,0,0,0]=60,339,312; then uniform quads -13:720,188 -11:686,631 -15:657,834
//           -9:642,439  8:548,679  10:535,810  12:527,683  -7:524,691  6:522,057
//   len=8:  [0]*8=52,009,756; then uniform octets -13:160,702 -11:151,753 -9:151,416
//           8:146,402  6:145,659  -15:143,266  4:132,332  10:123,175  -7:118,144
//   len=16: [0]*16=43,764,379; uniform runs 4:66,670 6:60,392 2:54,392 -9:49,827
//           -7:45,556 8:44,805; PLUS first "zero-but-one" entries appear:
//           [0*15,18]=49,529  [0*15,20]=49,137  [0*15,22]=46,662
//   len=32: [0]*32=31,686,314; top 9 runner-ups are ALL "zero-but-one" (one non-zero
//           value at a sliding position within an otherwise-zero window), e.g.
//           [0*31,-31]=29,034  [-31,0*31]=28,230  [0*31,-29]=27,681  ... down to 23,357.
//           No uniform nonzero run makes top-10 anymore.
//   len=64: [0]*64=23,366,469; top 9 runner-ups are all "zero-but-one" too, but now tiny:
//           best is only 5,750 occurrences (~0.003% of 167M windows).
// CONCLUSION from this table: past ~16-24 bytes, no non-zero repeated pattern is common
// enough to matter; the only "second tier" shape is a long zero run with one arbitrary-
// position exception, and even that decays to near-nothing by 64 bytes.
//
// BUT: the overlapping counts above are misleading for run-length questions, because a
// single long run gets recounted at every offset and every length. Non-overlapping /
// greedy-largest-first decomposition (nonoverlapping_zero_run_counts: for each maximal
// zero run of length L, greedily take as many of the largest available length as fit,
// then recurse on the remainder -- e.g. a 100-zero run becomes one 64 + one 32 + one 4,
// matching a run's binary representation when lengths are powers of two) gives a very
// different, truer picture:
//   length | non-overlapping count | bytes accounted (count * length)
//      2   |     2,911,082          |    5,822,164
//      4   |     1,983,263          |    7,933,052
//      8   |       688,718          |    5,509,744
//     16   |       499,269          |    7,988,304
//     32   |       524,737          |   16,791,584
//     64   |       413,968          |   26,493,952   <- single largest byte-coverage bucket
// Sum of bytes accounted = 70,538,800 vs total zero bytes = 79,230,690; the ~8.69M gap is
// exactly the zero bytes living in runs shorter than length 2 (isolated single zeros) or
// left as an unclaimable length-1 remainder -- a clean, fully consistent partition.
// CONCLUSION: byte-coverage *increases* with run length up to at least 64 -- long zero
// runs are genuinely common, not a sliding-window counting artifact. Combined with the
// "no non-zero pattern past ~16-24 bytes" finding above, the data's redundancy reduces
// to almost exactly two primitives: short repeated-value runs (useful only up to ~8-16
// bytes) and long zero runs with sparse, arbitrarily-placed single-value exceptions.
// Fixed-offset n-gram dictionaries are a structurally poor fit for the second primitive
// (every exception position needs its own dictionary entry, see alignment discussion
// below); unbounded-offset LZ-style matching handles it for free, which is the core
// reason zstd/7z beat every hand-rolled scheme here by ~2-3x.
//
// Alignment caveat worth remembering: top_ngrams/top_ngrams_with_counts use unaligned
// sliding windows (every offset), which is correct for "how often does this substring
// occur" but those dictionaries then get *consumed* by tokenize_and_entropy, which walks
// greedily left-to-right from each block's start -- so a pattern frequent at arbitrary
// offsets in the corpus won't necessarily land on a token boundary during encoding. The
// dictionary-selection step is measuring something slightly more optimistic than what
// the encoding step can actually exploit. (pair_run_token_entropy_size, by contrast,
// uses aligned non-overlapping chunks(2) throughout, so no such mismatch there.)
//
// -------------------------------------------------------------------------------------
// 4) HAND-ROLLED SCHEMES TRIED, IN ORDER, WITH RESULTS (all "per file" = one independent
//    model per file, matching Syzygy's real per-table granularity; "block-adaptive" =
//    independent frequency table per fixed-size block, never crossing a file boundary)
// -------------------------------------------------------------------------------------
// a) Single-byte same-value-run tokens, Huffman over (value, run_length) pairs, global
//    per-file model, run length capped at max_run (run_token_entropy_size):
//      max_run=16: 46.51MB   64: 45.63MB   255: 45.48MB   1024: 45.45MB (best)
//    Worse than plain block-adaptive (43.67MB) -- a global (non-block-adaptive) model
//    loses more from lacking local specialization than it gains from run-merging.
//
// b) Same idea but the unit is a non-overlapping 2-byte pair, not 1 byte
//    (pair_run_token_entropy_size) -- catches alternating 2-value patterns that (a)
//    completely misses (every alternating run has single-byte run-length 1):
//      max_run=16: 42.64MB   64: 42.26MB   255: 42.20MB   1024: 42.19MB (best)
//    Meaningfully better than (a), and edges out plain block-adaptive slightly.
//
// c) Block-adaptive, alphabet augmented with a flat global (per-file) dictionary: top-15
//    most frequent n-grams for each of lengths {2,4,8} (45 extra symbols total), greedy
//    longest-match-first tokenization, never crossing a block boundary
//    (dict_block_entropy_size_per_file):
//      block=1024: plain 43.66MB -> dict 41.35MB
//      block=2048: plain 44.10MB -> dict 40.65MB
//      block=4096: plain 44.42MB -> dict 40.27MB (best)
//    Helps substantially, and shifts the optimal block size up (dictionary tokens make
//    larger blocks pay off, unlike the plain case where 1024 was best).
//
// d) Three-tier hierarchical dictionary (hierarchical_dict_block_entropy_size_per_file):
//    global tier = top-5 n-grams for each of {2,4,8} + explicit zero-run(16) and
//    zero-run(32) tokens, shared across the whole file, cost paid once; regional tier =
//    top-10 n-grams for each of {2,4,8} (excluding anything the global tier already has),
//    shared across each superblock of block_size*8 bytes, cost paid once per superblock;
//    then per-block Huffman entropy over the combined alphabet:
//      block=1024 (region=8192):   43.38MB  -- worse than (c) here; regional dict
//                                              overhead dominates at small blocks
//      block=2048 (region=16384):  40.95MB
//      block=4096 (region=32768):  39.68MB
//      block=8192 (region=65536):  39.34MB (best)
//    Beats (c) once blocks are large enough to amortize the regional dictionary cost.
//
// e) Global top-10 n-grams for lengths {2,4,8,16,32,64}, printed with counts, is what
//    produced the structural findings in section 3 above (global_top_ngrams_diagnostic).
//
// f) Block-adaptive, base literal unit = non-overlapping 2-byte pair (not a mined
//    dictionary -- fixed structure), PLUS explicit zero-run tokens (checked only when
//    the current byte is 0, cheap short-circuit; longest-match-first; never crosses a
//    block boundary) (pair_block_entropy_size_per_file). This is a direct application of
//    the section-3 finding: model the two real primitives (short value runs via the pair
//    unit, long zero runs via explicit tokens) instead of mining arbitrary n-grams.
//    f1) zero-run tokens = {8, 64} only:
//      block=1024: 39.86MB  2048: 38.71MB  4096: 37.82MB  8192: 37.48MB (best)
//    f2) zero-run tokens = every power of two up to 512 ({2,4,...,512}):
//      block=1024: 39.61MB  2048: 38.30MB  4096: 37.31MB  8192: 36.91MB (best, peak)
//                  16384: 37.17MB  32768: 37.39MB   (gets WORSE past 8192 -- larger
//                  blocks see more distinct literal pair values, which dilutes the
//                  zero-run token savings faster than it amortizes further; 8192 is a
//                  genuine peak, not "still climbing, stopped early")
//    f3) same as f2 plus hand-picked extra lengths {6,10,12,14,20,26,48,80,100,150,200}:
//      block=1024: 39.25MB  2048: 37.96MB  4096: 36.92MB  8192: 36.46MB (best overall)
//                  16384: 36.70MB  32768: 36.90MB
//    This is the best hand-rolled result found so far: 36.46MB at block=8192. The extra
//    hand-picked lengths in f3 give a real but small win (~1.2%) over the pure
//    power-of-two ladder in f2 -- confirms those specific lengths are genuinely a bit
//    more common than a binary decomposition alone captures, but this is deep into
//    diminishing returns (each refinement a-through-f bought progressively less: block-
//    adaptive was the big first win, pair-units another solid step, dictionaries another
//    percent or two each, explicit zero-run tokens the biggest remaining win, hand-tuned
//    extra lengths a small polish on top of that).
//
// -------------------------------------------------------------------------------------
// 5) BOTTOM LINE
// -------------------------------------------------------------------------------------
// Every hand-rolled scheme (a-f above) converges into a 36-46MB band. Best hand-rolled:
// 36.46MB (f3). Real zstd: 17.71MB. Real 7z: 14.85MB. This ~2.4x (zstd) to ~2.9x (7z)
// gap looks like a genuine structural ceiling for "fixed small alphabet + per-block/
// per-file Huffman coding": none of these schemes do real adaptive, unbounded-offset
// matching, which is exactly what section 3 showed the data actually needs (sparse,
// arbitrary-position exceptions in long zero runs -- a natural fit for LZ, a bad fit for
// any fixed dictionary or token-length list, no matter how well tuned).
//
// RECOMMENDED NEXT STEP (not yet tried): wire up LZMA directly via the `xz2`/`liblzma`
// crate -- it's literally the algorithm 7z uses under the hood, so it should get close
// to 14.85MB directly with far less effort than continuing to hand-tune entropy/
// dictionary schemes, and the index()/reindexing work above can still be layered under it.
// =====================================================================================

use super::pos::{load_tablebase, save_tablebase, Pos, Status};

pub fn load_replacing_unknowns(path: &str) -> std::io::Result<[Vec<Status>; 100]> {
    let mut tb = load_tablebase(path)?;
    for file in tb.iter_mut() {
        for s in file.iter_mut() {
            if *s == Status::UNKOWN {
                *s = Status(0);
            }
        }
    }
    Ok(tb)
}

fn flatten(tb: &[Vec<Status>; 100]) -> Vec<u8> {
    tb.iter().flat_map(|f| f.iter().map(|s| s.0 as u8)).collect()
}

// Same as flatten, but every file index in `sparsify` is serialized as a sparse
// (u24 index, i8 value) list of its non-zero entries instead of a dense byte-per-entry
// array -- real bytes, not just a size estimate, so this can be fed to a real
// compressor (zstd) to see the actual effect of switching representation.
fn flatten_hybrid(tb: &[Vec<Status>; 100], sparsify: &[usize]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, file) in tb.iter().enumerate() {
        if sparsify.contains(&i) {
            for (idx, s) in file.iter().enumerate() {
                if s.0 == 0 { continue; }
                let idx = idx as u32;
                out.push((idx & 0xff) as u8);
                out.push(((idx >> 8) & 0xff) as u8);
                out.push(((idx >> 16) & 0xff) as u8);
                out.push(s.0 as u8);
            }
        } else {
            out.extend(file.iter().map(|s| s.0 as u8));
        }
    }
    out
}

// STALE POST-INDEX-REDESIGN: this reordered a file so the side-to-move bit (the LSB of
// the index, interleaved as [..., side0, side1, side0, side1, ...]) became the outermost
// dimension -- a mechanical, index-agnostic approximation of "[side][kings][p1][p2]".
// That was useful when Pos::index() had side innermost; it measurably helped zstd back
// then. Pos::index() was since redesigned to put side outermost natively (see the
// research notes at the top of this file), so calling this now just scrambles an
// already-good layout and measurably *hurts* real compression (zstd: 17.71MB current
// order vs 19.79MB reordered). Kept only for historical comparison in the diagnostics
// below; don't use it as an actual improvement post-redesign.
fn reorder_side_outer(file: &[Status]) -> Vec<u8> {
    let mut out = Vec::with_capacity(file.len());
    out.extend(file.iter().step_by(2).map(|s| s.0 as u8));
    out.extend(file.iter().skip(1).step_by(2).map(|s| s.0 as u8));
    out
}

fn flatten_side_outer(tb: &[Vec<Status>; 100]) -> Vec<u8> {
    tb.iter().flat_map(|f| reorder_side_outer(f)).collect()
}

// Approximate LZ77-style match coverage: hash every MIN_MATCH-byte window into a
// fixed-size table storing only the most recent position with that hash (like a fast/
// greedy LZ matcher, e.g. lz4's hash chains simplified to depth 1). When a later window
// hashes the same and verifies, extend the match and count those bytes as "covered".
// Reports the fraction of the buffer covered by matches >= MIN_MATCH -- redundancy that
// only LZ-style dictionary matching (not per-byte entropy coding) can exploit. This is
// the diagnostic that should explain (or fail to explain) 7z's outsized advantage over
// every entropy-based estimate above.
fn lz_match_fraction(data: &[u8]) -> f64 {
    const MIN_MATCH: usize = 8;
    const HASH_BITS: u32 = 22;
    const HASH_SIZE: usize = 1 << HASH_BITS;
    let n = data.len();
    if n < MIN_MATCH { return 0.0; }
    let mut table = vec![u32::MAX; HASH_SIZE];
    let hash_at = |i: usize| -> usize {
        let mut h: u64 = 0;
        for k in 0..MIN_MATCH {
            h = h.wrapping_mul(2654435761).wrapping_add(data[i + k] as u64);
        }
        (h as usize) & (HASH_SIZE - 1)
    };
    let mut covered = 0usize;
    let mut i = 0usize;
    while i + MIN_MATCH <= n {
        let h = hash_at(i);
        let cand = table[h];
        table[h] = i as u32;
        if cand != u32::MAX {
            let c = cand as usize;
            if c < i && data[c..c + MIN_MATCH] == data[i..i + MIN_MATCH] {
                let mut len = MIN_MATCH;
                while i + len < n && c + len < i && data[c + len] == data[i + len] {
                    len += 1;
                }
                covered += len;
                i += len;
                continue;
            }
        }
        i += 1;
    }
    covered as f64 / n as f64
}

fn byte_counts(data: &[u8]) -> [u64; 256] {
    let mut counts = [0u64; 256];
    for &b in data { counts[b as usize] += 1; }
    counts
}

fn print_distribution(data: &[u8]) {
    let counts = byte_counts(data);
    let total = data.len() as f64;

    println!("Total:   {} bytes  ({:.2} MB)", data.len(), data.len() as f64 / 1_048_576.0);
    println!("Zeros:   {} ({:.2}%)", counts[0], counts[0] as f64 / total * 100.0);

    let wins: u64    = counts[1..=0x7e].iter().sum();
    let losses: u64  = counts[0x80..].iter().sum();
    let unknowns_left = counts[0x7f];
    println!("Wins:    {} ({:.2}%)", wins,   wins   as f64 / total * 100.0);
    println!("Losses:  {} ({:.2}%)", losses, losses as f64 / total * 100.0);
    if unknowns_left > 0 {
        println!("WARNING: {} unreplaced unknowns (0x7f)", unknowns_left);
    }

    println!("\nTop 15 non-zero values (as i8):");
    let mut nz: Vec<(i8, u64)> = counts.iter().enumerate().skip(1)
        .filter(|(_, &c)| c > 0)
        .map(|(i, &c)| (i as u8 as i8, c))
        .collect();
    nz.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
    for (val, count) in nz.iter().take(15) {
        println!("  {:5}: {:>12}  ({:.4}%)", val, count, *count as f64 / total * 100.0);
    }
}

// Shannon entropy lower bound — what a perfect Huffman/arithmetic coder achieves.
fn entropy_size(data: &[u8]) -> usize {
    let counts = byte_counts(data);
    let total = data.len() as f64;
    let bits: f64 = counts.iter()
        .filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum();
    (bits / 8.0).ceil() as usize
}

// Separate WDL (draw/win/loss) from DTZ magnitude, then compute entropy of each.
// WDL has only 3 symbols → ~1.5 bits/entry.
// DTZ has values 1..=23 with a tighter distribution → ~3–4 bits/entry.
fn wdl_dtm_entropy_sizes(data: &[u8]) -> (usize, usize) {
    let (mut draw, mut win, mut loss) = (0u64, 0u64, 0u64);
    let mut dtm_counts = [0u64; 128];
    for &b in data {
        let v = b as i8;
        if v == 0 { draw += 1; }
        else if v > 0 { win += 1; dtm_counts[v as usize] += 1; }
        else          { loss += 1; dtm_counts[(-v) as usize] += 1; }
    }
    let n = data.len() as f64;
    let wdl_bits: f64 = [draw, win, loss].iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / n).log2()))
        .sum();
    let dtm_total: f64 = dtm_counts.iter().sum::<u64>() as f64;
    let dtm_bits: f64 = dtm_counts.iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / dtm_total).log2()))
        .sum();
    (
        (wdl_bits / 8.0).ceil() as usize,
        (dtm_bits / 8.0).ceil() as usize,
    )
}

// Simple (count u8, value u8) RLE.
fn rle_size(data: &[u8]) -> usize {
    if data.is_empty() { return 0; }
    let mut size = 0usize;
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == byte { run += 1; }
        size += ((run + 254) / 255) * 2;
        i += run;
    }
    size
}

// Zero-specific RLE: non-zero bytes emitted literally (1 byte each),
// zero runs as (0x00, u16 count) = 3 bytes per chunk.
fn zero_rle_size(data: &[u8]) -> usize {
    let mut size = 0;
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0 {
            let mut run = 1usize;
            while i + run < data.len() && data[i + run] == 0 { run += 1; }
            size += ((run + 65534) / 65535) * 3;
            i += run;
        } else {
            size += 1;
            i += 1;
        }
    }
    size
}

// Sparse: non-zero entries only as (u32 index, u8 value).
fn sparse_size(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b != 0).count() * 5
}

// Per-file hybrid: for each file, choose whichever is smaller -- the plain dense array
// (1 byte/entry) or a sparse (u24 index, i8 value) list of only the non-zero entries (4
// bytes/entry -- every file here is well under 2^24 = 16.7M entries, so a 24-bit index
// always fits). Prints every file where sparse wins, and the total size if every file
// independently picks its own best representation.
fn print_hybrid_dense_sparse_per_file(tb: &[Vec<Status>; 100]) {
    let mut total_dense = 0usize;
    let mut total_hybrid = 0usize;
    let mut files_where_sparse_wins = 0usize;
    for (i, file) in tb.iter().enumerate() {
        if file.is_empty() { continue; }
        let dense = file.len();
        let nonzero = file.iter().filter(|s| s.0 != 0).count();
        let sparse = nonzero * 4; // 3-byte index + 1-byte value
        total_dense += dense;
        total_hybrid += dense.min(sparse);
        if sparse < dense {
            files_where_sparse_wins += 1;
            println!("  file {:2}: dense={:>10}  nonzero={:>8}  sparse={:>10}  saves {:>10} bytes",
                i, dense, nonzero, sparse, dense - sparse);
        }
    }
    println!("  {} of 100 files favour sparse", files_where_sparse_wins);
    println!("  total dense:  {} bytes ({:.2} MB)", total_dense, mb(total_dense));
    println!("  total hybrid: {} bytes ({:.2} MB)", total_hybrid, mb(total_hybrid));
}

// Pair entropy: treat each consecutive pair as a 2-byte symbol.
// If adjacent values are correlated (e.g. (0, nonzero) pairs dominate),
// the pair entropy will be substantially less than 2× single-byte entropy,
// meaning a context model that conditions on the previous byte would help.
fn pair_entropy_size(data: &[u8]) -> usize {
    // Only sample up to 8M pairs to keep this fast.
    const MAX_PAIRS: usize = 8_000_000;
    let step = (data.len() / 2 / MAX_PAIRS).max(1);
    let mut counts = std::collections::HashMap::<(u8, u8), u64>::new();
    let mut sampled = 0u64;
    let mut i = 0;
    while i + 1 < data.len() {
        *counts.entry((data[i], data[i + 1])).or_insert(0) += 1;
        sampled += 1;
        i += 2 * step;
    }
    let total = sampled as f64;
    let bits_per_pair: f64 = counts.values()
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum::<f64>() / total;
    // Scale bits_per_pair back to the full dataset (pairs cover data.len()/2 entries).
    let pairs_full = (data.len() / 2) as f64;
    (bits_per_pair * pairs_full / 8.0).ceil() as usize
}

// Delta-encode each entry against its immediate predecessor (wrapping u8 subtraction),
// resetting at each file boundary since adjacent files have no positional relationship
// to each other. Tests whether "smooth local change" (as opposed to mere correlation
// with the raw previous value, which pair_entropy_size already tests) is the dominant
// structure -- i.e. whether delta-coding before entropy-coding would help.
fn delta_entropy_size(tb: &[Vec<Status>; 100]) -> usize {
    let mut counts = [0u64; 256];
    let mut total = 0u64;
    for file in tb.iter() {
        if file.is_empty() { continue; }
        counts[file[0].0 as u8 as usize] += 1; // first entry of a file has no predecessor
        total += 1;
        for w in file.windows(2) {
            let delta = w[1].0.wrapping_sub(w[0].0) as u8;
            counts[delta as usize] += 1;
            total += 1;
        }
    }
    let total_f = total as f64;
    let bits: f64 = counts.iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / total_f).log2()))
        .sum();
    (bits / 8.0).ceil() as usize
}

// Delta/XOR-style entropy against an entry `stride` positions earlier within the SAME
// file (never crossing a file boundary; entries with no such predecessor are encoded
// literally). stride=2 tests "same king+pawn squares, opposite side to move"
// correlation -- i.e. whether the side-to-move bit (currently innermost) should stay
// adjacent like this or be pulled out into its own structure.
fn stride_entropy_size(tb: &[Vec<Status>; 100], stride: usize) -> usize {
    let mut counts = [0u64; 256];
    let mut total = 0u64;
    for file in tb.iter() {
        if file.is_empty() { continue; }
        let lead = stride.min(file.len());
        for s in &file[..lead] {
            counts[s.0 as u8 as usize] += 1;
            total += 1;
        }
        for i in lead..file.len() {
            let delta = file[i].0.wrapping_sub(file[i - stride].0) as u8;
            counts[delta as usize] += 1;
            total += 1;
        }
    }
    let total_f = total as f64;
    let bits: f64 = counts.iter().filter(|&&c| c > 0)
        .map(|&c| c as f64 * -((c as f64 / total_f).log2()))
        .sum();
    (bits / 8.0).ceil() as usize
}

// Sum of each file's OWN entropy (rather than one global distribution over the
// concatenation of all files). If this is meaningfully smaller than the global
// entropy, splitting by file (and giving each its own model/table) is worth doing
// on its own, independent of any other transform.
fn per_file_entropy_sum(tb: &[Vec<Status>; 100]) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let data: Vec<u8> = f.iter().map(|s| s.0 as u8).collect();
            entropy_size(&data)
        })
        .sum()
}

// Entropy computed independently per fixed-size block (simulating a realistic
// block-based Huffman/range coder -- the kind real tablebase formats use -- rather
// than one global adaptive model), plus a rough per-block table-overhead estimate
// (1 byte per distinct symbol present in that block, standing in for a compact
// per-block header). Comparing across block sizes shows the adaptivity/overhead
// tradeoff: small blocks adapt to local statistics better but pay more header cost.
fn block_entropy_size(data: &[u8], block_size: usize) -> usize {
    let mut total = 0usize;
    let mut i = 0;
    while i < data.len() {
        let end = (i + block_size).min(data.len());
        let block = &data[i..end];
        let counts = byte_counts(block);
        let distinct = counts.iter().filter(|&&c| c > 0).count();
        let n = block.len() as f64;
        let bits: f64 = counts.iter().filter(|&&c| c > 0)
            .map(|&c| c as f64 * -((c as f64 / n).log2()))
            .sum();
        total += (bits / 8.0).ceil() as usize + distinct;
        i = end;
    }
    total
}

// Same as block_entropy_size, but blocks never cross a file boundary (the naive
// flattened version mixes statistics from two adjacent, differently-distributed files
// at every boundary). `reorder` picks whether each file is read in its natural order
// or side-outer-reordered before blocking.
fn block_entropy_size_per_file(tb: &[Vec<Status>; 100], block_size: usize, reorder: bool) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes = if reorder { reorder_side_outer(f) } else { f.iter().map(|s| s.0 as u8).collect() };
            block_entropy_size(&bytes, block_size)
        })
        .sum()
}

// Finds the `top_k` most frequent distinct n-byte windows in `data` (n = the length of
// each window). Used to build a small shared "dictionary" of common multi-byte
// sequences for dict_block_entropy_size.
fn top_ngrams(data: &[u8], n: usize, top_k: usize) -> Vec<Vec<u8>> {
    top_ngrams_with_counts(data, n, top_k).into_iter().map(|(s, _)| s).collect()
}

fn top_ngrams_with_counts(data: &[u8], n: usize, top_k: usize) -> Vec<(Vec<u8>, u64)> {
    if data.len() < n { return Vec::new(); }
    let mut counts: std::collections::HashMap<&[u8], u64> = std::collections::HashMap::new();
    for w in data.windows(n) {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut v: Vec<(&[u8], u64)> = counts.into_iter().collect();
    v.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    v.truncate(top_k);
    v.into_iter().map(|(s, c)| (s.to_vec(), c)).collect()
}

// Counts non-overlapping all-zero runs per length: each maximal run of consecutive zero
// bytes is greedily decomposed largest-length-first (e.g. a run of 100 zeros with
// lengths [64,32,16,8,4,2] contributes one to 64, one to 32, one to 4, and none to
// 16/8/2 -- 64+32+4=100, each byte of the run claimed by exactly one bucket). This is
// the non-overlapping counterpart to top_ngrams' all-zero entry, which uses an
// unaligned sliding window and so recounts the same underlying zero bytes at every
// offset and every length.
fn nonoverlapping_zero_run_counts(data: &[u8], lengths: &[usize]) -> std::collections::HashMap<usize, u64> {
    let mut sorted_lengths = lengths.to_vec();
    sorted_lengths.sort_unstable_by(|a, b| b.cmp(a));
    let mut counts: std::collections::HashMap<usize, u64> = lengths.iter().map(|&n| (n, 0)).collect();

    let mut i = 0usize;
    while i < data.len() {
        if data[i] != 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < data.len() && data[i] == 0 {
            i += 1;
        }
        let mut remaining = i - start;
        for &n in &sorted_lengths {
            if remaining >= n {
                let take = remaining / n;
                *counts.get_mut(&n).unwrap() += take as u64;
                remaining -= take * n;
            }
        }
    }
    counts
}

// Diagnostic for the trim-boundary hypothesis: each file is trimmed at generation time
// right after its last non-draw/non-unknown entry (see generator_impl's `trim` step), so
// the file's length is entirely determined by wherever that one entry happens to land.
// Reindexing (e.g. the king-table zigzag reordering) can shift which (kings,p1,p2,side)
// tuple ends up last in a file, without changing anything about the "real" content --
// if the new tail landed in a sparser neighbourhood than the old one did, the file grows
// purely from trim-boundary luck, independent of any actual compression-locality effect.
// This measures that directly: for each file, the "isolation gap" is the distance (in
// bytes) between the last two non-zero (post-unknowns-replaced) entries -- i.e. how much
// pure trailing zero run exists purely because of where that final entry landed.
fn print_trim_isolation(tb: &[Vec<Status>; 100]) {
    let mut total_gap = 0usize;
    for (i, file) in tb.iter().enumerate() {
        if file.is_empty() { continue; }
        let last_nonzero = file.iter().rposition(|s| s.0 != 0);
        let Some(last) = last_nonzero else { continue };
        let second_last = file[..last].iter().rposition(|s| s.0 != 0);
        let gap = match second_last {
            Some(sl) => last - sl,
            None => last + 1, // only one non-zero entry in the whole file
        };
        total_gap += gap;
        if gap > 1000 {
            println!("  file {:2}: len={:>10}  last_nonzero_at={:>10}  isolation_gap={:>8}",
                i, file.len(), last, gap);
        }
    }
    println!("  total isolation gap across all files: {} bytes ({:.2} KB)", total_gap, total_gap as f64 / 1024.0);
}

// Finds the top_k longest maximal runs of consecutive zero entries anywhere in the
// tablebase, printing each run's file, start offset, and length.
fn print_longest_zero_runs(tb: &[Vec<Status>; 100], top_k: usize) {
    let mut runs: Vec<(usize, usize, usize)> = Vec::new(); // (file_idx, start, len)
    for (i, file) in tb.iter().enumerate() {
        let mut j = 0usize;
        while j < file.len() {
            if file[j].0 != 0 {
                j += 1;
                continue;
            }
            let start = j;
            while j < file.len() && file[j].0 == 0 {
                j += 1;
            }
            runs.push((i, start, j - start));
        }
    }
    runs.sort_unstable_by_key(|&(_, _, len)| std::cmp::Reverse(len));
    runs.truncate(top_k);
    for (file_idx, start, len) in runs {
        println!("  file {:2}: start={:>10}  end={:>10}  len={:>10}", file_idx, start, start + len, len);
    }
}

// Directly inverts Pos::index()'s "(non-pawn, non-pawn)" twopiece formula (the catch-all
// `(_, _)` arm) for a given file/index, without brute-forcing over the position space:
// index = side*(NUM_KINGS_PAWNLESS*62*61) + king_idx*(62*61) + p1c*61 + p2c, where p1c/p2c
// are p1's/p2's raw square with already-placed squares (kings, and p1 for p2c) removed
// from the numbering. Each step is inverted directly: king_idx -> (wk,bk) is a reverse
// lookup on the small (32x64) KINGS_IDX_PAWNLESS table, and p1c/p2c -> raw square is a
// single pass over 0..64 counting non-excluded squares (the exact inverse of "compact by
// removing already-placed squares"). Only valid for files whose p1 and p2 are both
// non-pawn pieces.
fn invert_nonpawn_twopiece_index(target_index: usize) -> (usize, crate::repr::square::Square, crate::repr::square::Square, crate::repr::square::Square, crate::repr::square::Square) {
    use crate::repr::square::Square;
    use crate::egtb::{KINGS_IDX_PAWNLESS, NUM_KINGS_PAWNLESS};

    let sub_block = 62 * 61;
    let block = NUM_KINGS_PAWNLESS * sub_block;
    let side = target_index / block;
    let r1 = target_index % block;
    let king_idx = (r1 / sub_block) as u16;
    let r2 = r1 % sub_block;
    let p1c = r2 / 61;
    let p2c = r2 % 61;

    let mut wk_sq = None;
    'outer: for wk in 0u8..32 {
        for bk in 0u8..64 {
            if KINGS_IDX_PAWNLESS[wk as usize][bk as usize] == king_idx {
                wk_sq = Some((wk, bk));
                break 'outer;
            }
        }
    }
    let (wk, bk) = wk_sq.expect("king_idx not found in KINGS_IDX_PAWNLESS");

    let mut count = 0usize;
    let mut sq1 = None;
    for s in 0u8..64 {
        if s == wk || s == bk { continue; }
        if count == p1c { sq1 = Some(s); break; }
        count += 1;
    }
    let sq1 = sq1.expect("p1c out of range");

    let mut count = 0usize;
    let mut sq2 = None;
    for s in 0u8..64 {
        if s == wk || s == bk || s == sq1 { continue; }
        if count == p2c { sq2 = Some(s); break; }
        count += 1;
    }
    let sq2 = sq2.expect("p2c out of range");

    // Raw `side` as used by the forward formula (side = !last_moved as usize): side==0
    // (White's numeric value) means White to move next, side==1 (Black's) means Black
    // to move next -- left as the raw 0/1 so callers can round-trip it directly instead
    // of re-deriving it from a re-interpreted boolean.
    (side, Square::from_u8(wk), Square::from_u8(bk), Square::from_u8(sq1), Square::from_u8(sq2))
}

// Prints the top_k most frequent n-byte sequences (and their counts) for each length in
// `lengths`, found globally over the whole (flattened) byte stream. Values are shown
// re-interpreted as i8 (matching Status's signed DTM/WDL encoding) for readability.
fn print_top_ngrams(data: &[u8], lengths: &[usize], top_k: usize) {
    for &n in lengths {
        println!("-- top {} sequences of length {} --", top_k, n);
        for (seq, count) in top_ngrams_with_counts(data, n, top_k) {
            let signed: Vec<i8> = seq.iter().map(|&b| b as i8).collect();
            println!("  count={:>10}  {:?}", count, signed);
        }
    }
}

// Same as block_entropy_size, but the alphabet each block draws from isn't just the 256
// single bytes: it also includes a small shared "dictionary" of the top_k most frequent
// n-byte sequences for each length in `lengths` (found once over the whole file, so its
// own storage cost -- a few hundred bytes -- is paid once rather than per block). Each
// block is greedily tokenized left-to-right, trying the longest dictionary lengths
// first and falling back to a literal single byte, never letting a match cross the
// block boundary (so blocks stay independently decodable). The resulting mixed
// literal/dictionary token stream is then Huffman-entropy-coded per block, exactly like
// block_entropy_size.
fn dict_block_entropy_size(data: &[u8], block_size: usize, lengths: &[usize], top_k: usize) -> usize {
    let mut sorted_lengths: Vec<usize> = lengths.to_vec();
    sorted_lengths.sort_unstable_by(|a, b| b.cmp(a)); // longest first, so matching prefers longer tokens
    let dicts: Vec<(usize, std::collections::HashSet<Vec<u8>>)> = sorted_lengths.iter()
        .map(|&n| (n, top_ngrams(data, n, top_k).into_iter().collect()))
        .collect();
    let dict_overhead: usize = dicts.iter().map(|(n, set)| n * set.len()).sum();

    let mut total = dict_overhead; // paid once, globally, for this file
    let mut block_start = 0usize;
    while block_start < data.len() {
        let block_end = (block_start + block_size).min(data.len());
        let mut counts: std::collections::HashMap<&[u8], u64> = std::collections::HashMap::new();
        let mut i = block_start;
        let mut ntokens = 0u64;
        while i < block_end {
            let mut take = 1usize;
            for &(n, ref set) in &dicts {
                if i + n <= block_end && set.contains(&data[i..i + n]) {
                    take = n;
                    break;
                }
            }
            *counts.entry(&data[i..i + take]).or_insert(0) += 1;
            ntokens += 1;
            i += take;
        }
        let n = ntokens as f64;
        let bits: f64 = counts.values()
            .map(|&c| c as f64 * -((c as f64 / n).log2()))
            .sum();
        total += (bits / 8.0).ceil() as usize + counts.len();
        block_start = block_end;
    }
    total
}

fn dict_block_entropy_size_per_file(tb: &[Vec<Status>; 100], block_size: usize, lengths: &[usize], top_k: usize, reorder: bool) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes: Vec<u8> = if reorder { reorder_side_outer(f) } else { f.iter().map(|s| s.0 as u8).collect() };
            dict_block_entropy_size(&bytes, block_size, lengths, top_k)
        })
        .sum()
}

// Same as top_ngrams, but skips any n-gram already present in `exclude` (used to keep a
// smaller regional dictionary from re-listing entries a larger shared dictionary
// already covers).
fn top_ngrams_excluding(data: &[u8], n: usize, top_k: usize, exclude: &std::collections::HashSet<Vec<u8>>) -> Vec<Vec<u8>> {
    if data.len() < n { return Vec::new(); }
    let mut counts: std::collections::HashMap<&[u8], u64> = std::collections::HashMap::new();
    for w in data.windows(n) {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut v: Vec<(&[u8], u64)> = counts.into_iter().filter(|(s, _)| !exclude.contains(*s)).collect();
    v.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    v.truncate(top_k);
    v.into_iter().map(|(s, _)| s.to_vec()).collect()
}

// (match length -> set of byte sequences of that length that count as one token), kept
// sorted longest-first so tokenization always prefers the longest available match.
type LenDict = Vec<(usize, std::collections::HashSet<Vec<u8>>)>;

fn tokenize_and_entropy(data: &[u8], start: usize, end: usize, dicts: &LenDict) -> usize {
    let mut counts: std::collections::HashMap<&[u8], u64> = std::collections::HashMap::new();
    let mut i = start;
    let mut ntokens = 0u64;
    while i < end {
        let mut take = 1usize;
        for (n, set) in dicts {
            if i + n <= end && set.contains(&data[i..i + n]) {
                take = *n;
                break;
            }
        }
        *counts.entry(&data[i..i + take]).or_insert(0) += 1;
        ntokens += 1;
        i += take;
    }
    let n = ntokens as f64;
    let bits: f64 = counts.values()
        .map(|&c| c as f64 * -((c as f64 / n).log2()))
        .sum();
    (bits / 8.0).ceil() as usize + counts.len()
}

// Three-tier scheme:
//  - a small "global" dictionary (top-k n-grams for a few lengths, plus explicit
//    all-zero run tokens) shared across the whole file, paid for once;
//  - a "regional" dictionary (more n-grams, excluding anything the global tier already
//    covers) shared across each superblock of `block_size * regional_multiplier` bytes,
//    paid for once per superblock;
//  - ordinary per-`block_size` blocks, entropy-coded independently using whatever
//    combined (global + regional + literal) alphabet is in scope for that region.
// The idea: a fixed small dictionary shared globally avoids re-paying table overhead
// everywhere, while the regional tier still lets the alphabet adapt to what's actually
// common in that neighbourhood, without paying full per-block dictionary cost.
fn hierarchical_dict_block_entropy_size(
    data: &[u8],
    block_size: usize,
    regional_multiplier: usize,
    global_ngram_topk: &[(usize, usize)],
    zero_run_lengths: &[usize],
    regional_ngram_topk: &[(usize, usize)],
) -> usize {
    let mut global: LenDict = global_ngram_topk.iter()
        .map(|&(n, k)| (n, top_ngrams(data, n, k).into_iter().collect()))
        .collect();
    for &len in zero_run_lengths {
        global.push((len, std::iter::once(vec![0u8; len]).collect()));
    }
    global.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let global_overhead: usize = global.iter().map(|(n, set)| n * set.len()).sum();
    let empty_set: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();

    let superblock_size = block_size * regional_multiplier;
    let mut total = global_overhead;
    let mut super_start = 0usize;
    while super_start < data.len() {
        let super_end = (super_start + superblock_size).min(data.len());
        let region = &data[super_start..super_end];

        let mut combined: LenDict = global.clone();
        for &(n, k) in regional_ngram_topk {
            let exclude = global.iter().find(|(len, _)| *len == n).map(|(_, s)| s).unwrap_or(&empty_set);
            let regional_set: std::collections::HashSet<Vec<u8>> = top_ngrams_excluding(region, n, k, exclude).into_iter().collect();
            total += n * regional_set.len(); // regional dict overhead, paid once per superblock
            match combined.iter_mut().find(|(len, _)| *len == n) {
                Some(entry) => entry.1.extend(regional_set),
                None => combined.push((n, regional_set)),
            }
        }
        combined.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        let mut block_start = super_start;
        while block_start < super_end {
            let block_end = (block_start + block_size).min(super_end);
            total += tokenize_and_entropy(data, block_start, block_end, &combined);
            block_start = block_end;
        }
        super_start = super_end;
    }
    total
}

fn hierarchical_dict_block_entropy_size_per_file(tb: &[Vec<Status>; 100], block_size: usize, regional_multiplier: usize) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes: Vec<u8> = f.iter().map(|s| s.0 as u8).collect();
            hierarchical_dict_block_entropy_size(
                &bytes, block_size, regional_multiplier,
                &[(2, 5), (4, 5), (8, 5)], &[16, 32], &[(2, 10), (4, 10), (8, 10)],
            )
        })
        .sum()
}

// Block-adaptive entropy (same per-block Huffman-estimate machinery as
// block_entropy_size), but the base literal unit is a non-overlapping 2-byte pair
// instead of 1 byte, plus two fixed extra tokens: a run of exactly `zero_run_lengths`
// consecutive zero bytes (tried longest-first, e.g. [64, 8]), checked only when the
// current byte is zero (cheap short-circuit, since ~47% of bytes are zero but most
// don't start a long run). Falls back to the 2-byte literal (or a trailing single byte)
// when no zero-run token applies.
fn pair_block_entropy_size(data: &[u8], block_size: usize, zero_run_lengths: &[usize]) -> usize {
    let mut sorted_zero_runs: Vec<usize> = zero_run_lengths.to_vec();
    sorted_zero_runs.sort_unstable_by(|a, b| b.cmp(a));

    let mut total = 0usize;
    let mut block_start = 0usize;
    while block_start < data.len() {
        let block_end = (block_start + block_size).min(data.len());
        let mut counts: std::collections::HashMap<&[u8], u64> = std::collections::HashMap::new();
        let mut i = block_start;
        let mut ntokens = 0u64;
        while i < block_end {
            let mut take = None;
            if data[i] == 0 {
                for &n in &sorted_zero_runs {
                    if i + n <= block_end && data[i..i + n].iter().all(|&b| b == 0) {
                        take = Some(n);
                        break;
                    }
                }
            }
            let take = take.unwrap_or(if i + 2 <= block_end { 2 } else { 1 });
            *counts.entry(&data[i..i + take]).or_insert(0) += 1;
            ntokens += 1;
            i += take;
        }
        let n = ntokens as f64;
        let bits: f64 = counts.values()
            .map(|&c| c as f64 * -((c as f64 / n).log2()))
            .sum();
        total += (bits / 8.0).ceil() as usize + counts.len();
        block_start = block_end;
    }
    total
}

fn pair_block_entropy_size_per_file(tb: &[Vec<Status>; 100], block_size: usize, zero_run_lengths: &[usize]) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes: Vec<u8> = f.iter().map(|s| s.0 as u8).collect();
            pair_block_entropy_size(&bytes, block_size, zero_run_lengths)
        })
        .sum()
}

// Approximates the core idea of Syzygy's actual encoding (per vendor/shakmaty-syzygy's
// decompress_pairs/read_symlen: a Huffman code built over an alphabet whose symbols can
// each represent a *run* of consecutive identical entries, not just one entry -- so a
// single code captures both value-frequency skew and run-length redundancy at once).
// This is a simplified same-value-run version (Syzygy's real dictionary can in
// principle also reuse recurring multi-value patterns via its recursive left/right
// symbol composition; this only merges runs of one repeated value, capped at
// `max_run`). Deliberately skips the sparse-index random-access layer -- this is pure
// "how small would the coded stream be", not "how do you probe it quickly".
// Computed per file, matching Syzygy's real granularity (one canonical Huffman tree
// per table/file; blocks only bound random-access cost, they don't change the model).
fn run_token_entropy_size(data: &[u8], max_run: u32) -> usize {
    let mut counts: std::collections::HashMap<(u8, u32), u64> = std::collections::HashMap::new();
    let mut i = 0usize;
    let mut total_tokens = 0u64;
    while i < data.len() {
        let v = data[i];
        let mut run = 1u32;
        while i + (run as usize) < data.len() && data[i + run as usize] == v && run < max_run {
            run += 1;
        }
        *counts.entry((v, run)).or_insert(0) += 1;
        total_tokens += 1;
        i += run as usize;
    }
    let total = total_tokens as f64;
    let bits: f64 = counts.values()
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum();
    // Rough per-symbol table overhead: 2 bytes/symbol (covers the value + run-length
    // pair identifying it), matching the spirit of Syzygy's own per-symbol metadata.
    (bits / 8.0).ceil() as usize + counts.len() * 2
}

fn run_token_entropy_size_per_file(tb: &[Vec<Status>; 100], max_run: u32, reorder: bool) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes: Vec<u8> = if reorder { reorder_side_outer(f) } else { f.iter().map(|s| s.0 as u8).collect() };
            run_token_entropy_size(&bytes, max_run)
        })
        .sum()
}

// Same idea as run_token_entropy_size, but the fundamental unit is a non-overlapping
// 2-byte pair instead of a single byte, so a run can also collapse repeating
// multi-value patterns (e.g. an alternating 18,-19,18,-19,... sequence, which the
// single-byte version sees as run-length 1 throughout).
fn pair_run_token_entropy_size(data: &[u8], max_run: u32) -> usize {
    // An odd trailing byte (rare: at most one per file) is folded into a final unit by
    // reusing the previous byte as its pair partner; negligible effect on the estimate.
    let pairs: Vec<u16> = data.chunks(2)
        .map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&c[0]) as u16))
        .collect();

    let mut counts: std::collections::HashMap<(u16, u32), u64> = std::collections::HashMap::new();
    let mut i = 0usize;
    let mut total_tokens = 0u64;
    while i < pairs.len() {
        let v = pairs[i];
        let mut run = 1u32;
        while i + (run as usize) < pairs.len() && pairs[i + run as usize] == v && run < max_run {
            run += 1;
        }
        *counts.entry((v, run)).or_insert(0) += 1;
        total_tokens += 1;
        i += run as usize;
    }
    let total = total_tokens as f64;
    let bits: f64 = counts.values()
        .map(|&c| c as f64 * -((c as f64 / total).log2()))
        .sum();
    // Table overhead: 2 bytes for the pair value + 1 byte for the run length.
    (bits / 8.0).ceil() as usize + counts.len() * 3
}

fn pair_run_token_entropy_size_per_file(tb: &[Vec<Status>; 100], max_run: u32, reorder: bool) -> usize {
    tb.iter().filter(|f| !f.is_empty())
        .map(|f| {
            let bytes: Vec<u8> = if reorder { reorder_side_outer(f) } else { f.iter().map(|s| s.0 as u8).collect() };
            pair_run_token_entropy_size(&bytes, max_run)
        })
        .sum()
}

fn mb(bytes: usize) -> f64 { bytes as f64 / 1_048_576.0 }

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Real compression backend (zstd is a dev-dependency, only pulled in for this
    // analysis, not the main binary). level=19 is zstd's near-max "high effort" setting
    // without going into the very slow ultra levels; window_log widens the match window
    // well past the default so long-range repeats (this file is ~170MB) aren't missed.
    fn zstd_size(data: &[u8], level: i32) -> usize {
        let mut encoder = zstd::Encoder::new(Vec::new(), level).expect("zstd encoder init");
        encoder.window_log(27).ok(); // 128 MiB window
        encoder.long_distance_matching(true).ok(); // actually activates far-range match-finding
        encoder.write_all(data).expect("zstd write");
        let compressed = encoder.finish().expect("zstd finish");
        compressed.len()
    }

    // LZMA2-in-.xz-container at max preset with extreme mode -- the same underlying
    // algorithm 7z uses for its default .7z format, so this is the most direct real-
    // backend comparison to the ~14.8MB Explorer number available from Rust.
    fn lzma_size(data: &[u8], preset: u32, extreme: bool) -> usize {
        // xz2 doesn't re-export lzma-sys's LZMA_PRESET_EXTREME constant (1 << 31), so
        // it's inlined directly here.
        const LZMA_PRESET_EXTREME: u32 = 1 << 31;
        let preset = if extreme { preset | LZMA_PRESET_EXTREME } else { preset };
        let mut encoder = xz2::write::XzEncoder::new(Vec::new(), preset);
        encoder.write_all(data).expect("lzma write");
        let compressed = encoder.finish().expect("lzma finish");
        compressed.len()
    }

    // Same as lzma_size, but overrides the literal-context/literal-position/position
    // bit counts (lc/lp/pb) on top of a preset instead of using its defaults. Presets
    // pick generic values (typically lc=3,lp=0,pb=2, tuned for text/general binary);
    // our data is a mostly-zero stream of small-magnitude signed bytes with no
    // positional periodicity, so lower lp/pb (nothing meaningful repeats every 2/4
    // bytes) or different lc (how many high bits of the previous byte condition the
    // literal model) might fit better. lc+lp <= 4 is liblzma's hard constraint.
    fn lzma_size_custom(data: &[u8], preset: u32, extreme: bool, lc: u32, lp: u32, pb: u32) -> usize {
        const LZMA_PRESET_EXTREME: u32 = 1 << 31;
        let preset = if extreme { preset | LZMA_PRESET_EXTREME } else { preset };
        let mut opts = xz2::stream::LzmaOptions::new_preset(preset).expect("lzma options");
        opts.literal_context_bits(lc);
        opts.literal_position_bits(lp);
        opts.position_bits(pb);
        let mut filters = xz2::stream::Filters::new();
        filters.lzma2(&opts);
        let stream = xz2::stream::Stream::new_stream_encoder(&filters, xz2::stream::Check::Crc64)
            .expect("lzma stream encoder");
        let mut encoder = xz2::write::XzEncoder::new_stream(Vec::new(), stream);
        encoder.write_all(data).expect("lzma write");
        let compressed = encoder.finish().expect("lzma finish");
        compressed.len()
    }

    #[test]
    #[ignore]
    fn generate_untrimmed_tablebase() {
        let status = Pos::generator_untrimmed();
        save_tablebase(&status, "tablebase_untrimmed").expect("failed to save untrimmed tablebase");
    }

    #[test]
    #[ignore]
    fn compression_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");

        println!("\n=== Per-file breakdown ===");
        for (i, file) in tb.iter().enumerate() {
            if file.is_empty() { continue; }
            let zeros  = file.iter().filter(|s| s.0 == 0).count();
            let wins   = file.iter().filter(|s| s.0 > 0).count();
            let losses = file.iter().filter(|s| s.0 < 0).count();
            println!(
                "file {:2}: {:>10} entries  zeros={:>8} ({:5.1}%)  wins={:>7}  losses={:>7}",
                i, file.len(), zeros, zeros as f64 / file.len() as f64 * 100.0, wins, losses,
            );
        }

        let data = flatten(&tb);

        println!("\n=== Overall distribution ===");
        print_distribution(&data);

        let raw               = data.len();
        let rle               = rle_size(&data);
        let zero_rle          = zero_rle_size(&data);
        let sparse            = sparse_size(&data);
        let entropy           = entropy_size(&data);
        let (wdl_e, dtm_e)   = wdl_dtm_entropy_sizes(&data);
        let wdl_dtm_combined  = wdl_e + dtm_e;
        let pair_e            = pair_entropy_size(&data);

        println!("\n=== Compression results / estimates ===");
        println!("(* = theoretical minimum if we add arithmetic/Huffman coding on top)");
        println!();
        println!("Raw:                         {:>12} bytes  ({:.2} MB)  1.00x", raw,             mb(raw));
        println!("RLE (count,val):              {:>12} bytes  ({:.2} MB)  {:.2}x", rle,            mb(rle),            raw as f64 / rle as f64);
        println!("Zero-RLE:                    {:>12} bytes  ({:.2} MB)  {:.2}x", zero_rle,        mb(zero_rle),       raw as f64 / zero_rle as f64);
        println!("Sparse (idx+val):             {:>12} bytes  ({:.2} MB)  {:.2}x", sparse,         mb(sparse),         raw as f64 / sparse as f64);
        println!("Entropy lower bound *:        {:>12} bytes  ({:.2} MB)  {:.2}x", entropy,        mb(entropy),        raw as f64 / entropy as f64);
        println!("WDL entropy *:                {:>12} bytes  ({:.2} MB)  (for all {} entries)", wdl_e, mb(wdl_e), raw);
        println!("DTZ entropy *:                {:>12} bytes  ({:.2} MB)  (for non-zero entries only)", dtm_e, mb(dtm_e));
        println!("WDL + DTZ entropy combined *: {:>12} bytes  ({:.2} MB)  {:.2}x", wdl_dtm_combined, mb(wdl_dtm_combined), raw as f64 / wdl_dtm_combined as f64);
        println!("Pair entropy *:               {:>12} bytes  ({:.2} MB)  {:.2}x  (2x single would be {:.2} MB)", pair_e, mb(pair_e), raw as f64 / pair_e as f64, mb(entropy * 2));
        println!();
        println!("Note: if pair entropy << 2x single entropy, adjacent values are correlated");
        println!("      and a context-conditioned model (e.g. PPM, LSTM) would help further.");

        let delta_e     = delta_entropy_size(&tb);
        let stride2_e   = stride_entropy_size(&tb, 2);
        let per_file_e  = per_file_entropy_sum(&tb);

        println!("\n=== Additional diagnostics ===");
        println!("Delta entropy * (vs prev entry, reset per file): {:>12} bytes  ({:.2} MB)  {:.2}x", delta_e, mb(delta_e), raw as f64 / delta_e as f64);
        println!("Stride-2 entropy * (vs same pos, other side):    {:>12} bytes  ({:.2} MB)  {:.2}x", stride2_e, mb(stride2_e), raw as f64 / stride2_e as f64);
        println!("Per-file entropy sum * (own distribution each):  {:>12} bytes  ({:.2} MB)  {:.2}x  (global was {:.2} MB)", per_file_e, mb(per_file_e), raw as f64 / per_file_e as f64, mb(entropy));
        println!();
        println!("Note: if delta entropy << pair entropy, values change smoothly (delta-code first).");
        println!("      if per-file sum << global entropy, per-file models are worth it on their own.");

        let data_reord = flatten_side_outer(&tb);
        let block_sizes = [256usize, 512, 1024, 4096, 65536];

        println!("\n=== Block entropy sweep (current order vs side-outer reorder; flattened vs per-file) ===");
        println!("(* = theoretical minimum with block-adaptive Huffman/range coding + rough table overhead)");
        println!();
        println!("{:>8}  {:>14}  {:>14}  {:>14}  {:>14}", "block", "flat/cur", "flat/reord", "per-file/cur", "per-file/reord");
        for &bs in &block_sizes {
            let flat_cur    = block_entropy_size(&data, bs);
            let flat_reord  = block_entropy_size(&data_reord, bs);
            let pf_cur      = block_entropy_size_per_file(&tb, bs, false);
            let pf_reord    = block_entropy_size_per_file(&tb, bs, true);
            println!("{:>8}  {:>10.2} MB  {:>10.2} MB  {:>10.2} MB  {:>10.2} MB", bs, mb(flat_cur), mb(flat_reord), mb(pf_cur), mb(pf_reord));
        }
        println!();
        println!("Note: 'per-file' never lets a block span two files (the flattened version can, at");
        println!("      every file boundary, mixing two differently-distributed files' statistics).");

        let lz_current = lz_match_fraction(&data);
        let lz_reord   = lz_match_fraction(&data_reord);
        let entropy_r  = entropy_size(&data_reord);

        println!("\n=== Side-outer reorder ([colour][kings][p1][p2]) summary ===");
        println!("Entropy lower bound * (reordered):   {:>12} bytes  ({:.2} MB)  {:.2}x  (current order was {:.2} MB)", entropy_r, mb(entropy_r), raw as f64 / entropy_r as f64, mb(entropy));
        println!("LZ match coverage (current order):   {:.2}% of bytes fall inside a >=8-byte repeat", lz_current * 100.0);
        println!("LZ match coverage (reordered):        {:.2}% of bytes fall inside a >=8-byte repeat", lz_reord * 100.0);
        println!();
        println!("Note: LZ match coverage estimates the redundancy only dictionary/LZ-style matching");
        println!("      can exploit (what 7z is doing) -- this is what should explain the gap between");
        println!("      our best entropy-based estimate above and 7z's actual ~15MB result, if anything does.");

        println!("\n=== Real backend: zstd level 19, 128MiB window ===");
        let zstd_cur    = zstd_size(&data, 19);
        let zstd_reord  = zstd_size(&data_reord, 19);
        let zstd_pf_cur: usize = tb.iter().filter(|f| !f.is_empty())
            .map(|f| zstd_size(&f.iter().map(|s| s.0 as u8).collect::<Vec<u8>>(), 19))
            .sum();
        let zstd_pf_reord: usize = tb.iter().filter(|f| !f.is_empty())
            .map(|f| zstd_size(&reorder_side_outer(f), 19))
            .sum();
        println!("zstd, current order, single stream:   {:>12} bytes  ({:.2} MB)  {:.2}x", zstd_cur, mb(zstd_cur), raw as f64 / zstd_cur as f64);
        println!("zstd, reordered, single stream:       {:>12} bytes  ({:.2} MB)  {:.2}x", zstd_reord, mb(zstd_reord), raw as f64 / zstd_reord as f64);
        println!("zstd, current order, per-file:        {:>12} bytes  ({:.2} MB)  {:.2}x", zstd_pf_cur, mb(zstd_pf_cur), raw as f64 / zstd_pf_cur as f64);
        println!("zstd, reordered, per-file:             {:>12} bytes  ({:.2} MB)  {:.2}x", zstd_pf_reord, mb(zstd_pf_reord), raw as f64 / zstd_pf_reord as f64);
        println!();
        println!("Note: per-file loses cross-file dictionary sharing (each file starts its own zstd");
        println!("      stream from scratch), which can hurt if separate files repeat similar patterns;");
        println!("      compare against the single-stream numbers to see whether that cost is worth");
        println!("      whatever independent-model benefit splitting provides.");
    }

    #[test]
    #[ignore]
    fn syzygy_style_run_token_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let raw: usize = tb.iter().map(|f| f.len()).sum();

        println!("\n=== Syzygy-style: Huffman over same-value-run tokens, per file ===");
        println!("(simplified reimplementation of the idea in vendor/shakmaty-syzygy's decoder;");
        println!(" skips the sparse-index random-access layer, as requested)");
        println!();
        for &max_run in &[16u32, 64, 255, 1024] {
            let cur   = run_token_entropy_size_per_file(&tb, max_run, false);
            let reord = run_token_entropy_size_per_file(&tb, max_run, true);
            println!("max_run={:>5}: current={:>12} bytes ({:.2} MB, {:.2}x)   reordered={:>12} bytes ({:.2} MB, {:.2}x)",
                max_run, cur, mb(cur), raw as f64 / cur as f64, reord, mb(reord), raw as f64 / reord as f64);
        }

        println!("\n=== Same idea, but run tokens are over 2-byte pairs instead of single bytes ===");
        println!();
        for &max_run in &[16u32, 64, 255, 1024] {
            let cur   = pair_run_token_entropy_size_per_file(&tb, max_run, false);
            let reord = pair_run_token_entropy_size_per_file(&tb, max_run, true);
            println!("max_run={:>5}: current={:>12} bytes ({:.2} MB, {:.2}x)   reordered={:>12} bytes ({:.2} MB, {:.2}x)",
                max_run, cur, mb(cur), raw as f64 / cur as f64, reord, mb(reord), raw as f64 / reord as f64);
        }
    }

    #[test]
    #[ignore]
    fn dict_augmented_block_entropy_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let raw: usize = tb.iter().map(|f| f.len()).sum();

        println!("\n=== Block-adaptive entropy, alphabet augmented with top-15 2/4/8-byte n-grams (45 extra tokens, per file) ===");
        println!();
        for &block_size in &[1024usize, 2048, 4096] {
            let plain = block_entropy_size_per_file(&tb, block_size, false);
            let dict  = dict_block_entropy_size_per_file(&tb, block_size, &[2, 4, 8], 15, false);
            println!("block={:>5}: plain block-entropy={:>12} bytes ({:.2} MB, {:.2}x)   dict-augmented={:>12} bytes ({:.2} MB, {:.2}x)",
                block_size, plain, mb(plain), raw as f64 / plain as f64, dict, mb(dict), raw as f64 / dict as f64);
        }
    }

    #[test]
    fn decode_isolation_gap_positions() {
        // file 6 = KNNvK (p1=WhiteKnight, p2=WhiteKnight), file 37 = KBvKN
        // (p1=WhiteBishop, p2=BlackKnight); both non-pawn, so the "(_, _)" index arm
        // applies. Targets are the last and second-to-last non-zero entries found by
        // trim_isolation_diagnostic on the current tablebase.
        let cases = [
            ("file 6 (KNNvK) last_nonzero",        "N", "N", 3_000_244usize),
            ("file 6 (KNNvK) second_last_nonzero",  "N", "N", 2_667_366usize),
            ("file 37 (KBvKN) last_nonzero",        "B", "n", 2_644_113usize),
            ("file 37 (KBvKN) second_last_nonzero", "B", "n", 1_747_674usize),
        ];
        for (label, p1_label, p2_label, target) in cases {
            let (side, wk, bk, sq1, sq2) = invert_nonpawn_twopiece_index(target);
            // side = !last_moved: side==0 means last_moved=Black (White to move next),
            // side==1 means last_moved=White (Black to move next).
            println!("{label}: WK={} BK={} {}={} {}={} side_to_move={}",
                wk.to_fen(), bk.to_fen(), p1_label, sq1.to_fen(), p2_label, sq2.to_fen(),
                if side == 0 { "White" } else { "Black" });

            // Round-trip: recompute the forward formula directly from the decoded
            // squares and confirm it reproduces the target index exactly.
            use crate::egtb::{KINGS_IDX_PAWNLESS, NUM_KINGS_PAWNLESS};
            let wk_u8 = wk as u8;
            let bk_u8 = bk as u8;
            let sq1_u8 = sq1 as u8;
            let sq2_u8 = sq2 as u8;
            let king_idx = KINGS_IDX_PAWNLESS[wk_u8 as usize][bk_u8 as usize] as usize;
            let p1c = sq1_u8 as usize - (sq1_u8 > wk_u8) as usize - (sq1_u8 > bk_u8) as usize;
            let p2c = sq2_u8 as usize
                - (sq2_u8 > wk_u8) as usize - (sq2_u8 > bk_u8) as usize - (sq2_u8 > sq1_u8) as usize;
            let recomputed = side * (NUM_KINGS_PAWNLESS * 62 * 61) + king_idx * (62 * 61) + p1c * 61 + p2c;
            assert_eq!(recomputed, target, "round-trip mismatch for {label}");
        }
    }

    #[test]
    #[ignore]
    fn trim_isolation_diagnostic() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        println!("\n=== Trim-boundary isolation gap per file (files with gap > 1000 bytes shown) ===");
        print_trim_isolation(&tb);
    }

    #[test]
    #[ignore]
    fn lzma_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let data = flatten(&tb);
        let raw = data.len();

        println!("\n=== LZMA (xz2, LZMA2-in-.xz) at various presets ===");
        for &(preset, extreme) in &[(6u32, false), (9, false), (9, true)] {
            let size = lzma_size(&data, preset, extreme);
            println!("preset={} extreme={}: {} bytes ({:.2} MB, {:.2}x)",
                preset, extreme, size, mb(size), raw as f64 / size as f64);
        }
    }

    #[test]
    #[ignore]
    fn lzma_custom_params_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let data = flatten(&tb);
        let raw = data.len();

        println!("\n=== LZMA (xz2), preset 9 extreme, custom lc/lp/pb vs default (lc=3,lp=0,pb=2) ===");
        // pb sweep at default lc/lp: our data has no fixed-width positional structure
        // (unlike e.g. audio samples or fixed-width records), so lower pb (fewer
        // position-dependent probability contexts) may fit better than the default 2.
        for &pb in &[0u32, 1, 2] {
            let size = lzma_size_custom(&data, 9, true, 3, 0, pb);
            println!("lc=3 lp=0 pb={}: {} bytes ({:.2} MB, {:.2}x)", pb, size, mb(size), raw as f64 / size as f64);
        }
        // lc sweep at pb=0 (whichever pb wins above would be substituted here in a
        // follow-up; starting from pb=0 as the more principled default for this data).
        for &lc in &[0u32, 1, 2, 3, 4] {
            let size = lzma_size_custom(&data, 9, true, lc, 0, 0);
            println!("lc={} lp=0 pb=0: {} bytes ({:.2} MB, {:.2}x)", lc, size, mb(size), raw as f64 / size as f64);
        }
    }

    #[test]
    #[ignore]
    fn hybrid_dense_sparse_zstd_comparison() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");

        let all_11 = [6usize, 12, 31, 32, 36, 37, 38, 42, 43, 48, 54];
        let under_100 = [6usize, 36, 37, 42];

        let dense_data = flatten(&tb);
        let hybrid_11_data = flatten_hybrid(&tb, &all_11);
        let hybrid_4_data = flatten_hybrid(&tb, &under_100);

        println!("\n=== zstd level 19 on dense vs hybrid (sparse-encoded selected files) ===");
        println!("raw sizes: dense={} ({:.2} MB)  hybrid_11={} ({:.2} MB)  hybrid_4={} ({:.2} MB)",
            dense_data.len(), mb(dense_data.len()),
            hybrid_11_data.len(), mb(hybrid_11_data.len()),
            hybrid_4_data.len(), mb(hybrid_4_data.len()));

        let dense_z = zstd_size(&dense_data, 19);
        let hybrid_11_z = zstd_size(&hybrid_11_data, 19);
        let hybrid_4_z = zstd_size(&hybrid_4_data, 19);

        println!("zstd(dense):                                {} bytes ({:.2} MB)", dense_z, mb(dense_z));
        println!("zstd(hybrid, all 11 sparsified):            {} bytes ({:.2} MB)", hybrid_11_z, mb(hybrid_11_z));
        println!("zstd(hybrid, only <100-nonzero sparsified): {} bytes ({:.2} MB)", hybrid_4_z, mb(hybrid_4_z));
    }

    #[test]
    #[ignore]
    fn hybrid_dense_sparse_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        println!("\n=== Per-file hybrid: dense array vs sparse (u24 idx, i8 val) list, whichever is smaller ===");
        print_hybrid_dense_sparse_per_file(&tb);
    }

    #[test]
    #[ignore]
    fn nonzero_counts_for_specific_files() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        for &i in &[6usize, 36, 37, 42] {
            let file = &tb[i];
            let nonzero = file.iter().filter(|s| s.0 != 0).count();
            println!("  file {:2}: len={:>10}  nonzero={:>8}  ({:.5}%)",
                i, file.len(), nonzero, nonzero as f64 / file.len() as f64 * 100.0);
        }
    }

    #[test]
    #[ignore]
    fn longest_zero_runs_diagnostic() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        println!("\n=== Top 10 longest consecutive-zero runs ===");
        print_longest_zero_runs(&tb, 10);
    }

    #[test]
    #[ignore]
    fn global_top_ngrams_diagnostic() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let data = flatten(&tb);
        println!("\n=== Global top-10 n-grams, by length ===");
        print_top_ngrams(&data, &[2, 4, 8, 16, 32, 64], 10);

        println!("\n=== Non-overlapping all-zero-run counts, by length (largest-length-first decomposition) ===");
        let lengths = [2usize, 4, 8, 16, 32, 64];
        let counts = nonoverlapping_zero_run_counts(&data, &lengths);
        for &n in &lengths {
            println!("  length={:>3}: count={:>12}", n, counts[&n]);
        }
    }

    #[test]
    #[ignore]
    fn pair_block_entropy_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let raw: usize = tb.iter().map(|f| f.len()).sum();

        println!("\n=== Block-adaptive entropy, pair-unit literals + zero-run tokens for all powers of two up to 512 ===");
        println!();
        let zero_runs: Vec<usize> = (1..=9).map(|p| 1usize << p).collect(); // 2,4,8,...,512
        for &block_size in &[1024usize, 2048, 4096, 8192, 16384, 32768] {
            let size = pair_block_entropy_size_per_file(&tb, block_size, &zero_runs);
            println!("block={:>5}: {:>12} bytes ({:.2} MB, {:.2}x)",
                block_size, size, mb(size), raw as f64 / size as f64);
        }

        println!("\n=== Same, but zero-run tokens also include 6,10,12,14,20,26,48,80,100,150,200 ===");
        println!();
        let mut zero_runs_extra = zero_runs.clone();
        zero_runs_extra.extend_from_slice(&[6, 10, 12, 14, 20, 26, 48, 80, 100, 150, 200]);
        for &block_size in &[1024usize, 2048, 4096, 8192, 16384, 32768] {
            let size = pair_block_entropy_size_per_file(&tb, block_size, &zero_runs_extra);
            println!("block={:>5}: {:>12} bytes ({:.2} MB, {:.2}x)",
                block_size, size, mb(size), raw as f64 / size as f64);
        }
    }

    #[test]
    #[ignore]
    fn hierarchical_dict_block_entropy_analysis() {
        let tb = load_replacing_unknowns("tablebase").expect("failed to load tablebase");
        let raw: usize = tb.iter().map(|f| f.len()).sum();

        println!("\n=== Hierarchical dict: global top-5 2/4/8-grams + zero-runs(16,32), regional top-10 2/4/8-grams per (block*8) ===");
        println!();
        for &block_size in &[1024usize, 2048, 4096, 8192] {
            let size = hierarchical_dict_block_entropy_size_per_file(&tb, block_size, 8);
            println!("block={:>5} (region={:>6}): {:>12} bytes ({:.2} MB, {:.2}x)",
                block_size, block_size * 8, size, mb(size), raw as f64 / size as f64);
        }
    }

    #[test]
    #[ignore]
    fn zstd_ultra_level_check() {
        let tb = load_replacing_unknowns("tablebase_untrimmed").expect("failed to load tablebase_untrimmed");
        let data = flatten(&tb);
        let data_reord = flatten_side_outer(&tb);
        let raw = data.len();

        for level in [19] {
            let cur   = zstd_size(&data, level);
            let reord = zstd_size(&data_reord, level);
            println!("zstd level {level} (LDM enabled): current={:.2} MB ({:.2}x)  reordered={:.2} MB ({:.2}x)",
                mb(cur), raw as f64 / cur as f64, mb(reord), raw as f64 / reord as f64);
        }
    }
}
