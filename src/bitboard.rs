/*
================================================================================
                                SQUARE
================================================================================
Square representation of bitboard.
00 is least significant bit and 63 is most significant bit.

8  56 57 58 59 60 61 62 63
7  48 49 50 51 52 53 54 55
6  40 41 42 43 44 45 46 47
5  32 33 34 35 36 37 38 39
4  24 25 26 27 28 29 30 31
3  16 17 18 19 20 21 22 23
2  08 09 10 11 12 13 14 15
1  00 01 02 03 04 05 06 07
   a  b  c  d  e  f  g  h

Note that rank and files are both 0-indexed.
*/

use crate::util::squares;

/// Square indices on bitboards.
/// For example 1 << Square::a1 is the mask for the a1 square. 
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Square {
    a1, b1, c1, d1, e1, f1, g1, h1,
    a2, b2, c2, d2, e2, f2, g2, h2,
    a3, b3, c3, d3, e3, f3, g3, h3,
    a4, b4, c4, d4, e4, f4, g4, h4,
    a5, b5, c5, d5, e5, f5, g5, h5,
    a6, b6, c6, d6, e6, f6, g6, h6,
    a7, b7, c7, d7, e7, f7, g7, h7,
    a8, b8, c8, d8, e8, f8, g8, h8,
}

impl Square {
    /// Converts the given rank and file into a square
    pub fn from_rank_file(rank: u8, file: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Square>(rank * 8 + file) }
    }

    /// Converts the given square into a rank and a file
    pub fn to_rank_file(self) -> (u8, u8) {
        let square = self as u8;
        (square / 8, square % 8)
    }

    /// Returns the rank of the given square
    pub fn rank(self) -> u8 {
        self as u8 / 8
    }

    /// Converts the given square into fen notation (equivalent to to_string in this case)
    pub fn to_fen(self) -> String {
        self.to_string()
    }

    /// Converts the given square into unicode for display (equivalent to to_string in this case)
    pub fn to_unicode(self) -> String {
        self.to_string()
    }

    /// Converts given fen into a square or None if "-"
    pub fn from_fen(fen: &str) -> Option<Self> {
        if fen == "-" { return None; }
        let mut chars = fen.chars();
        let file = chars.next().unwrap() as u8 - b'a';
        let rank = chars.next().unwrap() as u8 - b'1';
        Some(Square::from_rank_file(rank, file))
    }
}


impl ToString for Square {
    fn to_string(&self) -> String {
        let ranks = ['1', '2', '3', '4', '5', '6', '7', '8'];
        let files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        let (rank, file) = self.to_rank_file();
        format!("{}{}", files[file as usize], ranks[rank as usize])
    }
}

/*
================================================================================
                               PIECE
================================================================================
*/

#[derive(Clone, Copy, PartialEq, Eq)]
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

impl Piece {
    /// Array of all pieces
    pub const ALL: [Self; 12] = [
        Piece::WhitePawn,
        Piece::WhiteKnight,
        Piece::WhiteBishop,
        Piece::WhiteRook,
        Piece::WhiteQueen,
        Piece::WhiteKing,
        Piece::BlackPawn,
        Piece::BlackKnight,
        Piece::BlackBishop,
        Piece::BlackRook,
        Piece::BlackQueen,
        Piece::BlackKing,
    ];

    /// Array of all white pieces
    pub const WHITE: [Self; 6] = [
        Piece::WhitePawn,
        Piece::WhiteKnight,
        Piece::WhiteBishop,
        Piece::WhiteRook,
        Piece::WhiteQueen,
        Piece::WhiteKing,
    ];

    /// Array of all black pieces
    pub const BLACK: [Self; 6] = [
        Piece::BlackPawn,
        Piece::BlackKnight,
        Piece::BlackBishop,
        Piece::BlackRook,
        Piece::BlackQueen,
        Piece::BlackKing,
    ];

    pub const PAWNS:   [Self; 2] = [Piece::WhitePawn,   Piece::BlackPawn];
    pub const KNIGHTS: [Self; 2] = [Piece::WhiteKnight, Piece::BlackKnight];
    pub const BISHOPS: [Self; 2] = [Piece::WhiteBishop, Piece::BlackBishop];
    pub const ROOKS:   [Self; 2] = [Piece::WhiteRook,   Piece::BlackRook];
    pub const QUEENS:  [Self; 2] = [Piece::WhiteQueen,  Piece::BlackQueen];
    pub const KINGS:   [Self; 2] = [Piece::WhiteKing,   Piece::BlackKing];

