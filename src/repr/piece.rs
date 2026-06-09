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
}