use once_cell::sync::Lazy;
use rand::{Rng, RngCore};

use crate::{movegen::attacks::knight_attacks, repr::{bitboard::BB, board::Board, square::Square}};

/// Table of knight attacks
pub static KNIGHT_ATTACKS: Lazy<[BB; 64]> = Lazy::new(|| {
    let mut table = [BB::new(0); 64];
    for square in Square::all() {
        table[square as usize] = knight_attacks(square.bb());
    }
    table
});

/// Table of king attacks
pub static KING_ATTACKS: Lazy<[BB; 64]> = Lazy::new(|| {
    let mut table = [BB::new(0); 64];
    for square in Square::all() {
        let king = square.bb();
        table[square as usize] |= (king & !Board::A_FILE) >> 1;
        table[square as usize] |= (king & !Board::A_FILE) << 7;
        table[square as usize] |= (king & !Board::A_FILE) >> 9;
        table[square as usize] |= king << 8;
        table[square as usize] |= king >> 8;
        table[square as usize] |= (king & !Board::H_FILE) << 1;
        table[square as usize] |= (king & !Board::H_FILE) << 9;
        table[square as usize] |= (king & !Board::H_FILE) >> 7;
    }
    table
});

/// Rook attack table, indexed by `Square` and the computed index using the
/// corresponding magic value.
pub static ROOK_ATTACKS: Lazy<[Vec<BB>; 64]> = Lazy::new(|| {
    let mut table = std::array::from_fn(|_| Vec::new());
    for square in Square::all() {
        let current_table = try_create_rook_magic_table(square, ROOK_MAGIC[square as usize]);
        table[square as usize] = current_table.expect(
            &format!("Precomputed magic value for rook at {} is incorrect", square.to_fen())
        );
    }
    table
});

/// Bishop attack table, indexed by `Square` and the computed index using the
/// corresponding magic value.
pub static BISHOP_ATTACKS: Lazy<[Vec<BB>; 64]> = Lazy::new(|| {
    let mut table = std::array::from_fn(|_| Vec::new());
    for square in Square::all() {
        let current_table = try_create_bishop_magic_table(square, BISHOP_MAGIC[square as usize]);
        table[square as usize] = current_table.expect(
            &format!("Precomputed magic value for bishop at {} is incorrect", square.to_fen())
        );
    }
    table
});

/// Rook mask for magic bitboard calculation
pub static ROOK_MASK: Lazy<[BB; 64]> = Lazy::new(|| {
    let mut table = [BB::new(0); 64];
    for square in Square::all() {
        let (rank, file) = square.rank_file();
        let mut mask = BB::new(0);
        for r in 1..7 {
            if r == rank { continue; }
            let attacked_square = Square::from_rank_file(r, file);
            mask |= attacked_square
        }
        for f in 1..7 {
            if f == file { continue; }
            let attacked_square = Square::from_rank_file(rank, f);
            mask |= attacked_square
        }
        table[square as usize] = mask;
    }
    table
});

/// Bishop mask for magic bitboard calculation
pub static BISHOP_MASK: Lazy<[BB; 64]> = Lazy::new(|| {
    let mut table = [BB::new(0); 64];
    for square in Square::all() {
        let (rank, file) = square.rank_file();
        let mut mask = BB::new(0);
        // antidiagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 + i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= attacked_square;
        }
        // main diagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 - i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= attacked_square;
        }
        table[square as usize] = mask;
    }
    table
});

/// Number of bits for the index into rook's magic table
pub const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    10, 09, 09, 09, 09, 09, 09, 10,
    11, 10, 10, 10, 10, 11, 10, 11,
];

/// Number of bits for the index into bishops's magic table
pub const BISHOP_BITS: [u8; 64] = [
    5, 4, 5, 5, 5, 5, 4, 5,
    4, 4, 5, 5, 5, 5, 4, 4,
    4, 4, 7, 7, 7, 7, 4, 4,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    4, 4, 7, 7, 7, 7, 4, 4,
    4, 4, 5, 5, 5, 5, 4, 4,
    5, 4, 5, 5, 5, 5, 4, 5,
];