    pub fn pawn(side: Side) -> Self { Piece::PAWNS[side as usize] }
    pub fn knight(side: Side) -> Self { Piece::KNIGHTS[side as usize] }
    pub fn bishop(side: Side) -> Self { Piece::BISHOPS[side as usize] }
    pub fn rook(side: Side) -> Self { Piece::ROOKS[side as usize] }
    pub fn queen(side: Side) -> Self { Piece::QUEENS[side as usize] }
    pub fn king(side: Side) -> Self { Piece::KINGS[side as usize] }
    pub fn of_side(side: Side) -> [Self; 6] { if side == Side::White { Piece::WHITE } else { Piece::BLACK }}

    pub fn side(self) -> Side {
        if self as u8 <= 5 {
            Side::White
        } else {
            Side::Black
        }
    }

    pub fn from_fen(s: &str) -> Self {
        match s {
            "P" => Piece::WhitePawn,   "p" => Piece::BlackPawn,
            "N" => Piece::WhiteKnight, "n" => Piece::BlackKnight,
            "B" => Piece::WhiteBishop, "b" => Piece::BlackBishop,
            "R" => Piece::WhiteRook,   "r" => Piece::BlackRook,
            "Q" => Piece::WhiteQueen,  "q" => Piece::BlackQueen,
            "K" => Piece::WhiteKing,   "k" => Piece::BlackKing,
            _ => panic!("invalid fen piece: {s}"),
        }
    }

    /// Returns a unicode representation of the piece
    pub fn to_unicode(self) -> String {
        match self {
            Piece::WhitePawn   => "♙",
            Piece::WhiteKnight => "♘",
            Piece::WhiteBishop => "♗",
            Piece::WhiteRook   => "♖",
            Piece::WhiteQueen  => "♕",
            Piece::WhiteKing   => "♔",
            Piece::BlackPawn   => "♟",
            Piece::BlackKnight => "♞",
            Piece::BlackBishop => "♝",
            Piece::BlackRook   => "♜",
            Piece::BlackQueen  => "♛",
            Piece::BlackKing   => "♚",
        }.to_string()
    }

    /// Returns an ascii representation of the piece
    pub fn to_ascii(self) -> String {
        match self {
            Piece::WhitePawn   => "P",
            Piece::WhiteKnight => "N",
            Piece::WhiteBishop => "B",
            Piece::WhiteRook   => "R",
            Piece::WhiteQueen  => "Q",
            Piece::WhiteKing   => "K",
            Piece::BlackPawn   => "p",
            Piece::BlackKnight => "n",
            Piece::BlackBishop => "b",
            Piece::BlackRook   => "r",
            Piece::BlackQueen  => "q",
            Piece::BlackKing   => "k",
        }.to_string()
    }

    /// Returns a fen representation of the piece
    pub fn to_fen(self) -> String {
        self.to_ascii()
    }
}

/*
================================================================================
                               Side
================================================================================
*/

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    White,
    Black
}

impl Side {
    pub fn from_fen(s: &str) -> Self {
        match s {
            "w" => Side::White,
            "b" => Side::Black,
            _ => panic!("invalid fen side: {s}"),
        }
    }

    pub fn to_fen(self) -> String {
        match self {
            Side::White => "w",
            Side::Black => "b"
        }.to_string()
    }

    pub fn to_ascii(self) -> String {
        match self {
            Side::White => "white",
            Side::Black => "black",
        }.to_string()
    }

    pub fn other(self) -> Side {
        unsafe { std::mem::transmute(1 ^ (self as u8)) }
    }
}

/*
================================================================================
                               Board
================================================================================
*/

pub struct Board {
    /// Bitboards for each piece, indexed by `Piece`
    pub(crate) pieces: [u64; 12],

    /// Bitboards for all pieces of each side, indexed by `Side`
    pub(crate) sides: [u64; 2],

