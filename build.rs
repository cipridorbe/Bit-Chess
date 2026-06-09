use std::io::Write;

fn sw_pdep(mut val: u64, mut mask: u64) -> u64 {
    let mut result = 0u64;
    while mask != 0 {
        let lsb = mask & mask.wrapping_neg();
        if val & 1 != 0 { result |= lsb; }
        val >>= 1;
        mask &= mask - 1;
    }
    result
}

fn sw_pext(val: u64, mut mask: u64) -> u64 {
    let mut result = 0u64;
    let mut bit = 0u32;
    while mask != 0 {
        let lsb = mask & mask.wrapping_neg();
        if val & lsb != 0 { result |= 1u64 << bit; }
        bit += 1;
        mask &= mask - 1;
    }
    result
}

fn rook_attacks(sq: usize, occ: u64) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut out = 0u64;
    for (dr, df) in [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while (0..8).contains(&r) && (0..8).contains(&f) {
            let s = (r * 8 + f) as u32;
            out |= 1 << s;
            if occ & (1 << s) != 0 { break; }
            r += dr; f += df;
        }
    }
    out
}

fn bishop_attacks(sq: usize, occ: u64) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut out = 0u64;
    for (dr, df) in [(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while (0..8).contains(&r) && (0..8).contains(&f) {
            let s = (r * 8 + f) as u32;
            out |= 1 << s;
            if occ & (1 << s) != 0 { break; }
            r += dr; f += df;
        }
    }
    out
}

fn rook_blocker_mask(sq: usize) -> u64 {
    let (rank, file) = (sq / 8, sq % 8);
    let mut mask = 0u64;
    for r in 1..7usize { if r != rank { mask |= 1 << (r * 8 + file); } }
    for f in 1..7usize { if f != file { mask |= 1 << (rank * 8 + f); } }
    mask
}

fn bishop_blocker_mask(sq: usize) -> u64 {
    let (rank, file) = ((sq / 8) as i8, (sq % 8) as i8);
    let mut mask = 0u64;
    for (dr, df) in [(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while r > 0 && r < 7 && f > 0 && f < 7 {
            mask |= 1 << (r * 8 + f) as u32;
            r += dr; f += df;
        }
    }
    mask
}

fn rook_post_mask(sq: usize) -> u64 {
    let (rank, file) = (sq / 8, sq % 8);
    let mut mask = 0u64;
    for i in 0..8usize {
        if i != rank { mask |= 1 << (i * 8 + file); }
        if i != file { mask |= 1 << (rank * 8 + i); }
    }
    mask
}

fn bishop_post_mask(sq: usize) -> u64 {
    let (rank, file) = ((sq / 8) as i8, (sq % 8) as i8);
    let mut mask = 0u64;
    for (dr, df) in [(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while (0..8).contains(&r) && (0..8).contains(&f) {
            mask |= 1 << (r * 8 + f) as u32;
            r += dr; f += df;
        }
    }
    mask
}

const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    12, 11, 11, 11, 11, 11, 11, 12,
];

const BISHOP_BITS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6,
    5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
    6, 5, 5, 5, 5, 5, 5, 6,
];

fn build_table(
    bits: &[u8; 64],
    blocker_fn: fn(usize) -> u64,
    post_fn: fn(usize) -> u64,
    attack_fn: fn(usize, u64) -> u64,
) -> Vec<u16> {
    let total: usize = bits.iter().map(|&b| 1usize << b).sum();
    let mut table = vec![0u16; total];
    let mut offset = 0usize;
    for sq in 0..64 {
        let blocker_mask = blocker_fn(sq);
        let post_mask = post_fn(sq);
        let n = 1usize << bits[sq];
        for i in 0..n {
            let subset     = sw_pdep(i as u64, blocker_mask);
            let attacks    = attack_fn(sq, subset);
            let compressed = sw_pext(attacks, post_mask) as u16;
            table[offset + i] = compressed;
        }
        offset += n;
    }
    table
}

fn write_table(path: &str, table: &[u16]) {
    let mut f = std::fs::File::create(path).unwrap();
    write!(f, "[").unwrap();
    for (i, &v) in table.iter().enumerate() {
        if i % 32 == 0 { writeln!(f).unwrap(); }
        write!(f, "{v},").unwrap();
    }
    writeln!(f, "]").unwrap();
}

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let rook_table   = build_table(&ROOK_BITS,   rook_blocker_mask,   rook_post_mask,   rook_attacks);
    let bishop_table = build_table(&BISHOP_BITS, bishop_blocker_mask, bishop_post_mask, bishop_attacks);

    write_table(&format!("{out_dir}/rook_attacks_flat.rs"),   &rook_table);
    write_table(&format!("{out_dir}/bishop_attacks_flat.rs"), &bishop_table);

    println!("cargo:rerun-if-changed=build.rs");
}
