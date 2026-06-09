/*
 Contains tables used to quickly access/calculate attack bitboards for 
 individual, different pieces
*/

use once_cell::sync::Lazy;
use rand::{Rng, RngCore};

use crate::{bitboard::{Board, Side, Square}, util::all_squares};
// =============================================================================
//                              LEAPER PIECES
// =============================================================================

/// Table of precomputed pawn attacks, indexed by `Side` and `Square`
pub static PAWN_ATTACKS: Lazy<[[u64; 64]; 2]> = Lazy::new(|| {
    let mut table = [[0; 64]; 2];
    for square in all_squares() {
        let bitboard = 1 << square as u8;

        table[Side::White as usize][square as usize] |= (bitboard & !Board::A_FILE) << 7;
        table[Side::White as usize][square as usize] |= (bitboard & !Board::H_FILE) << 9;

        table[Side::Black as usize][square as usize] |= (bitboard & !Board::A_FILE) >> 9;
        table[Side::Black as usize][square as usize] |= (bitboard & !Board::H_FILE) >> 7;
    }
    table
});

/// Table of precomputed knight attacks, indexed by `Square`
pub static KNIGHT_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let mut attacks = 0;
        let bitboard = 1 << square as u8;
        // Move left by 1
        attacks |= (bitboard & !Board::A_FILE) << 15;
        attacks |= (bitboard & !Board::A_FILE) >> 17;
        // Move left by 2
        attacks |= (bitboard & !(Board::A_FILE | Board::B_FILE)) << 6;
        attacks |= (bitboard & !(Board::A_FILE | Board::B_FILE)) >> 10;
        // Move right by 1
        attacks |= (bitboard & !Board::H_FILE) << 17;
        attacks |= (bitboard & !Board::H_FILE) >> 15;
        // Move right by 2
        attacks |= (bitboard & !(Board::H_FILE | Board::G_FILE)) << 10;
        attacks |= (bitboard & !(Board::H_FILE | Board::G_FILE)) >> 6;
        
        table[square as usize] = attacks;
    }
    table
});

/// Table of precomputed king attacks, indexed by `Square`
pub static KING_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let mut attacks = 0;
        let bitboard = 1 << square as u8;
        // vertical attacks
        attacks |= bitboard << 8;
        attacks |= bitboard >> 8;
        // left attacks
        attacks |= (bitboard & !Board::A_FILE) >> 1;
        attacks |= (bitboard & !Board::A_FILE) >> 9;
        attacks |= (bitboard & !Board::A_FILE) << 7;
        // right attacks
        attacks |= (bitboard & !Board::H_FILE) << 1;
        attacks |= (bitboard & !Board::H_FILE) << 9;
        attacks |= (bitboard & !Board::H_FILE) >> 7;
        
        table[square as usize] = attacks;
    }
    table
});

// =============================================================================
//                      SLIDER PIECES (MAGIC BITBOARDS)
// =============================================================================

/// Rook attack table, indexed by `Square` and the computed index using the
/// corresponding magic value.
pub static ROOK_ATTACKS: Lazy<[Vec<u64>; 64]> = Lazy::new(|| {
    let mut table = std::array::from_fn(|_| Vec::new());
    for square in all_squares() {
        let current_table = try_create_rook_magic_table(square, ROOK_MAGIC[square as usize]);
        table[square as usize] = current_table.expect(
            &format!("Precomputed magic value for rook at {} is incorrect", square.to_unicode())
        );
    }
    table
});

/// Bishop attack table, indexed by `Square` and the computed index using the
/// corresponding magic value.
pub static BISHOP_ATTACKS: Lazy<[Vec<u64>; 64]> = Lazy::new(|| {
    let mut table = std::array::from_fn(|_| Vec::new());
    for square in all_squares() {
        let current_table = try_create_bishop_magic_table(square, BISHOP_MAGIC[square as usize]);
        table[square as usize] = current_table.expect(
            &format!("Precomputed magic value for bishop at {} is incorrect", square.to_unicode())
        );
    }
    table
});