/// Magic numbers for the rook, indexed by `Square`.
/// Calculated with `compute_rook_magics()` and from
/// https://www.chessprogramming.org/Best_Magics_so_far
pub const ROOK_MAGIC: [u64; 64] = [
    0x0080001080400620,  // a1
    0x2240009000442000,
    0x2100110060004009,
    0x8680080010000480,
    0x1100100408000300,
    0x0280040080420001,
    0x8c00018804051006,
    0x0380082040800300,
    0x02488002a0804000,  // a2
    0x800980200482c000,
    0x000200220080c214,
    0x080100091000a100,
    0x0200800c00080080,
    0x0102800400020080,
    0x0304000108145002,
    0x1001000080420100,
    0x0201020021420080,  // a3
    0x8810014004a01041,
    0x0202808020041000,
    0x8010008028028010,
    0x40a0150008001100,
    0x0002008022040080,
    0x4006240045121048,
    0x1104020001804104,
    0x000080a080044000,  // a4
    0x062a200040100048,
    0x20110091002000c8,
    0x4004180080100081,
    0x8050910100080004,
    0x0802008200041018,
    0x0a01000100040200,
    0x040000860000c401,
    0x0420284000800080,  // a5
    0xc010022000400041,
    0x0470200282801000,
    0x0010040040400800,
    0x0004800400800800,
    0x4004201008014004,
    0x02d0100154008208,
    0x07304400820000c1,
    0x00d2804000208000,  // a6
    0x007004200240c000,
    0x9421004020010030,
    0x1001000a10010020,
    0x051008001101000c,
    0x8801000400090002,
    0x04008810010c0002,
    0xc001000084410002,
    0x48FFFE99FECFAA00,  // a7
    0x48FFFE99FECFAA00,
    0x497FFFADFF9C2E00,
    0x613FFFDDFFCE9200,
    0xffffffe9ffe7ce00,
    0xfffffff5fff3e600,
    0x0003ff95e5e6a4c0,
    0x510FFFF5F63C96A0,
    0xEBFFFFB9FF9FC526,  // a8
    0x61FFFEDDFEEDAEAE,
    0x53BFFFEDFFDEB1A2,
    0x127FFFB9FFDFB5F6,
    0x411FFFDDFFDBF4D6,
    0x1002000870030402,
    0x0003ffef27eebe74,
    0x7645FFFECBFEA79E,
];

/// Magic numbers for the bishop, indexed by `Square`.
/// Calculated with `compute_bishop_magics()` and from
/// https://www.chessprogramming.org/Best_Magics_so_far
pub const BISHOP_MAGIC: [u64; 64] = [
    0xffedf9fd7cfcffff,  // a1
    0xfc0962854a77f576,
    0x0892440401229000,
    0x1204140094058200,
    0x41040c2200420288,
    0x0082080208000042,
    0xfc0a66c64a7ef576,
    0x7ffdfdfcbd79ffff,
    0xfc0846a64a34fff6,  // a2
    0xfc087a874a3cf7f6,
    0x28005000a202d001,
    0x40c0482600402000,
    0x0580040421020201,
    0x8010410442401800,
    0xfc0864ae59b4ff76,
    0x3c0860af4b35ff76,
    0x73C01AF56CF4CFFB,  // a3
    0x41A01CFAD64AAFFC,
    0x8004004800401200,
    0x040d000804110110,
    0x0026000401210041,
    0x0002000101008240,
    0x7c0c028f5b34ff76,
    0xfc0a028e5ab4df76,
    0x113612e220600a00,  // a4
    0x4383100004040804,
    0x0082080044444400,
    0x1464080000202040,
    0x0081010000104002,
    0x0081908008080400,
    0x0888030002028243,
    0x010c008101422880,
    0x810c100c00400401,  // a5
    0xc00a211501103000,
    0x01004c0200900860,
    0x4081420080080081,
    0x0210120080001004,
    0x090a08020001c048,
    0x1110010061820200,
    0x8201024200038200,
    0xDCEFD9B54BFCC09F,  // a6
    0xF95FFA765AFD602B,
    0x0500220022011000,
    0x4040820204200200,
    0x100124100c000180,
    0x4023021806000041,
    0x43ff9a5cf4ca0c01,
    0x4BFFCD8E7C587601,
    0xfc0ff2865334f576,  // a7
    0xfc0bf6ce5924f576,
    0x1800004a00900c40,
    0x0800018842020004,
    0x0000000810240004,
    0x1a84182808282420,
    0xc3ffb7dc36ca8c89,
    0xc3ff8a54f4ca2c89,
    0xfffffcfcfd79edff,  // a8
    0xfc0863fccb147576,
    0x004010a108511000,
    0xc20048050c840400,
    0x0081000418030400,
    0x0000484421041100,
    0xfc087e8e4bb2f736,
    0x43ff9e4ef4ca2c89,
];

