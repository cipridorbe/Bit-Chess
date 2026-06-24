use std::ops::{Index, IndexMut};

use crate::{eval::{Eval, pst::{PIECE_VALUE_EG, PIECE_VALUE_MG, PST_EG, PST_MG}}, movegen::r#move::Move, repr::{bitboard::BB, castling::CastlingRights, colour::Colour, hash::Hash, piece::{Piece, PieceType}, square::Square}, test_assert};

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
    /// History of all moves
    pub move_history: Vec<(Move, Option<Piece>)>,
    /// Additional state information
    pub state: BoardState,
}

impl Board {
    pub const A_FILE: BB = BB::new(0x0101010101010101);
    pub const B_FILE: BB = BB::new(0x0202020202020202);
    pub const G_FILE: BB = BB::new(0x4040404040404040);
    pub const H_FILE: BB = BB::new(0x8080808080808080);

    pub const RANK_1: BB = BB::new(0x00000000000000ff);
    pub const RANK_2: BB = BB::new(0x000000000000ff00);
    pub const RANK_3: BB = BB::new(0x0000000000ff0000);
    pub const RANK_4: BB = BB::new(0x00000000ff000000);
    pub const RANK_5: BB = BB::new(0x000000ff00000000);
    pub const RANK_6: BB = BB::new(0x0000ff0000000000);
    pub const RANK_7: BB = BB::new(0x00ff000000000000);
    pub const RANK_8: BB = BB::new(0xff00000000000000);

    pub const TOP: BB = BB::new(0xffffffff00000000);
    pub const BOTTOM: BB = BB::new(0x00000000ffffffff);

    pub fn starting_position() -> Self {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    /// Bitboard of all occupied squares.
    pub fn occupied(&self) -> BB {
        self.colours[0] | self.colours[1]
    }

    /// Bitboard of attacks by the given colour to the other colour
    pub fn attacks(&self, colour: Colour) -> BB {
        self.state.attacks[colour][0] | self.state.attacks[colour][1]
    }

    /// Whether or not the side to move is in check
    pub fn in_check(&self) -> bool {
        test_assert!(((self.state.attacks[!self.colour][0] | self.state.attacks[!self.colour][1]) & self[Piece::king(self.colour)] != 0) == (self.state.checkers != 0)); 
        self.state.checkers != 0
    }

    /// Whether or not the side not to move is in check
    pub fn other_in_check(&self) -> bool {
        self.attacks(self.colour) & self[Piece::king(!self.colour)] != 0
    }

    pub fn is_rule_draw(&self) -> bool {
        if self.state.repetitions >= 3 || self.halfmove_clock >= 100 {
            return true;
        }
        let num_pieces = self.occupied().count_ones();
        if num_pieces >= 5 {
            return false;
        } else if num_pieces <= 2 {
            return true;
        } else if num_pieces == 3 {
            return self.state.phase_unbounded == 1
        } else {
            let bishops = self[Piece::WhiteBishop] | self[Piece::BlackBishop];
            if (bishops & Square::DARK_SQUARES).count_ones() == 2 {
                return true;
            }
            if (bishops & Square::LIGHT_SQUARES).count_ones() == 2 {
                return true;
            }
            return false;
        }
    }

    /// Adds a hash to the current hash history and updates `state.repetitions`,
    /// assuming `halfmove_clock` and `total_halfmoves` have been updated to match the current move
    pub fn add_hash_to_history(&mut self, hash: Hash) {
        self.hash_history.push(hash);
        self.state.repetitions = 1;
        if self.halfmove_clock >= 4 {
            let mut idx = self.hash_history.len() as i16 - 3;
            let end = self.hash_history.len()  as i16 - self.halfmove_clock as i16;
            while idx >= end && idx >= 0{
                if self.hash_history [idx as usize] == hash {
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
            move_history: Vec::new(),
            state: BoardState {
                hash: unsafe { std::mem::transmute(0u64) },
                pawn_hash: unsafe { std::mem::transmute(0u64) },
                attacks: [[BB::new(0); 2]; 2],
                checkers: BB::new(0),
                mg_eval: 0,
                eg_eval: 0,
                repetitions: 0,
                phase_unbounded: 0,
                xray_attacks: [BB::new(0); 2],
                pinners: [BB::new(0); 2],
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
                out.state.hash ^= Hash::POSITION_PIECE[piece][square];
                if piece.is_pawn_or_king() {
                    out.state.pawn_hash ^= Hash::POSITION_PIECE[piece][square];
                }
                out.state.phase_unbounded += piece.phase_value();
                out.state.mg_eval += PIECE_VALUE_MG[piece] + PST_MG[piece][square];
                out.state.eg_eval += PIECE_VALUE_EG[piece] + PST_EG[piece][square];
                file += 1;
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
        for (colour, piece_type) in [(Colour::White, PieceType::Leaper), (Colour::White, PieceType::Slider), (Colour::Black, PieceType::Leaper), (Colour::Black, PieceType::Slider)] {
            out.state.attacks[colour][piece_type] = out.calculate_attacks(colour, piece_type)
        }
        out.state.checkers = out.calculate_checkers();
        for &colour in &[Colour::White, Colour::Black] {
            let (xray, pinners_bb) = out.compute_raw_xray_and_pinners(colour);
            out.state.xray_attacks[colour] = xray;
            out.state.pinners[!colour] = pinners_bb;
        }
        if (out.attacks(out.colour) & out[Piece::king(!out.colour)]) != 0 {
            panic!("Side not to move cannot be in check");
        }
        out
    }
}

impl Index<Piece> for Board {
    type Output = BB;
    fn index(&self, index: Piece) -> &Self::Output {
        &self.pieces[index]
    }
}

impl IndexMut<Piece> for Board {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self.pieces[index]
    }
}

impl Index<Colour> for Board {
    type Output = BB;
    fn index(&self, index: Colour) -> &Self::Output {
        &self.colours[index]
    }
}

impl IndexMut<Colour> for Board {
    fn index_mut(&mut self, index: Colour) -> &mut Self::Output {
        &mut self.colours[index]
    }
}

impl Index<Square> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Square) -> &Self::Output {
        &self.mailbox[index]
    }
}

impl IndexMut<Square> for Board {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self.mailbox[index]
    }
}

/// Contains additional information about a position that is copied when
/// making a move to later unmake it.
#[derive(Clone)]
pub struct BoardState {
    /// Zobrist hash of the position
    pub hash: Hash,
    /// Pawn-king Zobrist hash of position
    pub pawn_hash: Hash,
    /// Indexed as `attacks[colour][piece_type]`
    pub attacks: [[BB; 2]; 2],
    /// Bitboard of source squares of pieces checking the current side to move
    pub checkers: BB,
    /// middle game static evaluation of the position 
    pub mg_eval: Eval,
    /// end game static evaluation of the position
    pub eg_eval: Eval,
    /// How many times this position has appeared before
    pub repetitions: u8,
    /// The phase of the game, between 0 and 24
    pub phase_unbounded: u8,
    pub xray_attacks: [BB; 2],
    pub pinners: [BB; 2],
}