/// Mask for board occupancy when calculating rook attacks.
/// The mask consists of the squares the rook would attack in an empty board,
/// not including the last squares
pub static ROOK_MASK: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        for r in 1..7 {
            if r == rank { continue; }
            let attacked_square = Square::from_rank_file(r, file);
            mask |= 1 << attacked_square as u8;
        }
        for f in 1..7 {
            if f == file { continue; }
            let attacked_square = Square::from_rank_file(rank, f);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});

/// Rook attacks on an empty bitboard
pub static ROOK_EMPTY_ATTAKCS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        for r in 0..8 {
            if r == rank { continue; }
            let attacked_square = Square::from_rank_file(r, file);
            mask |= 1 << attacked_square as u8;
        }
        for f in 0..8 {
            if f == file { continue; }
            let attacked_square = Square::from_rank_file(rank, f);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});

/// Mask for board occupancy when calculating bishop attacks.
/// The mask consists of the squares the bishop would attack in an empty board,
/// not including the last squares
pub static BISHOP_MASK: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        // antidiagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 + i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        // main diagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 - i;
            let f = file as i8 + i;
            if r <= 0 || r >= 7 || f <= 0 || f >= 7 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});

/// Bishop attacks on an empty bitboard
pub static BISHOP_EMPTY_ATTACKS: Lazy<[u64; 64]> = Lazy::new(|| {
    let mut table = [0; 64];
    for square in all_squares() {
        let (rank, file) = square.to_rank_file();
        let mut mask = 0;
        // antidiagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 + i;
            let f = file as i8 + i;
            if r < 0 || r >= 8 || f < 0 || f >= 8 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        // main diagonal
        for i in -8..8 {
            if i == 0 { continue; }
            let r = rank as i8 - i;
            let f = file as i8 + i;
            if r < 0 || r >= 8 || f < 0 || f >= 8 { continue; }
            let attacked_square = Square::from_rank_file(r as u8, f as u8);
            mask |= 1 << attacked_square as u8;
        }
        table[square as usize] = mask;
    }
    table
});

/// Number of bits for the index into rook's magic table.
/// Equivalent to the number of squares a rook can attack, excluding the last
/// squares
pub const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    12, 11, 11, 11, 11, 11, 11, 12,
];

/// Number of bits for the index into bishops's magic table.
/// Equivalent to the number of squares a bishop can attack, excluding the last
/// squares
pub const BISHOP_BITS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6,
    5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
    6, 5, 5, 5, 5, 5, 5, 6,
];

/// Magic numbers for the rook, indexed by `Square`.
/// Calculated with `compute_rook_magics()`
pub const ROOK_MAGIC: [u64; 64] = [
    0x0080001080400620,
    0x2240009000442000,
    0x2100110060004009,
    0x8680080010000480,
    0x1100100408000300,
    0x0280040080420001,
    0x8c00018804051006,
    0x0380082040800300,
    0x02488002a0804000,
    0x800980200482c000,
    0x000200220080c214,
    0x080100091000a100,
    0x0200800c00080080,
    0x0102800400020080,
    0x0304000108145002,
    0x1001000080420100,
    0x0201020021420080,
    0x8810014004a01041,
    0x0202808020041000,
    0x8010008028028010,
    0x40a0150008001100,
    0x0002008022040080,
    0x4006240045121048,
    0x1104020001804104,
    0x000080a080044000,
    0x062a200040100048,
    0x20110091002000c8,
    0x4004180080100081,
    0x8050910100080004,
    0x0802008200041018,
    0x0a01000100040200,
    0x040000860000c401,
    0x0420284000800080,
    0xc010022000400041,
    0x0470200282801000,
    0x0010040040400800,
    0x0004800400800800,
    0x4004201008014004,
    0x02d0100154008208,
    0x07304400820000c1,
    0x00d2804000208000,
    0x007004200240c000,
    0x9421004020010030,
    0x1001000a10010020,
    0x051008001101000c,
    0x8801000400090002,
    0x04008810010c0002,
    0xc001000084410002,
    0x4041418000610100,
    0x0000810240022900,
    0x0000420124108200,
    0x0000820800700180,
    0x3040800400280080,
    0x0002000410288200,
    0x00c50002000ca100,
    0x0000410c00804200,
    0xa4010121c0800411,
    0x2241020281a04092,
    0x0090800a001021c2,
    0xc000100008050021,
    0x4901002204100801,
    0x1002000870030402,
    0x002005021000a80c,
    0x9400044101239402,
];

