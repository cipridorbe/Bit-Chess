use std::ops::{Index, IndexMut};

use crate::repr::{bitboard::BB, castling::CastlingRights, colour::Colour, hash::Hash, piece::Piece, square::Square};

#[derive(Clone)]
pub struct Board {
    /// Bitboard of piece-wise occupancy
    pub pieces: [BB; 12],
    /// Bitboard of occupancy of all pieces of same color
    pub colours: [BB; 2],
    /// Array mapping squares to pieces
    pub mailbox: [Option<Piece>; 64],
    /// The current side to move
    pub colour: Colour,
    /// Current castling rights
    pub castling_rights: CastlingRights,
    /// Enpassant square, if any
    pub enpassant: Option<Square>,
    /// Full moves made since the start of the game
    pub fullmoves: u8,
    /// Total half moves made since the last irreversible move
    pub halfmove_clock: u8,
    /// History of all board positions
    pub hash_history: Vec<Hash>,
    /// Additional state information
    pub state: BoardState
}

impl Board {
    pub const A_FILE: BB = BB::new(0x0101010101010101);
    pub const B_FILE: BB = BB::new(0x0202020202020202);
    pub const G_FILE: BB = BB::new(0x4040404040404040);
    pub const H_FILE: BB = BB::new(0x8080808080808080);

    pub const RANK_1: BB = BB::new(0x00000000000000ff);
    pub const RANK_2: BB = BB::new(0x000000000000ff00);
    pub const RANK_7: BB = BB::new(0x00ff000000000000);
    pub const RANK_8: BB = BB::new(0xff00000000000000);

    /// Bitboard of all occupied squares.
    pub fn occupied(&self) -> BB {
        self.colours[0] | self.colours[1]
    }

    /// Bitboard of attacks by the given colour to the other colour
    pub fn attacks(&self, colour: Colour) -> BB {
        self.state.attacks[colour as usize][0] | self.state.attacks[colour as usize][1]
    }

    /// Adds a hash to the current hash history and updates `state.repetitions`,
    /// assuming `halfmove_clock` and `total_halfmoves` have been updated to match the current move
    pub fn add_hash_to_history(&mut self, hash: Hash) {
        self.hash_history.push(hash);
        self.state.repetitions = 1;
        if self.halfmove_clock >= 4 {
            let mut idx = self.hash_history.len() as i8 - 2;
            let end = self.hash_history.len()  as i8 - self.halfmove_clock as i8;
            while idx >= end {
                if self.hash_history[idx as usize] == hash {
                    self.state.repetitions += 1;
                }
                idx -= 2;
            }
        }
    }

    fn empty() -> Self {
        Board {
            pieces: [BB::new(0); 12],
            colours: [BB::new(0); 2],
            mailbox: [None; 64],
            colour: Colour::White,
            castling_rights: unsafe { std::mem::transmute(0u8) },
            enpassant: None,
            fullmoves: 0,
            halfmove_clock: 0,
            hash_history: Vec::new(),
            state: BoardState {
                hash: unsafe { std::mem::transmute(0u64) },
                attacks: [[BB::new(0); 2]; 2],
                in_check: [false; 2],
                checkers: BB::new(0),
                mg_eval: 0,
                eg_eval: 0,
                repetitions: 0,
                phase_unbounded: 0
            } 
        }
    }

    /// The FEN of the current board
    pub fn to_fen(&self) -> String {
        let mut out = String::new();
        for rank in (0..8).rev() {
            let mut empty_counter = 0;
            for file in 0..8 {
                let square = Square::from_rank_file(rank, file);
                let piece = self[square];
                match piece {
                    None => empty_counter += 1,
                    Some(p) => {
                        if empty_counter != 0 {
                            out.push_str(&empty_counter.to_string());
                            empty_counter = 0;
                        }
                        out.push_str(&p.to_fen());
                    }
                }
            }
            if empty_counter != 0 {
                out.push_str(&empty_counter.to_string());
            }
            if rank != 0 {
                out.push('/');
            }
        }

        out.push(' ');
        out.push_str(&self.colour.to_fen());
        out.push(' ');
        out.push_str(&self.castling_rights.to_fen());
        out.push(' ');
        out.push_str(&match self.enpassant {
            None => "-".to_string(),
            Some(square) => square.to_fen()
        });
        out.push(' ');
        out.push_str(&self.halfmove_clock.to_string());
        out.push(' ');
        out.push_str(&self.fullmoves.to_string());

        out
    }

