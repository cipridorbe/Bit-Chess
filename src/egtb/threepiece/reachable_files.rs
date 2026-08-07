use crate::egtb::threepiece::pos::Pos;

// File-level predecessor reachability: for each material file, the *other* files that
// generating predecessors of a position in that file can land in -- via uncaptures (which
// add a piece, only possible from 1/2-piece files, landing one piece count higher) and
// unpromotions (which change a slot's piece type back to a pawn, possibly changing file()
// even from a 1/2-piece source, since re-sorting the pawn against the remaining slot(s)
// can move it to a different file() bucket -- don't assume piece-count parity between a
// source and its targets, it varies per file and was wrong to hand-derive; this table is
// the empirical ground truth). Independent of king placement (see the doc comment on the
// test that derives it), so it's precomputed once here rather than recomputed at
// generation time.
//
// Sorted ascending by source file, so `reachable_files` can binary-search it.
//
// Regenerate by running `cargo test --release --lib
// egtb::threepiece::generator::reachability::compute -- --nocapture` and pasting its
// "file X: reachable -> {...}" lines back in below (mechanically: each becomes
// `(X, &[...]),`) if piece encoding or move generation ever changes.
const REACHABLE_FILES: &[(usize, &[usize])] = &[
    (10, &[0, 5, 121, 181, 242, 302, 363, 423, 484, 544]),
    (65, &[5, 126, 247, 368, 489]),
    (120, &[10, 65, 131, 186, 252, 307, 373, 428, 494, 549]),
    (121, &[0]),
    (126, &[5]),
    (131, &[10, 121, 126, 132, 181, 187, 253, 302, 313, 374, 423, 434, 495, 544, 555]),
    (132, &[121]),
    (133, &[132]),
    (137, &[126]),
    (138, &[137, 187]),
    (142, &[131, 132, 133, 137, 138, 187, 254, 313, 314, 375, 434, 435, 496, 555, 556]),
    (181, &[5]),
    (186, &[65, 126, 137, 181, 187, 247, 258, 308, 368, 379, 429, 489, 500, 550]),
    (187, &[126, 181]),
    (197, &[137, 138, 186, 187, 258, 259, 379, 380, 500, 501]),
    (241, &[120, 131, 142, 186, 197, 263, 307, 318, 384, 428, 439, 505, 549, 560]),
    (242, &[0]),
    (247, &[5]),
    (252, &[10, 181, 242, 247, 253, 264, 302, 308, 319, 385, 423, 445, 506, 544, 566]),
    (253, &[121, 242]),
    (254, &[132, 253]),
    (258, &[126, 247]),
    (259, &[187, 258, 308]),
    (263, &[131, 187, 252, 253, 254, 258, 259, 265, 308, 313, 319, 320, 386, 434, 445, 446, 507, 555, 566, 567]),
    (264, &[242]),
    (265, &[253, 264]),
    (266, &[264]),
    (269, &[247]),
    (270, &[269, 308]),
    (271, &[269, 319]),
    (274, &[252, 264, 265, 266, 269, 270, 271, 308, 319, 387, 445, 447, 508, 566, 568]),
    (302, &[5]),
    (307, &[65, 126, 247, 258, 269, 302, 313, 319, 368, 390, 440, 489, 511, 561]),
    (308, &[181, 247]),
    (313, &[126, 302]),
    (314, &[137, 313]),
    (318, &[137, 186, 258, 259, 269, 270, 307, 308, 313, 314, 320, 379, 390, 391, 441, 500, 511, 512, 562]),
    (319, &[247, 302]),
    (320, &[258, 313, 319]),
    (329, &[258, 269, 271, 307, 319, 320, 390, 392, 511, 513]),
    (362, &[120, 186, 252, 263, 274, 307, 318, 329, 395, 428, 450, 516, 549, 571]),
    (363, &[0]),
    (368, &[5]),
    (373, &[10, 181, 302, 363, 368, 374, 385, 396, 423, 429, 440, 451, 517, 544, 577]),
    (374, &[121, 363]),
    (375, &[132, 374]),
    (379, &[126, 368]),
    (380, &[187, 379, 429]),
    (384, &[131, 187, 313, 373, 374, 375, 379, 380, 386, 397, 429, 434, 440, 441, 451, 452, 518, 555, 577, 578]),
    (385, &[242, 363]),
    (386, &[253, 374, 385]),
    (387, &[264, 385]),
    (390, &[247, 368]),
    (391, &[308, 390, 429]),
    (392, &[319, 390, 440]),
    (395, &[252, 308, 319, 373, 385, 386, 387, 390, 391, 392, 398, 429, 440, 445, 451, 453, 519, 566, 577, 579]),
    (396, &[363]),
    (397, &[374, 396]),
    (398, &[385, 396]),
    (399, &[396]),
    (401, &[368]),
    (402, &[401, 429]),
    (403, &[401, 440]),
    (404, &[401, 451]),
    (406, &[373, 396, 397, 398, 399, 401, 402, 403, 404, 429, 440, 451, 520, 577, 580]),
    (423, &[5]),
    (428, &[65, 126, 247, 368, 379, 390, 401, 423, 434, 445, 451, 489, 522, 572]),
    (429, &[181, 368]),
    (434, &[126, 423]),
    (435, &[137, 434]),
    (439, &[137, 186, 258, 379, 380, 390, 391, 401, 402, 428, 429, 434, 435, 446, 452, 500, 522, 523, 573]),
    (440, &[302, 368]),
    (441, &[313, 379, 440]),
    (445, &[247, 423]),
    (446, &[258, 434, 445]),
    (447, &[269, 445]),
    (450, &[258, 269, 307, 379, 390, 392, 401, 403, 428, 440, 441, 445, 446, 447, 453, 511, 522, 524, 574]),
    (451, &[368, 423]),
    (452, &[379, 434, 451]),
    (453, &[390, 445, 451]),
    (461, &[379, 390, 401, 404, 428, 451, 452, 453, 522, 525]),
    (483, &[120, 186, 307, 373, 384, 395, 406, 428, 439, 450, 461, 527, 549, 582]),
    (484, &[0]),
    (489, &[5]),
    (494, &[10, 181, 302, 423, 484, 489, 495, 506, 517, 528, 544, 550, 561, 572, 583]),
    (495, &[121, 484]),
    (496, &[132, 495]),
    (500, &[126, 489]),
    (501, &[187, 500, 550]),
    (505, &[131, 187, 313, 434, 494, 495, 496, 500, 501, 507, 518, 529, 550, 555, 561, 562, 572, 573, 583, 584]),
    (506, &[242, 484]),
    (507, &[253, 495, 506]),
    (508, &[264, 506]),
    (511, &[247, 489]),
    (512, &[308, 511, 550]),
    (513, &[319, 511, 561]),
    (516, &[252, 308, 319, 445, 494, 506, 507, 508, 511, 512, 513, 519, 530, 550, 561, 566, 572, 574, 583, 585]),
    (517, &[363, 484]),
    (518, &[374, 495, 517]),
    (519, &[385, 506, 517]),
    (520, &[396, 517]),
    (522, &[368, 489]),
    (523, &[429, 522, 550]),
    (524, &[440, 522, 561]),
    (525, &[451, 522, 572]),
    (527, &[373, 429, 440, 451, 494, 517, 518, 519, 520, 522, 523, 524, 525, 531, 550, 561, 572, 577, 583, 586]),
    (528, &[484]),
    (529, &[495, 528]),
    (530, &[506, 528]),
    (531, &[517, 528]),
    (532, &[528]),
    (533, &[489]),
    (534, &[533, 550]),
    (535, &[533, 561]),
    (536, &[533, 572]),
    (537, &[533, 583]),
    (538, &[494, 528, 529, 530, 531, 532, 533, 534, 535, 536, 537, 550, 561, 572, 583]),
    (544, &[5]),
    (549, &[65, 126, 247, 368, 489, 500, 511, 522, 533, 544, 555, 566, 577, 583]),
    (550, &[181, 489]),
    (555, &[126, 544]),
    (556, &[137, 555]),
    (560, &[137, 186, 258, 379, 500, 501, 511, 512, 522, 523, 533, 534, 549, 550, 555, 556, 567, 578, 584]),
    (561, &[302, 489]),
    (562, &[313, 500, 561]),
    (566, &[247, 544]),
    (567, &[258, 555, 566]),
    (568, &[269, 566]),
    (571, &[258, 269, 307, 390, 500, 511, 513, 522, 524, 533, 535, 549, 561, 562, 566, 567, 568, 579, 585]),
    (572, &[423, 489]),
    (573, &[434, 500, 572]),
    (574, &[445, 511, 572]),
    (577, &[368, 544]),
    (578, &[379, 555, 577]),
    (579, &[390, 566, 577]),
    (580, &[401, 577]),
    (582, &[379, 390, 401, 428, 500, 511, 522, 525, 533, 536, 549, 572, 573, 574, 577, 578, 579, 580, 586]),
    (583, &[489, 544]),
    (584, &[500, 555, 583]),
    (585, &[511, 566, 583]),
    (586, &[522, 577, 583]),
    (593, &[500, 511, 522, 533, 537, 549, 583, 584, 585, 586]),
    (604, &[120, 186, 307, 428, 494, 505, 516, 527, 538, 549, 560, 571, 582, 593]),
];

