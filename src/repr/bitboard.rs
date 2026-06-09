use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr, ShrAssign};

use crate::{repr::square::Square, test_assert};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BB(u64);

impl BB {
    pub const fn new(bb: u64) -> Self {
        BB(bb)
    }

    /// Removes the least significant set bit from `self`
    /// and returns its corresponding `Square`
    pub fn pop_lsb(&mut self) -> Square {
        test_assert!(self != 0);
        let out = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        Square::from_u8(out)
    }

    /// Returns the number of ones in `self`
    pub fn count_ones(self) -> u8 {
        self.0.count_ones() as u8
    }

    /// Returns an iterator over the squares corresponding
    /// to the set bits of `self`
    pub fn squares(self) -> BBIter {
        BBIter(self).into_iter()
    }
}

struct BBIter(BB);

impl Iterator for BBIter {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        if (*self).0 == 0 {
            None
        } else {
            Some(self.0.pop_lsb())
        }
    }
}

impl BitOr for BB {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        BB(self.0 | rhs.0)
    }
}

impl BitOrAssign for BB {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitOr<Square> for BB {
    type Output = Self;
    fn bitor(self, rhs: Square) -> Self::Output {
        self | rhs.bb()
    }
}

impl BitOr<BB> for Square {
    type Output = BB;
    fn bitor(self, rhs: BB) -> Self::Output {
        self.bb() | rhs
    }
}

impl BitOrAssign<Square> for BB {
    fn bitor_assign(&mut self, rhs: Square) {
        *self |= rhs.bb()
    }
}

impl BitAnd for BB {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        BB(self.0 & rhs.0)
    }
}

impl BitAndAssign for BB {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitAnd<Square> for BB {
    type Output = Self;
    fn bitand(self, rhs: Square) -> Self::Output {
        self & rhs.bb()
    }
}

impl BitAnd<BB> for Square {
    type Output = BB;
    fn bitand(self, rhs: BB) -> Self::Output {
        self.bb() & rhs
    }
}

impl BitAndAssign<Square> for BB {
    fn bitand_assign(&mut self, rhs: Square) {
        *self &= rhs.bb()
    }
}

impl BitXor for BB {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        BB(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for BB {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl BitXor<Square> for BB {
    type Output = Self;
    fn bitxor(self, rhs: Square) -> Self::Output {
        self ^ rhs.bb()
    }
}

impl BitXor<BB> for Square {
    type Output = BB;
    fn bitxor(self, rhs: BB) -> Self::Output {
        self.bb() ^ rhs
    }
}

impl BitXorAssign<Square> for BB {
    fn bitxor_assign(&mut self, rhs: Square) {
        *self ^= rhs.bb()
    }
}

impl Not for BB {
    type Output = Self;
    fn not(self) -> Self::Output {
        BB(!self.0)
    }
}

impl PartialEq<u64> for BB {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<BB> for u64 {
    fn eq(&self, other: &BB) -> bool {
        *self == other.0
    }
}

impl Shl<u8> for BB {
    type Output = Self;
    fn shl(self, rhs: u8) -> Self::Output {
        BB(self.0 << rhs)
    }
}

impl ShlAssign<u8> for BB {
    fn shl_assign(&mut self, rhs: u8) {
        self.0 <<= rhs
    }
}

impl Shr<u8> for BB {
    type Output = Self;
    fn shr(self, rhs: u8) -> Self::Output {
        BB(self.0 >> rhs)
    }
}

impl ShrAssign<u8> for BB {
    fn shr_assign(&mut self, rhs: u8) {
        self.0 >>= rhs
    }
}