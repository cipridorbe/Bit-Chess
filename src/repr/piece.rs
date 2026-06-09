use crate::repr::colour::Colour;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Piece {
    WhitePawn,
    WhiteKnight,
    WhiteBishop,
    WhiteRook,
    WhiteQueen,
    WhiteKing,

    BlackPawn,
    BlackKnight,
    BlackBishop,
    BlackRook,
    BlackQueen,
    BlackKing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    Leaper,
    Slider,
}

impl Piece {
    pub const ALL: [Piece; 12] = unsafe {
        std::mem::transmute([
            0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
        ])
    };

    pub const PHASE_VALUES: [u8; 12] = [
        0, 1, 1, 2, 4, 0,
        0, 1, 1, 2, 4, 0,
    ];

    pub fn piece_type(self) -> PieceType {
        match self {
            Piece::WhitePawn | Piece::BlackPawn |
            Piece::WhiteKnight | Piece::BlackKnight |
            Piece::WhiteKing | Piece::BlackKing => {
                PieceType::Leaper
            },
            Piece::WhiteBishop | Piece::BlackBishop |
            Piece::WhiteRook | Piece::BlackRook |
            Piece::WhiteQueen | Piece::BlackQueen => {
                PieceType::Slider
            }
            
        }
    }

    pub fn colour(self) -> Colour {
        match self {
            Piece::WhitePawn
            | Piece::WhiteKnight
            | Piece::WhiteBishop
            | Piece::WhiteRook
            | Piece::WhiteQueen
            | Piece::WhiteKing => {
                Colour::White
            },
            Piece::BlackPawn
            | Piece::BlackKnight
            | Piece::BlackBishop
            | Piece::BlackRook
            | Piece::BlackQueen
            | Piece::BlackKing => {
                Colour::Black
            }
        }
    }

    pub fn phase_value(self) -> u8 {
        Piece::PHASE_VALUES[self as usize]
    }

    pub fn pawn(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhitePawn,
            Colour::Black => Piece::BlackPawn
        }
    }

    pub fn knight(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhiteKnight,
            Colour::Black => Piece::BlackKnight
        }
    }

    pub fn bishop(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhiteBishop,
            Colour::Black => Piece::BlackBishop
        }
    }

    pub fn rook(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhiteRook,
            Colour::Black => Piece::BlackRook
        }
    }

    pub fn queen(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhiteQueen,
            Colour::Black => Piece::BlackQueen
        }
    }

    pub fn king(colour: Colour) -> Self {
        match colour {
            Colour::White => Piece::WhiteKing,
            Colour::Black => Piece::BlackKing
        }
    }

    pub fn to_fen(self) -> String {
        match self {
            Piece::WhitePawn => "P",
            Piece::WhiteKnight => "N",
            Piece::WhiteBishop => "B",
            Piece::WhiteRook => "R",
            Piece::WhiteQueen => "Q",
            Piece::WhiteKing => "K",
            Piece::BlackPawn => "p",
            Piece::BlackKnight => "n",
            Piece::BlackBishop => "b",
            Piece::BlackRook => "r",
            Piece::BlackQueen => "q",
            Piece::BlackKing => "k",
        }.to_string()
    }

    pub fn from_fen(fen: &str) -> Self {
        match fen {
            "P" => Piece::WhitePawn,
            "N" => Piece::WhiteKnight,
            "B" => Piece::WhiteBishop,
            "R" => Piece::WhiteRook,
            "Q" => Piece::WhiteQueen,
            "K" => Piece::WhiteKing,
            "p" => Piece::BlackPawn,
            "n" => Piece::BlackKnight,
            "b" => Piece::BlackBishop,
            "r" => Piece::BlackRook,
            "q" => Piece::BlackQueen,
            "k" => Piece::BlackKing,
            _ => panic!("Unexpected piece fen {}", fen)
        }
    }
}