    /// Bitboard for global occupancy (all pieces)
    pub(crate) occupied: u64,

    /// The player to play in current turn
    pub(crate) side: Side,

    /// Castling information, stored as 0000 bq bk wq wk
    pub(crate) castling: u8,

    /// The square that can be captured by en passant, if any
    pub(crate) enpassant: Option<Square>,

    /// Number of halfmoves since last pawn move or captured, used for 50 move rule
    pub(crate) halfmoves: u8,

    /// Number of full moves since start of the game
    pub(crate) fullmoves: u8
}

impl Board {

    pub const A_FILE: u64 = 0x0101010101010101;
    pub const B_FILE: u64 = 0x0202020202020202;
    pub const G_FILE: u64 = 0x4040404040404040;
    pub const H_FILE: u64 = 0x8080808080808080;

    pub const RANK_1: u64 = 0x00000000000000ff;
    pub const RANK_2: u64 = 0x000000000000ff00;
    pub const RANK_7: u64 = 0x00ff000000000000;
    pub const RANK_8: u64 = 0xff00000000000000;

    pub const BLACK_QUEEN_CASTLE: u8 = 0b1000;
    pub const BLACK_KING_CASTLE:  u8 = 0b0100;
    pub const WHITE_QUEEN_CASTLE: u8 = 0b0010;
    pub const WHITE_KING_CASTLE:  u8 = 0b0001;

    /// Returns the starting position of a regular game of chess
    pub fn starting_position() -> Self {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    /// Returns the piece at the given square, or None if empty.
    /// Note: this method is slow. Should not be used in engine.
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        for piece in Piece::ALL {
            if self.pieces[piece as usize] & (1 << square as u8) != 0{
                return Some(piece);
            }
        }
        return None;
    }

    /// Returns the queenside/kingside castling rights for the given side
    pub fn castling_rights(&self, side: Side) -> (bool, bool) {
        let (queen, king) = match side {
            Side::White => (Board::WHITE_QUEEN_CASTLE, Board::WHITE_KING_CASTLE),
            Side::Black => (Board::BLACK_QUEEN_CASTLE, Board::BLACK_KING_CASTLE),
        };
        (self.castling & queen != 0, self.castling & king != 0)
    }

    /// Converts the current board to an array of `Option<Piece>`
    pub fn to_piece_array(&self) -> [[Option<Piece>; 8]; 8] {
        let mut out = [[None; 8]; 8];
        for piece in Piece::ALL {
            for square in squares(self.pieces[piece as usize]) {
                let (row, col) = square.to_rank_file();
                out[row as usize][col as usize] = Some(piece);
            }
        }
        out
    }

    /// Converts the given piece array into piece bitboard occupancy
    pub fn from_piece_array(arr: &[[Option<Piece>; 8]; 8]) -> [u64; 12] {
        let mut out = [0; 12];
        for row in 0..8 {
            for col in 0.. 8 {
                if let Some(piece) = arr[row][col] {
                    let square = Square::from_rank_file(row as u8, col as u8);
                    out[piece as usize] |= 1 << square as u8;
                }
            }
        }
        out
    }

    fn format_board(&self, piece_fn: impl Fn(Piece) -> String, empty: char) -> String {
        let arr = self.to_piece_array();
        let file_bar = "  a b c d e f g h";
        let mut out = String::from(file_bar);
        out.push('\n');

        for rank in (0..8usize).rev() {
            out.push_str(&format!("{} ", rank + 1));
            for file in 0..8usize {
                match arr[rank][file] {
                    Some(piece) => out.push_str(&piece_fn(piece)),
                    None        => out.push(empty),
                }
                if file < 7 { out.push(' '); }
            }
            out.push_str(&format!(" {}\n", rank + 1));
        }

        out.push_str(file_bar);

        let mut castling = String::new();
        if self.castling & Board::WHITE_KING_CASTLE  != 0 { castling.push('K'); }
        if self.castling & Board::WHITE_QUEEN_CASTLE != 0 { castling.push('Q'); }
        if self.castling & Board::BLACK_KING_CASTLE  != 0 { castling.push('k'); }
        if self.castling & Board::BLACK_QUEEN_CASTLE != 0 { castling.push('q'); }
        if castling.is_empty()         { castling.push('-'); }

        let enpassant = match self.enpassant {
            Some(sq) => sq.to_fen(),
            None     => "-".to_string(),
        };

        out.push_str(&format!("\n\nSide:       {}", self.side.to_ascii()));
        out.push_str(&format!("\nCastling:   {}", castling));
        out.push_str(&format!("\nEn passant: {}", enpassant));
        out.push_str(&format!("\nHalf moves: {}", self.halfmoves));
        out.push_str(&format!("\nFull moves: {}", self.fullmoves));

        out
    }