// All other files reachable, via predecessor generation, from positions in `file`. Empty
// for the ~600 files not present in the table (either terminal-ish files with no such
// predecessors, or files never returned by REACHABLE_FILES' source sweep at all).
pub(crate) fn reachable_files(file: usize) -> &'static [usize] {
    match REACHABLE_FILES.binary_search_by_key(&file, |&(f, _)| f) {
        Ok(i) => REACHABLE_FILES[i].1,
        Err(_) => &[],
    }
}

// The subset of reachable_files(file) that are themselves 3-piece files -- i.e. the ones
// that must share a fixed memory budget when `file` is being drained (uncapture targets
// from a 1/2-piece source, or unpromotion targets from a 3-piece source). Reachable files
// with piece_count <= 2 get unlimited budget instead, same as `file`'s own bucket, since
// low-piece files are negligibly small.
pub(crate) fn three_piece_targets(file: usize) -> impl Iterator<Item = usize> {
    reachable_files(file).iter().copied().filter(|&f| Pos::piece_count_for_file(f) == 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_by_source_file() {
        assert!(REACHABLE_FILES.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn lookup_matches_table() {
        assert_eq!(reachable_files(121), &[0]);
        assert_eq!(reachable_files(10), &[0, 5, 121, 181, 242, 302, 363, 423, 484, 544]);
        assert_eq!(reachable_files(999), &[] as &[usize]); // not a real file, no entry
        assert_eq!(reachable_files(1), &[] as &[usize]); // real file, no reachable targets
    }

    #[test]
    fn three_piece_targets_excludes_low_piece_entries() {
        // file 131 (2-piece) reaches 15 files, 14 of which are 3-piece uncapture targets
        // and one (file 10, also 2-piece) is a same-piece-count target -- that one must be
        // excluded, since it gets unlimited budget like file 131's own bucket does.
        let targets: Vec<usize> = three_piece_targets(131).collect();
        assert!(!targets.contains(&10));
        assert!(targets.iter().all(|&f| Pos::piece_count_for_file(f) == 3));
        assert_eq!(targets.len(), reachable_files(131).len() - 1);

        // file 604 (WQ alone, 1-piece) reaches file 120 (WP alone, 1-piece, via
        // unpromotion) plus thirteen 2-piece uncapture targets -- none are 3-piece, so
        // the budgeted pool for file 604 is empty (every one of its targets is unlimited).
        assert_eq!(three_piece_targets(604).count(), 0);
    }
}