/// Magic numbers for the bishop, indexed by `Square`.
/// Calculated with `compute_bishop_magics()`
pub const BISHOP_MAGIC: [u64; 64] = [
    0x006043042805c440,
    0xc10ca14801110180,
    0x0892440401229000,
    0x1204140094058200,
    0x41040c2200420288,
    0x0082080208000042,
    0x0004829010100694,
    0x8020240204842003,
    0x1010111210012200,
    0x1000280284820206,
    0x28005000a202d001,
    0x40c0482600402000,
    0x0580040421020201,
    0x8010410442401800,
    0x0120210101202210,
    0x0080010041042010,
    0x0208802048010802,
    0x0004108830008a00,
    0x8004004800401200,
    0x040d000804110110,
    0x0026000401210041,
    0x0002000101008240,
    0x01110020ac012003,
    0x0c00810024110800,
    0x113612e220600a00,
    0x4383100004040804,
    0x0082080044444400,
    0x1464080000202040,
    0x0081010000104002,
    0x0081908008080400,
    0x0888030002028243,
    0x010c008101422880,
    0x810c100c00400401,
    0xc00a211501103000,
    0x01004c0200900860,
    0x4081420080080081,
    0x0210120080001004,
    0x090a08020001c048,
    0x1110010061820200,
    0x8201024200038200,
    0x408805080800a088,
    0x0805084804412207,
    0x0500220022011000,
    0x4040820204200200,
    0x100124100c000180,
    0x4023021806000041,
    0x0620011410909100,
    0x0002068602100088,
    0x8001080390490400,
    0x8441840508420820,
    0x1800004a00900c40,
    0x0800018842020004,
    0x0000000810240004,
    0x1a84182808282420,
    0x008c61084a088011,
    0x241005080102c204,
    0x0000808800c22000,
    0x20980201008e1004,
    0x004010a108511000,
    0xc20048050c840400,
    0x0081000418030400,
    0x0000484421041100,
    0x41014a0208420400,
    0x4140182100c08900,
];

/// Returns the bitboards of squares a rook can attack from a given square,
/// given the occupancy of the other pieces. Used only to calculate magic 
/// numbers
fn rook_attacks_with_occupancy(square: Square, occupancy: u64) -> u64 {
    let mut attacks = 0;
    let (rank, file) = square.to_rank_file();
    // Attack upwards
    for r in (rank + 1)..8 {
        let attacked_square = Square::from_rank_file(r, file);
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
            break;
        }
    }
    // Attack downwards
    for r in (0..rank).rev() {
        let attacked_square = Square::from_rank_file(r, file);
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
            break;
        }
    }
    // Attack rightwards
    for f in (file + 1)..8 {
        let attacked_square = Square::from_rank_file(rank, f);
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
            break;
        }
    }
    // Attack leftwards
    for f in (0..file).rev() {
        let attacked_square = Square::from_rank_file(rank, f);
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
            break;
        }
    }
    attacks
}

/// Returns the bitboards of squares a bishop can attack from a given square,
/// given the occupancy of the other pieces. Used only to calculate magic 
/// numbers
fn bishop_attacks_with_occupancy(square: Square, occupancy: u64) -> u64 {
    let mut attacks = 0;
    let (rank, file) = square.to_rank_file();
    // Attack up and right
    for i in 1..8 {
        let r = rank + i;
        let f = file + i;
        if r >= 8 || f >= 8 {
            break;
        }
        let attacked_square = Square::from_rank_file(r, f);
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
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
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
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
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
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
        attacks |= 1 << attacked_square as u8;
        if occupancy & (1 << attacked_square as u8) != 0{
            break;
        }
    }
    attacks
}

