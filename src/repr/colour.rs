use std::ops::Not;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Colour {
    White,
    Black
}

impl Colour {
    pub fn to_fen(self) -> String {
        match self {
            Colour::White => "w",
            Colour::Black => "b",
        }.to_string()
    }

    pub fn from_fen(fen: &str) -> Self {
        match fen {
            "w" => Colour::White,
            "b" => Colour::Black,
            _ => panic!("Invalid fen for colour {}", fen)
        }
    }
}

impl Not for Colour {
    type Output = Self;
    fn not(self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 ^ 1) }
    }
}