    pub fn to_ascii(&self) -> String {
        self.format_board(|p| p.to_ascii(), '.')
    }

    pub fn to_unicode(&self) -> String {
        self.format_board(|p| p.to_unicode(), '·')
    }

    /// Returns the FEN string of the current board state
    pub fn to_fen(&self) -> String {
        let arr = self.to_piece_array();

        let mut placement = String::new();
        for rank in (0..8usize).rev() {
            let mut empty = 0u32;
            for file in 0..8usize {
                match arr[rank][file] {
                    None => empty += 1,
                    Some(piece) => {
                        if empty > 0 {
                            placement.push(char::from_digit(empty, 10).unwrap());
                            empty = 0;
                        }
                        placement.push_str(&piece.to_fen());
                    }
                }
            }
            if empty > 0 { placement.push(char::from_digit(empty, 10).unwrap()); }
            if rank > 0  { placement.push('/'); }
        }

        let mut castling = String::new();
        if self.castling & Board::WHITE_KING_CASTLE  != 0 { castling.push('K'); }
        if self.castling & Board::WHITE_QUEEN_CASTLE != 0 { castling.push('Q'); }
        if self.castling & Board::BLACK_KING_CASTLE  != 0 { castling.push('k'); }
        if self.castling & Board::BLACK_QUEEN_CASTLE != 0 { castling.push('q'); }
        if castling.is_empty()         { castling.push('-'); }

        let enpassant = match self.enpassant {
            Some(sq) => sq.to_fen(),
            None     => "-".to_string(),
        };

        format!("{} {} {} {} {} {}",
            placement, self.side.to_fen(), castling, enpassant,
            self.halfmoves, self.fullmoves)
    }

    /// Converts the given FEN string into a board state
    pub fn from_fen(fen: &str) -> Self {
        let parts: Vec<&str> = fen.split(' ').collect();

        // Parse piece placement (rank 8 first in FEN)
        let mut arr = [[None; 8]; 8];
        for (rank_idx, rank_str) in parts[0].split('/').enumerate() {
            let rank = 7 - rank_idx;
            let mut file = 0;
            for ch in rank_str.chars() {
                if ch.is_ascii_digit() {
                    file += ch as usize - '0' as usize;
                } else {
                    arr[rank][file] = Some(Piece::from_fen(&ch.to_string()));
                    file += 1;
                }
            }
        }

        let pieces = Board::from_piece_array(&arr);

        let mut sides = [0u64; 2];
        for piece in Piece::WHITE { sides[Side::White as usize] |= pieces[piece as usize]; }
        for piece in Piece::BLACK { sides[Side::Black as usize] |= pieces[piece as usize]; }
        let occupied = sides[0] | sides[1];

        let side = Side::from_fen(parts[1]);

        let mut castling = 0u8;
        for ch in parts[2].chars() {
            match ch {
                'K' => castling |= Board::WHITE_KING_CASTLE,
                'Q' => castling |= Board::WHITE_QUEEN_CASTLE,
                'k' => castling |= Board::BLACK_KING_CASTLE,
                'q' => castling |= Board::BLACK_QUEEN_CASTLE,
                '-' => {}
                _   => panic!("invalid castling char: {ch}"),
            }
        }

        let enpassant = Square::from_fen(parts[3]);
        let halfmoves = parts[4].parse().unwrap();
        let fullmoves = parts[5].parse().unwrap();

        Board { pieces, sides, occupied, side, castling, enpassant, halfmoves, fullmoves }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_starting_position() {
        let board = Board::starting_position();
        println!("ASCII:\n{}\n", board.to_ascii());
        println!("Unicode:\n{}", board.to_unicode());
    }
}