/// Returns the bitboards of squares a rook can attack from a given square,
/// given the occupancy of the other pieces. Used only to calculate magic 
/// numbers
pub fn rook_attacks_with_occupancy(square: Square, occupancy: BB) -> BB {
    let mut attacks = BB::new(0);
    let (rank, file) = square.rank_file();
    // Attack upwards
    for r in (rank + 1)..8 {
        let attacked_square = Square::from_rank_file(r, file);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack downwards
    for r in (0..rank).rev() {
        let attacked_square = Square::from_rank_file(r, file);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack rightwards
    for f in (file + 1)..8 {
        let attacked_square = Square::from_rank_file(rank, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack leftwards
    for f in (0..file).rev() {
        let attacked_square = Square::from_rank_file(rank, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    attacks
}

/// Returns the bitboards of squares a bishop can attack from a given square,
/// given the occupancy of the other pieces. Used only to calculate magic 
/// numbers
pub fn bishop_attacks_with_occupancy(square: Square, occupancy: BB) -> BB {
    let mut attacks = BB::new(0);
    let (rank, file) = square.rank_file();
    // Attack up and right
    for i in 1..8 {
        let r = rank + i;
        let f = file + i;
        if r >= 8 || f >= 8 {
            break;
        }
        let attacked_square = Square::from_rank_file(r, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack down and left
    for i in 1..8 {
        if i > rank || i > file {
            break;
        }
        let r = rank - i;
        let f = file - i;
        let attacked_square = Square::from_rank_file(r, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack right and down
    for i in 1..8 {
        if i > rank {
            break;
        }
        let r = rank - i;
        let f = file + i;
        if f >= 8 {
            break;
        }
        let attacked_square = Square::from_rank_file(r, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    // Attack up and left
    for i in 1..8 {
        if i > file {
            break;
        }
        let r = rank + i;
        let f = file - i;
        if r >= 8 {
            break;
        }
        let attacked_square = Square::from_rank_file(r, f);
        attacks |= attacked_square;
        if occupancy & attacked_square != 0{
            break;
        }
    }
    attacks
}

/// Attempts to create a magic table for a rook at a given square with the
/// given magic number
fn try_create_rook_magic_table(square: Square, magic: u64) -> Option<Vec<BB>> {
    let bits = ROOK_BITS[square as usize];
    let mut table = vec![BB::new(0); 1 << bits];
    let mask = ROOK_MASK[square as usize];
    for subset in mask_subsets(mask) {
        let attacks = rook_attacks_with_occupancy(square, subset);
        let index = subset.0.wrapping_mul(magic) >> (64 - bits);
        if table[index as usize] == 0 {
            table[index as usize] = attacks;
        } else if table[index as usize] != attacks {
            return None;
        }
    }
    table.shrink_to_fit();
    Some(table)
}

/// Attempts to create a magic table for a bishop at a given square with the
/// given magic number
fn try_create_bishop_magic_table(square: Square, magic: u64) -> Option<Vec<BB>> {
    let bits = BISHOP_BITS[square as usize];
    let mut table = vec![BB::new(0); 1 << bits];
    let mask = BISHOP_MASK[square as usize];
    for subset in mask_subsets(mask) {
        let attacks = bishop_attacks_with_occupancy(square, subset);
        let index = subset.0.wrapping_mul(magic) >> (64 - bits);
        if table[index as usize] == 0 {
            table[index as usize] = attacks;
        } else if table[index as usize] != attacks {
            return None;
        }
    }
    table.shrink_to_fit();
    Some(table)
}

/// Finds a set of working magic numbers for the rook, or panics if it fails.
/// Only used to precompute the magic numbers used in the tables above.
#[allow(dead_code)]
fn compute_rook_magics() -> [u64; 64] {
    let mut rng = rand::rng();
    let mut magics_table = [0; 64];
    for square in Square::all() {
        for _ in 0..999999 {
            let candidate = magic_candidate(&mut rng);
            let table = try_create_rook_magic_table(square, candidate);
            if table.is_some() {
                magics_table[square as usize] = candidate;
                break;
            }
        }
        if magics_table[square as usize] == 0 {
            panic!("Failed to find a magic number for rook at square {}", square.to_fen());
        }
    }
    magics_table
}

/// Finds a set of working magic numbers for the bishop, or panics if it fails.
/// Only used to precompute the magic numbers used in the tables above.
#[allow(dead_code)]
fn compute_bishop_magics() -> [u64; 64] {
    let mut rng = rand::rng();
    let mut magics_table = [0; 64];
    for square in Square::all() {
        for _ in 0..999999 {
            let candidate = magic_candidate(&mut rng);
            let table = try_create_bishop_magic_table(square, candidate);
            if table.is_some() {
                magics_table[square as usize] = candidate;
                break;
            }
        }
        if magics_table[square as usize] == 0 {
            panic!("Failed to find a magic number for bishop at square {}", square.to_fen());
        }
    }
    magics_table
}

/// Returns a vector containing all subsets of a given mask. Used for magic
/// table creation.
/// For example 0101 -> [0000, 0001, 0100, 0101]
pub fn mask_subsets(mask: BB) -> Vec<BB> {
    let mut bit_indices = Vec::new();
    for i in 0..64 {
        if mask.0 & (1 << i) != 0 {
            bit_indices.push(i);
        }
    }
    let mut subsets = Vec::with_capacity(1 << bit_indices.len());
    for pattern in 0..(1 << bit_indices.len()) {
        let mut subset = BB::new(0);
        for i in 0..bit_indices.len() {
            if pattern & (1 << i) != 0 {
                subset.0 |= 1 << bit_indices[i];
            }
        }
        subsets.push(subset);
    }
    subsets
}

/// Returns a candidate for a magic number
fn magic_candidate(rng: &mut dyn RngCore) -> u64 {
    rng.random::<u64>() & rng.random::<u64>() & rng.random::<u64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_and_print_magics() {
        println!("Rook magics:");
        let rook_magics = compute_rook_magics();
        for square in Square::all() {
            println!("  {:3}: 0x{:016x}", square.to_fen(), rook_magics[square as usize]);
        }
        println!("\nBishop magics:");
        let bishop_magics = compute_bishop_magics();
        for square in Square::all() {
            println!("  {:3}: 0x{:016x}", square.to_fen(), bishop_magics[square as usize]);
        }
    }

    #[test]
    fn magic_numbers() {
        ROOK_ATTACKS[0][0];
        BISHOP_ATTACKS[0][0];
    }
}