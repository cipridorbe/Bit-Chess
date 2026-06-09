use std::ops::Not;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Colour {
    White,
    Black
}

impl Not for Colour {
    type Output = Self;
    fn not(self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 ^ 1) }
    }
}