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

use crate::{eval::eval, movegen::{attacks::{all_attacks, is_in_check}, r#move::Move}, util::squares};

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

#[derive(Clone)]
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
    pub(crate) fullmoves: u8,

    /// Square to piece mapping. Used to quickly find the piece in a square.
    /// Index by `Square`
    pub(crate) mailbox: [Option<Piece>; 64],

    /// The hash of the current board state
    pub(crate) hash: u64,

    /// The hash history
    pub(crate) history: HashHistory,

    /// The absolute score of the board. Positive = white wins, Negative = black win.
    pub(crate) score: i16,

    /// The number of times the current move appears in the search history
    pub(crate) repetitions: u8,
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
    pub fn hash(&self) -> u64 { self.hash }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.mailbox[square as usize]
    }

    /// Returns the queenside/kingside castling rights for the given side
    pub fn castling_rights(&self, side: Side) -> (bool, bool) {
        let (queen, king) = match side {
            Side::White => (Board::WHITE_QUEEN_CASTLE, Board::WHITE_KING_CASTLE),
            Side::Black => (Board::BLACK_QUEEN_CASTLE, Board::BLACK_KING_CASTLE),
        };
        (self.castling & queen != 0, self.castling & king != 0)
    }

    /// Returns if the game drew by repetion or by 50-move rule
    pub fn is_rule_draw(&self) -> bool {
        if self.halfmoves >= 50 {
            return true;
        }
        return self.repetitions >= 3;
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

    // Returns true if the king of the current side is in check
    pub fn in_check(&self) -> bool {
        let attacks = all_attacks(&self, self.side.other());
        attacks & self.pieces[Piece::king(self.side) as usize] != 0
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
        let mut mailbox = [None; 64];
        for i in 0..64u8 {
            let square: Square = unsafe { std::mem::transmute(i) };
            let (rank, file) = square.to_rank_file();
            mailbox[square as usize] = arr[rank as usize][file as usize];
        }

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

        let mut hash = 0;
        for i in 0..64u8 {
            let square: Square = unsafe { std::mem::transmute(i) };
            if let Some(piece) = mailbox[square as usize] {
                hash ^= POSITION_PIECE_HASH[piece as usize][square as usize];
            }
        }
        hash ^= SIDE_HASH[side as usize];
        hash ^= CASTLING_HASH[castling as usize];
        if let Some(square) = enpassant {
            let (_, file) = square.to_rank_file();
            hash ^= ENPASSANT_HASH[file as usize];
        }

        let mut history = HashHistory::new();
        history.hashes.push(hash);

        let mut b = Board { pieces, sides, occupied, side, castling, enpassant, halfmoves, fullmoves, mailbox, hash, history, score: 0, repetitions: 1};
        b.score = eval(&b);
        b
    }

    // Returns true if the current move has non pawn pieces left
    pub fn has_non_pawn_pieces(&self) -> bool {
        let pawn = Piece::pawn(self.side);
        let king = Piece::king(self.side);
        self.sides[self.side as usize] & !(self.pieces[pawn as usize] | self.pieces[king as usize]) != 0
    }
}


pub const POSITION_PIECE_HASH: [[u64; 64]; 12] = [
    [0xF8235369F6B98426, 0xA9DF684D0DF85E26, 0x0B3345EFAEA55F6F, 0xE7FD1DE17353B03A, 0x799E478497E0659E, 0x2E264E88AC7FFD54, 0x2D9BA2728B5E057B, 0x435D56172F25B33E, 0x033B950D214E7AD7, 0x0119671F8BA72899, 0x07683F7191293C81, 0x7EABB799CDE05785, 0x4B5F4263E77F446E, 0x41972582A808CF93, 0x7538ED4EDDCE4222, 0x9F094952F659AAF3, 0x915FB723894D77E5, 0xEC4752F05E16694B, 0x216BB503D06D3BA5, 0x0EF8D6DE4CCFFE46, 0x26DB2568736D41C8, 0x2E8598A4DF3D86AA, 0x112D7B0DBD680903, 0x030D3DCE23916147, 0x0B7828873F12F749, 0x33354A807D9DCFB5, 0x5B1DE570B2D73695, 0x43B439DCEB11A096, 0x4DA77B6C527A2247, 0x8371AD67804CFB16, 0xA73B36A3083CC216, 0xB6B9B5D9C999D064, 0x18B7A2B855FFA83B, 0xB78DB5D849B8D6D0, 0xCF81A50A5CD31E79, 0x102FF200E523A7A6, 0x26D836A3190A68CD, 0x12BBA4D770094F67, 0xC959606E1A2EDEA8, 0x1217496B29593359, 0x28E850CBFBD16873, 0xD1BEB6FCC42E34E6, 0x17EFC2CDBD4C408D, 0xC8E6ED0CAD977140, 0x5B2BF03231B00297, 0x0AC6E51207DE79C2, 0x3AEBF4BC3B9F731C, 0x1B9498C2F76DA450, 0xA117904479265728, 0xFBB06921AD1970D2, 0xC53D2C4F20312B31, 0xFA5DD7FF76F52DF9, 0xEC7E17A2125EC888, 0xB68A87C56AC00BD9, 0x858B9F3770ABDD32, 0xFD189C39BC8042C6, 0x7288B8460960D3DA, 0x3BF7F945C2E7D26F, 0x6843A7DACD390632, 0xB78D02A12AF7BDC2, 0xA44F7497114819E7, 0x7B27AF6374C2AA15, 0x27789E5E36402FF5, 0xA0C94122056F91CB],
    [0xA57757424E771049, 0x5F1F9D4941D9475D, 0x0D2122F95710E77D, 0x10B4BA81EA99F7B7, 0x80B57437B2A8ACAB, 0xEABDD25E9FEB6358, 0xBFDF88C5B398EA76, 0xC34B91622447B1F5, 0x136C51991DDEF97C, 0x4E93F12583CCCFEB, 0x9EE368D2251FF111, 0x7D3D5BDFD1207F0B, 0xADD6F3F2C8F22F88, 0x1A8A2F0629C72969, 0xC2D926BA3B38202E, 0xA468F73BAF79373F, 0xD8F01BA8C1789A58, 0x15D4A1878155270C, 0x3CD3D91FCEF18139, 0x0A90B0FD46DD751F, 0x0B11AB3093773EDF, 0x0D90B9370E8032E9, 0x4E8A62927E156B62, 0xFB7F917139B5B2D6, 0x0E79D134C5317B27, 0x9DE3F338E89BB73E, 0x7861E21182AFB233, 0x0B513304FE7FDCD0, 0x1D13A0676DD39720, 0x9D889892E716D4A1, 0x8CF05BAFECBC7EC7, 0x7D6D3E375F80C875, 0x81862EF7154E5272, 0xC4116677FA475D3B, 0xADCE3446A8144605, 0xAB649C2A8354D018, 0xDD6D4B1815DE21E8, 0xDC92DA9C09BADC73, 0x412D5427AA61D2AB, 0x2A69AF7DCBB58737, 0x67C605480F2552B0, 0xC35968A83BF95EB9, 0x613BBA03E9604200, 0xB9DE0C6E7A3F4F56, 0x9A5B72376FEC5E7D, 0x2D76A42781B67A7E, 0xB0B73CE1C0BE4D2C, 0xEDC36BA0E8EDCC58, 0xF5872781B6954575, 0xACB5941C097C4317, 0x530CE16EAE3F41AB, 0xCBC566C100D25CA1, 0xBDA69527CA1F3B5C, 0x27B5A5596E324568, 0x8FC2FD822158AA93, 0xD58365D52C1F2298, 0x442B3AC3EF893A7A, 0xBA0F7E6A311DFCB9, 0x9175E57C1435B666, 0x6760364DDD118391, 0xF132D2C6FC403AE1, 0xD76C057289D39229, 0x3E6FB7132440E1F7, 0x75256926FFCE374B],
    [0xB560A9A3B9253158, 0xD934695979ACAB77, 0x288F658468C0704E, 0x35850B590FB4DF4D, 0x1D57753A7397D8E8, 0xA0D6D68C886FD624, 0xE3B36160D5D18924, 0xAFFD7F493690742F, 0x87C0147637C4B8EF, 0x529284090A4F2F6F, 0xB7272F987B40838E, 0xF882302F2D1C0AF5, 0x0D9060A69589ECB4, 0xE21F9671774D26F4, 0x32303707EE8417F1, 0xFB9CFFBA83BB217C, 0xAEB55130BD592AB0, 0x1F8A4DA03763053D, 0x88F10B7B8AD7CB05, 0xD19644D215BABC79, 0x886BDB32977B3EC1, 0x0E8F31FEF3302996, 0xDB2120F2A7D7490B, 0xE5863EC78E9EF310, 0xBEFC08AC77D1CD99, 0x52B0DD97CC8264D1, 0x73A85D5444D5FFD3, 0xA8ECE980C7DBAD2A, 0x3C7161A1D306E769, 0x0D55A7B01528AE6C, 0x575280BEA44568A6, 0x40AB07895BC804ED, 0x206EE3962C3A5C7F, 0x25E20BEED7D1BDDC, 0x482447CE19766CE4, 0x803E2A0EA7D9CC25, 0x353A2DE5754A9C31, 0x4159F6BA0D79A7F5, 0x343C5DE9C415D2D1, 0x3E1CB4FE0E128770, 0x4BB8DED3CB490A1F, 0xFFD43D48BED35360, 0xD082E3C661B696E5, 0x5CEC78708F7F46A9, 0xE111F99BFFEA663D, 0x8F4DAA573A3B2E4E, 0xC5F8D283F0914570, 0x5F59260C5844D949, 0x4BCE80F50E4EA117, 0x01D80D5764A5D8DB, 0xD5ED7D32DD3C7E21, 0xD3073DC6C9F2D680, 0xDBC16664ED6BFBD1, 0x33F5CD010B40AB21, 0x25962819453E2636, 0x2566FB9BEDF4761B, 0xB5268FC673B09C45, 0x570052BCE35E2C7A, 0xCD7C93417006A6F1, 0xB4ABEC34CEE3C521, 0x92122EF9CB96331E, 0x0B78B6E527C2DABB, 0x0990CEBF3A0A5A71, 0x396C72720FE0A28F],
    [0x548682817D924A08, 0xB205477DFEF4D868, 0x028C0880499117FF, 0x9019332588F7AFAA, 0x4DB7336220F9743E, 0x56C14FD0F26ADF9E, 0xFA8D8F193EEF24A8, 0x0BABC037C03E16C6, 0x290336441CE4F702, 0x324CFFFE41DAB96E, 0x960C0728F27B449D, 0x98C091D1EAEFF9A1, 0x0780ADC61CDC3E68, 0x7D0DCFC7D1AD2478, 0x4AAD9151A8DC970F, 0x9A7623138D964382, 0x3E818F0235EDC4E8, 0xA21B987BFB4E8C54, 0x45DAB3859FC3FC57, 0x9EB18B85D1657864, 0x3079F18B04C4DB30, 0xBDF4AD51CECC1C64, 0x491DBFA8D5AD6B45, 0x744DEB56A5C93D41, 0x82AF4A53C193B2D7, 0x585D2372940F78C5, 0x8C69D94821CE0655, 0x7E465235BADA3370, 0xB3C485720616290A, 0x345186C9C784541B, 0x8B94A032FA681CA7, 0x905612F2E8C5C61B, 0xDC24CA28B40CF556, 0x71AD177752B0CBEF, 0xEDCD561720F6F310, 0xB4B7BB3AB7ADB99B, 0x1C4DEB3A42941AA4, 0xE662495D0546AA9C, 0x3A96FA8048E27EE2, 0x5D85B965A9086AAE, 0xCA54322BA134DE55, 0xDCBD836A3BBFDDFF, 0xB0C1A7A09C1DDB56, 0x180057E151E1ADD6, 0x35C18F78392602D1, 0x74ACB8DC01AE38C9, 0x486623B8B1271781, 0x7B6AA5151BAE62A7, 0x7907752FE6D3E0D0, 0x33445961EEE68511, 0x1ADF8B1B5A1B1F93, 0x22C690D39452BC9C, 0x91EE6B7D20FD9539, 0x0E1583065CD6D805, 0xE2404F020438630F, 0x7824EAF4922AEBF8, 0x4A1A2B936301D227, 0x04F58EC4FB625778, 0x6CC45586019BB2B4, 0x89AEA2C74F113F93, 0x34D8A4C0F6FA14EA, 0x975D40F2AD2772BE, 0x485BF290540C2D43, 0xE170AA2899581C07],
    [0x5DD517E10CE05C7F, 0xE91A73C95BE37B17, 0x3E495EDD6AD5D208, 0xF752AFCDF999774A, 0x7D8840ABCC6CA66C, 0x1956F5AAB7CD94D2, 0x1094800B7D846437, 0x8FCF84958313086C, 0x96E4B14882E981DE, 0x9D683A3BF1E663CA, 0x8CE42529634CE76D, 0x68574EBA18F7BA4C, 0x6AB194097401ECD9, 0x34458BAF90CF5A60, 0x5C600493007BEA9C, 0x06A2A1FB7B638CB6, 0xD8E0EE64602C5FB6, 0x4C37C6F2B009B208, 0x8ACC84354DC5CC0A, 0xD6504393CB91E81A, 0xE584E460F2350AD4, 0xCD89AEE95A49887D, 0x5E58984154BCADE0, 0x2FFAEF2EE088E962, 0x3B648580493C776F, 0x42D2C52B943673EC, 0x7DBFE7772543FF56, 0x76781EEBE081B38C, 0x35A6146BD26553AB, 0xEC2F934A6B5AD2BC, 0xE7A2BA1DA9985C6D, 0xD29AE3789C55D0B5, 0xE4114B57A3A689B0, 0x7086DA5EBA68C497, 0x445A8DD0A7DF699A, 0x45FAFC1B6AFA3893, 0x4D9A4576D2383529, 0x3D675BA1129781B9, 0x6C8C32F351CB0B41, 0x4E2AE09D08A96B51, 0xCEC8D4C69E5F0760, 0x2912016ED2BACD03, 0xAD330FB4AB8C023E, 0x134E7A664B511F6C, 0x7CA7B8BD0FBF82AE, 0x174B343C13F0B7E5, 0x2C0044AFCC29D2C1, 0x149FAA271BCE0A5B, 0xDFAA79D4883D95FB, 0xD8B36F1E9503583E, 0x3EE952C4B87BE52B, 0x2FCE72941F72A55E, 0x04D60AF1E527D0C3, 0xE27F3E8173684AB0, 0x7ED41926A958D1D3, 0xFFC982465AF65CA7, 0x41D5AED4E190FE20, 0x371BCDB338CF198B, 0x5D553643FA0F1DE1, 0xC965D2D0D8C22D76, 0x341939565ABCB856, 0xAA98CFA2AE8BAFA8, 0x7468D07FE417A1D6, 0x7844DF2FA2B89E94],
    [0x8E9C843F2C298D86, 0x351A131FBC92765E, 0x6D3B5B49F3D78F6C, 0x1C9C85EE57126F75, 0xE4873D639D87538A, 0xBC33B6B7278F833C, 0xA061F4A201F1D5E4, 0xDD86F2A9E8111712, 0xD90A686D059E1B12, 0x336493CF603E88F7, 0xC4C09DE4EA2DFC2B, 0x9001989551ED8D2D, 0xBD03476CD87DFD62, 0xD2639AF49656C282, 0x835B83FB9B39F630, 0xC0BE0D593E69713F, 0xD667526C8299BE81, 0xFA17495E764352A0, 0x5CC233B8282CFFFB, 0xFE9D825F2FD87896, 0xE55987779E1101A8, 0x03B2F829F8A0B885, 0xB96E2B199CE9D427, 0xD6AC67B51669A2EF, 0x05F4CD29C05FB262, 0xCEF0678FAFBF5BB3, 0x75CA4B2F6E7AB4E1, 0x4DCFA94B761EDA40, 0x83EF683B3AA56BE6, 0x591E9D45B5D3AD4D, 0xEE29AD7DB89FA8F4, 0xF6F155709FF0420F, 0xA3143E506BE2151C, 0x5079F9F89438EA77, 0x4B4F73DA14060419, 0xFE0A59E70C4F0D10, 0xF4B0FF0F41C62135, 0x0E5D59F454B4CBFF, 0x20BDB15BE679CC5F, 0x855AF19DE69821DE, 0x486E2BF3073C3F7E, 0x3E9A4C9A7D739156, 0xCF70BF70E2B7BD31, 0xA68C768F88492CA3, 0xCD45F72C421C6DBC, 0x999FD1EC773CF770, 0x494887D36841DE25, 0xEC7C0117EE4138D8, 0xB3560CAA9F041747, 0x6B5E2BCA384A3CD2, 0x93BDCDFE1D87DA91, 0xFE538A6BF6022EDC, 0xD67AF6AB08B0CA7D, 0x131687B5B957FD60, 0x6E3B3B656802AD47, 0xFE655198EEFEB209, 0x1A68ED0AA066AF75, 0xB666632F6F731BD5, 0x6B30E74F002E7C1B, 0x9964B2C1DA578652, 0xF0111DEB7254DFF2, 0xF3C4896974A94159, 0x0BC978A760F588C6, 0x10FE7D6DAB00E7E2],
    [0xC78941494BE3F07D, 0xB9E32BF36F0272A9, 0x745C8F8B46AA0E09, 0x8E5FEEFC0A265B43, 0x1D2DF5AFF499E48E, 0xC8B37021A5B84F4B, 0xB06CC972B627073F, 0xFE6E5DA82E1B7821, 0x4451150BA8BBBB6F, 0xBD8A97770B4DBF25, 0x2FF6D151C07257E0, 0xAFF2A6D52403704D, 0xEB2CC606C4B67C06, 0xE744BD4FED233AEF, 0x064F488C1D96B354, 0xBE5882137DA4B387, 0xF5320ADD2F793970, 0x43422FE05DADD64B, 0x24C00005619E59D8, 0x0A066F332B98FC0E, 0x9C93E3D71CAD05BC, 0xFE1A85504CE92B2A, 0x12B9227A442B5746, 0xC1BF465D78E118B5, 0x5F28277CE42A657F, 0x7964457BF1C9BA87, 0x9AD3AA25F08E91A2, 0xEEC2E9398C5356CC, 0xAF598CF51C1359D3, 0x3E8D7C5E00849B69, 0x18F2370D3905031F, 0x59C995B8C8BF0F7A, 0x92903F96DD68BD88, 0x092D4E65E42EA5DD, 0x988A3AE5D541F374, 0x40FDBBAD9A744A87, 0xD95E9DF81E4E6415, 0x1FDB11C72F378F9B, 0x92AA8CAD694F3BD7, 0xF7F7B6379DA65E07, 0x8A20D84180544997, 0x5BD7DB5ABD703A55, 0x690E541E43CBDC54, 0x7BC54C12F87AC5E0, 0x4ADCA628682CDA5C, 0x55B8E44F41A3515A, 0x1268467BB094C679, 0xF5BF581C2E11F822, 0x1AB79E28EE08E51F, 0x878FEE8B7B5AE2C8, 0x46B033A6C477AF6C, 0x6D956B6398F60AE8, 0x0BF375D0B9E44183, 0x62EF8095B719831F, 0x400EDE1C552C231C, 0x25B92BFF13A92F8E, 0xFC092251140341AB, 0xD0930C64C2C3728D, 0x2AFD6F3CB676A9F5, 0x716C0DD7AE405357, 0x64A7C1628696C3D3, 0xA174AD08273DADA2, 0x95844884F4FB175A, 0x6EABFC47D7305433],
    [0x3557038ADE6BA1C5, 0x0C184E7B44E4E33A, 0xB69168130B38DBBB, 0xB2FECCDACECEC350, 0xF57158E3A4CA09A7, 0x35E8B2E71B3528C5, 0xD3B1934249518FFB, 0x44F0C259418831D5, 0xC8F8A807FF337356, 0xE64E8D2ADBE5A61A, 0xDFCD2B6524B5D2A0, 0x0D3A7C4486CAE024, 0x3C2CA369BFD0EB04, 0x3CBE5305EBA05288, 0xC9F83D3B9AB9D2D2, 0x5164FDC721C2C26C, 0x10850640FB57DE76, 0xE5172BBF6322291C, 0xF791ABA40E3222FC, 0x4FB562DB34131C8E, 0x5590097D5EA8E1D0, 0xA57D22AB9B642E65, 0xECB2FE480C796BB4, 0xEE6C3169AAB086DC, 0x355A254EBFB096F8, 0x3EA1B1BEAA480449, 0xBEDD4D5CABF1579E, 0x1A13ECC2C353E96D, 0x71D2C9C589AA260B, 0x2D57364327782BB8, 0x56521A982C46AA5D, 0xABF32257384CA548, 0x2ED837136BA8E9C6, 0xAED195B3605C5DEB, 0x7CE9DEA3071C4D01, 0xB4FD3226E141C42B, 0xB0ED8C9DF06C3A63, 0x0797484680635284, 0x3CD76F93FA571516, 0x7EF5993BC95290FE, 0xFAFB63782CFAF962, 0x30C9EFCC251E9139, 0x096C6227C7C90031, 0x25C3029F507EF43E, 0x99EB980E9783EF61, 0xB2FBB94C55B5E6CD, 0x2AEECC1925871AE4, 0x2E10B927E797A6D1, 0xC8126FAE3C512BF3, 0x8CDCFDEDFF13CBD8, 0x6196A1B0E77A0993, 0x7A053048089CB787, 0x1FC356C99DD2426C, 0xC80B83FAD77757A0, 0x1805A0E755A3620B, 0xC40D0AC8F895577F, 0x67749F95B6C6C4A2, 0x2AF6A26ACF108A8E, 0x62CDC3196B5E2A4C, 0xA0E9115E3CE3FE0D, 0xD1B91379A0407FF3, 0x6AA8CA56AA6A0D32, 0x4F1EB75989AE89FC, 0x85E0E44DE701872A],
    [0x65B4CFBCC53B4027, 0x712842143F0EE185, 0x2F7ECCCFC6FA048A, 0xF84C5F5B1008710E, 0x039EFA039EDCC9B1, 0xE685C308BE622242, 0xD10135E5E57270C5, 0xE31326105C8B560E, 0x27319DDB1C20504B, 0x17A0A7065209FE1E, 0x813BB3A56AB41984, 0x58C722C5D7B2A5A3, 0xC78C2D193953A3AF, 0xE05A79542FC1FF2B, 0x80263D195F06607E, 0x869F98B2769E1711, 0x5AFAB255FCE8DA38, 0x50D8B99F57CBABFF, 0xCFC19879D57BC293, 0xFDC50E44FF02730C, 0x10B0EB72D6A4300E, 0xB9052683404946B6, 0xCEFCCFBB22719168, 0x274F6549D9491933, 0x819A312F0FDF3146, 0xC3A3BE9A572D0834, 0xE9A3607F3EF55F35, 0x43159E7C7FC23A47, 0x7664E469A1B3DD8E, 0x1183CC0D0A7F7B18, 0xA3629792BF949862, 0x2FB3C4259B55E89D, 0x68BE537BB5ED825E, 0x0F7550EBD229185F, 0x504637719D61B0B1, 0x94C4D50AFA68E2EF, 0x8E97D9B258F6C814, 0xCF9FCC30B82DC693, 0xC3503C747BDBD323, 0x0B702E61E028E6C7, 0x25B5E96C8CDC8F2B, 0x10E972B827BB1390, 0x3503D66E7112F506, 0xA8EB6DBFA6D9D9EE, 0x567D7FC08853670F, 0xAEAC022986E3B919, 0x5D0EFAFE16F884CA, 0xEBE0BA8C043DA5A0, 0x2ABE754BA51C52D4, 0xF30DEEEE3F0B8C69, 0x0EC31B6EE9CAB5D8, 0x2706F6BEBD8B2E28, 0xB35CB26B2B85CA8E, 0x3D2C7EE1B8FFD612, 0xC8371DBEF39EF86E, 0xCCAF5BC283EEDE18, 0xC3EE59EC2BAF367B, 0xFC758DD05F36018E, 0x84A1421CDB74AFA3, 0xF07FB87240D96E4E, 0x7AB3FF2CD35D1018, 0xD5927FAA0867D120, 0x1C61849CB1E8608B, 0x47943A84F7E438FB],
    [0x39AE91E6CD3A3D76, 0x93C8E742BE693AE1, 0x9D372E2D44530254, 0x5C4549E98F770B24, 0x4D2BC3B3E2CEF805, 0x003809030AB84EA6, 0xC076BD99C0D171F3, 0x5CC62D4C401CFEFD, 0x8BBA361977277D7B, 0x34BEC82B457053D9, 0x24503574651F8CC0, 0x516C5865D0FB2187, 0x629BE9BF7F19C4D7, 0xA5E3A40DD4C28DC3, 0xF9F5552D67227636, 0xCBF33D8317AA0F6F, 0x0F3AF387BEF9AFF8, 0x98588A9A50DDDCA1, 0x0407C35ACCE48EBA, 0xB0C68324037DB79C, 0x8CF4096C65C67947, 0x4A73232701D04F82, 0xAC13A162D5CD7C1A, 0xBBD7EF610D9B466C, 0x6A04A85317011161, 0xA1044B13AD636505, 0x457BD26FC802A69B, 0x6B903212C3E09A1E, 0x8A5FC4F3CD4C031B, 0x88C263A37028AD99, 0x2D2298A2E36B4D5B, 0x480C4FD5181A02D6, 0xE2B71CB5B8A5B5D2, 0x84460CD59A1F8D53, 0x7CA7C470BF0422EB, 0xB11A41D4FBC0E082, 0x1DC34AF31F86856A, 0xA1676092CFAA7E20, 0x6CF23C30EE85E3BA, 0x0472597E4E5F894B, 0x4B6FC20494DCD1F7, 0xE6BD14327F7DC0BC, 0x34367F9E725E41AC, 0x4A73F369A94D6EFF, 0x11A4ED89C4A56F7F, 0xF1577298EC1CE70A, 0xC1C427569F21326D, 0x89CAAAFF27D48F99, 0x135C453B56C09876, 0x5C528139934DD532, 0xD6DBBB2EE80804EF, 0x7FF13B49585904D9, 0x16D512E85A79E1EF, 0xD13E03332C3925B3, 0x253B14B5C1F6FE34, 0x87A440FA20A002FF, 0x79FC090AAB4F32C0, 0x8120F9BDF17F0514, 0x2A1A27B3A4DB3B60, 0x01A6C25F8C51438F, 0x026E20E36C8DD08D, 0x6489355918F076AE, 0x36AE5938BE875585, 0xB6670C610E3C360F],
    [0x13FA0D71EC6679FF, 0x1326BDE6C4B6D71A, 0x109E83847BC09122, 0xF9D008CC932DBA11, 0x2E4450EE714713B0, 0x783F0884CA76BEE7, 0x38C521ED7EB8EEDD, 0x7A9AB17A3EFD6198, 0x85EB2A476E9838DB, 0xE811F9F0337B8EDF, 0x55796261575DDD5D, 0xB9404AA2FB09FD42, 0xAF5532F1FFE9D467, 0x4A99CD15B362B480, 0xF93E438F2E82D584, 0xBF802375C84C7887, 0xC81DDC06A56D3B61, 0xD2556D77DBB22492, 0xB2F155F9C80C00A5, 0x168C2E024E227CD5, 0x91751C154C16DB51, 0x6F0171692BB2B66F, 0x9F31770830821263, 0x5F7E8A145837DD1F, 0x5CE069DFC326527A, 0x51FBD2E1F2D3D044, 0xF51047784E9BB325, 0xBCA93A54A43085E7, 0x5BEA588AD382FF6E, 0xDE68A7242669B333, 0xD14984B6C89F1528, 0x4B8747D9FD6A4BC8, 0xEB5467031C30A4BC, 0x371A7FA2D9ACC374, 0x9194F20E1394E0A2, 0x3ED592A0D547627F, 0xDBFA046FB3C41094, 0xDA2820DCD96AE82E, 0x3B4B867CA8FCBBC0, 0xB5475F4FDCA6B0C3, 0x2E14A6B2EA477F4C, 0x628C50D1AEA044CC, 0xC0A5B94392FDA82A, 0x018DA32F5151882A, 0xD34F84399333FA73, 0x96358FCB41225326, 0xE04FCC39972E845C, 0xFE571AB17AD5AD87, 0xF574B3201D7B8788, 0x2FBD43DADCC8C45F, 0x1DF8760D3DF90524, 0xE0DE346E73674C8E, 0xDB0EC649DC75C890, 0x39DA0177217C1578, 0x93F7577BA1366A7A, 0x41E7B23E249439A3, 0xBC3E816706175A41, 0x66077A0869FBDD48, 0x5881335688960604, 0x977278EEAC6E5109, 0x2870C13C18BF65F3, 0xAE678D9F4204F12E, 0x09F096EA6AB5EE60, 0x4C741FAF4D906976],
    [0xDF0DB9F336B9EA4E, 0x3F041B0D17EF4E2D, 0xC4D169F9D1C94C84, 0x418DF3EE0C9C077D, 0xD88F510E797FA051, 0x84BF510C20225128, 0x13B62D2011C239C1, 0x097982239BC8DA8B, 0x61D45A06AA10F4C6, 0x768C5925940183E3, 0xD7338BA86A750D29, 0xB5EFB2607D4146D9, 0x7E694ADE86C49094, 0x7AE02F485969485F, 0x3DF19F6D3B5AA6AE, 0x49333E4DD4583547, 0xFC4EFAFBEFB515BF, 0x76DC3C691C27090E, 0x201B6C1AA8206091, 0x63F683015BDAB7EA, 0x1D0228ADAD7B241F, 0xF2A4DA3DF9D2B6FE, 0x47AD69809CC7D23D, 0xE84AFA5890C6F618, 0x2ACED79D776A8549, 0x6378AE366DD79737, 0xB120011DA8870A67, 0x7F85EE2EEDC60674, 0x34B57681AFE70B7A, 0x847A1BB1564CD088, 0x584DD59E10D9F185, 0xBC0EC0FFE54BC155, 0x40F44AA7627058C4, 0xA5ECB86B812C7A61, 0x05D0B134E236EC91, 0x820274688953ECB1, 0xF4B814C6DD4DC412, 0x3E958EF9039B4FED, 0xC1430D28A4DB061C, 0xB9963FC0F6A6E49D, 0xB0B211198C75867A, 0xE626F915275BAD6D, 0x4260F29AF0D2A705, 0x666D64C812800418, 0x9E00E47F75B64459, 0xB9C600B8951EEBAA, 0xAF1F58BE7885CBF6, 0x5D84C34D753ABD8D, 0xFB42FC71D949079E, 0x2B0D7A587C78626D, 0x370048A3C80707A2, 0x631FE8338F1CC742, 0x49DD49905C599DA8, 0x5B6750D12AA94829, 0xC56D26E160BBC096, 0x5A0DFD1CB52FFE47, 0x25B2B40FE3B40540, 0x978D45747CB2A718, 0xF04F78AD50669C25, 0x8171BA81BC13B620, 0x46031C8EA0D5F52D, 0x0CD6B460650C24A2, 0x6F207F9857B1986B, 0x25CD8DE746C7F5DB],
];

pub const SIDE_HASH: [u64; 2] = [
    0xF07FC64E7395D88C, 0x98C001DA453AE1D1,
];

pub const CASTLING_HASH: [u64; 16] = [
    0xD05AE8C61E5AB58E, 0xD33C98BD3583CE2B, 0x0A74597A3CA57B9C, 0x72E8C25F5A4B7EF5,
    0xDAD739A515CFA1BF, 0x25A3AE93C42D48EF, 0xC2557C35898AEFAC, 0x0532EAF6B4A1076C,
    0x3574C58814667FE7, 0xF74A3DE13CE28A52, 0x6033CB18BA7AA32E, 0x49B591869F6E56AE,
    0x664EAAFD32C6D0CF, 0x734367E2B0D3E445, 0x7DC3B3499D5ED5C7, 0x0C19B2BCBB580542,
];

pub const ENPASSANT_HASH: [u64; 8] = [
    0x1956AD8F80547346, 0x84EB2DDF50D70B4F, 0x49BD65161249A1EF, 0x099B64B28C293297,
    0xE055691AEE984258, 0x70ECE4CC1A5A0C75, 0xA7FBCBAB8C907970, 0x6BA79BD9351CB6CC,
];

/// Contains the history of all the hashes in the current game
#[derive(Clone)]
pub struct HashHistory {
    /// The hash history
    pub(crate) hashes: Vec<u64>,
    /// The index of the last irreversible move (pawn/capture)
    pub(crate) start_idx: usize,
}

impl HashHistory {
    pub fn new() -> Self {
        HashHistory {
            hashes: Vec::with_capacity(256),
            start_idx: 0
        }
    }
    
    /// Adds a hash to the history and returns the number of times the move
    /// repeats (including current)
    pub fn add(&mut self, hash: u64, mv: Move, piece: Piece) -> u8 {
        self.hashes.push(hash);
        if mv.is_capture() || piece == Piece::WhitePawn || piece == Piece::BlackPawn {
            self.start_idx = self.hashes.len() - 1;
        }
        let mut i = self.hashes.len() as i32 - 3;
        let mut repeats = 1;
        while i >= self.start_idx as i32 {
            if self.hashes[i as usize] == hash {
                repeats += 1;
            }
            i -= 2;
        }
        return repeats;
    }

    /// Removes the last move
    pub fn pop(&mut self) {
        self.hashes.pop();
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