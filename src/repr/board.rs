use std::ops::{Index, IndexMut};

use crate::repr::{bitboard::BB, colour::Colour, hash::Hash, piece::Piece, square::Square};

#[derive(Clone)]
pub struct Board {
    piece_bb: [BB; 12],
    colour_bb: [BB; 2],
    mailbox: [Option<Piece>; 64],
    colour: Colour,
    halfmoves: u16,
    hash_history: Vec<Hash>
}

impl Board {
    pub fn occupied_bb(&self) -> BB {
        self.colour_bb[0] | self.colour_bb[1]
    }
}

impl Index<Piece> for Board {
    type Output = BB;
    fn index(&self, index: Piece) -> &Self::Output {
        &self.piece_bb[index as usize]
    }
}

impl IndexMut<Piece> for Board {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self.piece_bb[index as usize]
    }
}

impl Index<Colour> for Board {
    type Output = BB;
    fn index(&self, index: Colour) -> &Self::Output {
        &self.colour_bb[index as usize]
    }
}

impl IndexMut<Colour> for Board {
    fn index_mut(&mut self, index: Colour) -> &mut Self::Output {
        &mut self.colour_bb[index as usize]
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