    /// Converts the FEN into a `Board`
    pub fn from_fen(fen: &str) -> Self {
        let parts: Vec<_> = fen.split(' ').collect();
        if parts.len() != 6 {
            panic!("Invalid fen for board state: {}", fen);
        }

        let mut out = Board::empty();
        
        let rows: Vec<_> = parts[0].split('/').collect();
        if rows.len() != 8 {
            panic!("Invalid fen for piece positions: {}", fen);
        }
        for (i, row) in rows.iter().enumerate() {
            let rank = 7 - i;
            let mut file = 0;
            for c in row.chars() {
                if let Some(empty) = c.to_digit(10) {
                    file += empty as u8;
                    continue;
                }
                let square = Square::from_rank_file(rank as u8, file);
                let piece = Piece::from_fen(&c.to_string());
                out[square] = Some(piece);
                out[piece] |= square;
                out[piece.colour()] |= square;
                out.state.hash ^= Hash::POSITION_PIECE[piece as usize][square as usize];
                out.state.phase_unbounded += piece.phase_value();
            }
        }
        out.colour = Colour::from_fen(parts[1]);
        if out.colour == Colour::White {
            out.state.hash ^= Hash::SIDE_HASH;
        }
        out.castling_rights = CastlingRights::from_fen(parts[2]);
        out.state.hash ^= Hash::CASTLING_HASH[out.castling_rights.0 as usize];
        out.enpassant = Square::from_fen(parts[3]);
        if let Some(square) = out.enpassant {
            let file = square.file();
            out.state.hash ^= Hash::ENPASSANT_HASH[file as usize];
        }
        out.halfmove_clock = u8::from_str_radix(parts[4], 10).unwrap();
        out.fullmoves = u8::from_str_radix(parts[5], 10).unwrap();
        out.hash_history.push(out.state.hash);

        out.state.repetitions = 1;

        unimplemented!()
    }
}

impl Index<Piece> for Board {
    type Output = BB;
    fn index(&self, index: Piece) -> &Self::Output {
        &self.pieces[index as usize]
    }
}

impl IndexMut<Piece> for Board {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self.pieces[index as usize]
    }
}

impl Index<Colour> for Board {
    type Output = BB;
    fn index(&self, index: Colour) -> &Self::Output {
        &self.colours[index as usize]
    }
}

impl IndexMut<Colour> for Board {
    fn index_mut(&mut self, index: Colour) -> &mut Self::Output {
        &mut self.colours[index as usize]
    }
}

impl Index<Square> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Square) -> &Self::Output {
        &self.mailbox[index as usize]
    }
}

impl IndexMut<Square> for Board {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self.mailbox[index as usize]
    }
}

/// Contains additional information about a position that is copied when
/// making a move to later unmake it.
#[derive(Clone)]
pub struct BoardState {
    /// Zobrist hash of the position
    hash: Hash,
    /// Indexed as attacks[colour][leaper/slider]
    attacks: [[BB; 2]; 2],
    /// Whether or not the king is in check, indexed by colour
    in_check: [bool; 2], 
    /// Bitboard of source squares of pieces checking the current side to move
    checkers: BB,
    /// middle game static evaluation of the position 
    mg_eval: i16,
    /// end game static evaluation of the position
    eg_eval: i16,
    /// How many times this position has appeared before
    repetitions: u8,
    /// The phase of the game, between 0 and 24
    phase_unbounded: u8
}