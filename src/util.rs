// Contains useful functions used in various parts of the program

use crate::bitboard::Square;

/// If a file has a 1, the entire file is marked with a 1
pub fn populate_files(bb: u64) -> u64 {
    populate_files_up(bb) | populate_files_down(bb)
}

/// Populates files only on higher ranks
pub fn populate_files_up(mut bb: u64) -> u64 {
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;
    bb
}

/// Populates files only going down
pub fn populate_files_down(mut bb: u64) -> u64 {
    bb |= bb >> 8;
    bb |= bb >> 16;
    bb |= bb >> 32;
    bb
}

/// Returns the given number, only keeping the least significant bit
#[inline]
pub fn lsb(number: u64) -> u64 {
    number & (!number + 1)
}

/// Returns the index of the least significant bit of the given number
/// Returns 0 if given 0
#[inline]
pub fn lsb_index(number: u64) -> u8 {
    // Get only the last bit and send it to the pow2 method
    lsb_index_pow2(lsb(number))
}

// Used for the method below
const LSB_POW2_TABLE: [u8; 64] = [
     0,  1, 48,  2, 57, 49, 28,  3,
    61, 58, 50, 42, 38, 29, 17,  4,
    62, 55, 59, 36, 53, 51, 43, 22,
    45, 39, 33, 30, 24, 18, 12,  5,
    63, 47, 56, 27, 60, 41, 37, 16,
    54, 35, 52, 21, 44, 32, 23, 11,
    46, 26, 40, 15, 34, 20, 31, 10,
    25, 14, 19,  9, 13,  8,  7,  6
]; 
/// Returns the index of the set bit in the given power of 
/// Returns 0 if given 0
#[inline]
pub fn lsb_index_pow2(power_of_2: u64) -> u8 {
    LSB_POW2_TABLE[(power_of_2.wrapping_mul(0x03f79d71b4cb0a89)) as usize >> 58]
}

/// Useful iterator over the set bits of a given number
pub struct LSBIndexIter(u64);
impl Iterator for LSBIndexIter {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let lsb_bit = lsb(self.0 as u64);
            let out = lsb_index_pow2(lsb_bit);
            self.0 ^= lsb_bit;
            Some(unsafe { std::mem::transmute(out) })
        }
    }
}

/// Returns an iterator over the indices of the set bits in a bitboard
pub fn squares(bitboard: u64) -> LSBIndexIter { LSBIndexIter(bitboard).into_iter() }

/// Returns an iterator over all of the squares
pub fn all_squares() -> AllSquaresIterator { AllSquaresIterator(0).into_iter() }

pub struct AllSquaresIterator(u8);
impl Iterator for AllSquaresIterator {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 < 64 {
            let out = self.0;
            self.0 += 1;
            Some(unsafe { std::mem::transmute(out) })
        } else {
            None
        }
    }
}

/// Returns a string representation of a bitboard with rank and file labels
pub fn format_bitboard(bitboard: u64) -> String {
    let file_bar = "  a b c d e f g h";
    let mut out = String::from(file_bar);
    out.push('\n');
    for rank in (0..8u8).rev() {
        out.push_str(&format!("{} ", rank + 1));
        for file in 0..8u8 {
            let bit = (bitboard >> (rank * 8 + file)) & 1;
            out.push(if bit == 1 { '1' } else { '.' });
            if file < 7 { out.push(' '); }
        }
        out.push_str(&format!(" {}\n", rank + 1));
    }
    out.push_str(file_bar);
    out
}