/// Attempts to create a magic table for a rook at a given square with the
/// given magic number
fn try_create_rook_magic_table(square: Square, magic: u64) -> Option<Vec<u64>> {
    let bits = ROOK_BITS[square as usize];
    let mut table = vec![0; 1 << bits];
    let mask = ROOK_MASK[square as usize];
    for subset in mask_subsets(mask) {
        let attacks = rook_attacks_with_occupancy(square, subset);
        let index = subset.wrapping_mul(magic) >> (64 - bits);
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
fn try_create_bishop_magic_table(square: Square, magic: u64) -> Option<Vec<u64>> {
    let bits = BISHOP_BITS[square as usize];
    let mut table = vec![0; 1 << bits];
    let mask = BISHOP_MASK[square as usize];
    for subset in mask_subsets(mask) {
        let attacks = bishop_attacks_with_occupancy(square, subset);
        let index = subset.wrapping_mul(magic) >> (64 - bits);
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
    for square in all_squares() {
        for _ in 0..999999 {
            let candidate = magic_candidate(&mut rng);
            let table = try_create_rook_magic_table(square, candidate);
            if table.is_some() {
                magics_table[square as usize] = candidate;
                break;
            }
        }
        if magics_table[square as usize] == 0 {
            panic!("Failed to find a magic number for rook at square {}", square.to_unicode());
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
    for square in all_squares() {
        for _ in 0..999999 {
            let candidate = magic_candidate(&mut rng);
            let table = try_create_bishop_magic_table(square, candidate);
            if table.is_some() {
                magics_table[square as usize] = candidate;
                break;
            }
        }
        if magics_table[square as usize] == 0 {
            panic!("Failed to find a magic number for bishop at square {}", square.to_unicode());
        }
    }
    magics_table
}

/// Returns a vector containing all subsets of a given mask. Used for magic
/// table creation.
/// For example 0101 -> [0000, 0001, 0100, 0101]
fn mask_subsets(mask: u64) -> Vec<u64> {
    let mut bit_indices = Vec::new();
    for i in 0..64 {
        if mask & (1 << i) != 0 {
            bit_indices.push(i);
        }
    }
    let mut subsets = Vec::with_capacity(1 << bit_indices.len());
    for pattern in 0..(1 << bit_indices.len()) {
        let mut subset = 0;
        for i in 0..bit_indices.len() {
            if pattern & (1 << i) != 0 {
                subset |= 1 << bit_indices[i];
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
    use crate::util::all_squares;

    #[test]
    fn slider_attacks_no_panic() {
        for square in all_squares() {
            let rook_mask = ROOK_MASK[square as usize];
            let bishop_mask = BISHOP_MASK[square as usize];
            for &occ in &[0u64, rook_mask] {
                let idx = occ.wrapping_mul(ROOK_MAGIC[square as usize]) >> (64 - ROOK_BITS[square as usize]);
                let _ = ROOK_ATTACKS[square as usize][idx as usize];
            }
            for &occ in &[0u64, bishop_mask] {
                let idx = occ.wrapping_mul(BISHOP_MAGIC[square as usize]) >> (64 - BISHOP_BITS[square as usize]);
                let _ = BISHOP_ATTACKS[square as usize][idx as usize];
            }
        }
    }

    #[test]
    fn compute_and_print_magics() {
        println!("Rook magics:");
        let rook_magics = compute_rook_magics();
        for square in all_squares() {
            println!("  {:3}: 0x{:016x}", square.to_fen(), rook_magics[square as usize]);
        }
        println!("\nBishop magics:");
        let bishop_magics = compute_bishop_magics();
        for square in all_squares() {
            println!("  {:3}: 0x{:016x}", square.to_fen(), bishop_magics[square as usize]);
        }
    }
}