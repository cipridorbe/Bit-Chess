use crate::{egtb::threepiece::{pos::Pos, revmove::{Flag, MovingPiece, RevMove}}, repr::{colour::Colour, piece::Piece, square::Square}};

impl Pos {
    pub(crate) fn make_revmove(&mut self, revmove: RevMove) {
        if let Some(reflection) = revmove.reflection {
            self.reflect(reflection);
        }
        self.enpassant = revmove.enpassant;
        let source = match revmove.moving {
            MovingPiece::WhiteKing => {
                let out = self.king[Colour::White];
                self.king[Colour::White] = revmove.to;
                out
            },
            MovingPiece::BlackKing => {
                let out = self.king[Colour::Black];
                self.king[Colour::Black] = revmove.to;
                out
            },
            MovingPiece::P1 => {
                let out = self.p1.0;
                self.p1.0 = revmove.to;
                out
            },
            MovingPiece::P2 => {
                let out = self.p2.unwrap().0;
                self.p2 = Some((revmove.to, self.p2.unwrap().1));
                out
            },
            MovingPiece::P3 => {
                let out = self.p3.unwrap().0;
                self.p3 = Some((revmove.to, self.p3.unwrap().1));
                out
            }
        };
        match revmove.flag {
            Flag::Quiet => {},
            Flag::Enpassant => {
                let new_square = match self.last_moved {
                    Colour::White => Square::from_u8(source as u8 - 8),
                    Colour::Black => Square::from_u8(source as u8 + 8),
                };
                self.p3 = Some((new_square, revmove.uncaptured.unwrap()));
            },
            Flag::Promotion => {
                let pawn = Piece::pawn(self.last_moved);
                match revmove.moving {
                    MovingPiece::P1 => self.p1.1 = pawn,
                    MovingPiece::P2 => self.p2 = Some((self.p2.unwrap().0, pawn)),
                    MovingPiece::P3 => self.p3 = Some((self.p3.unwrap().0, pawn)),
                    _ => panic!("cannot have king unpromotion")
                }
            }
        }
        if revmove.uncaptured.is_some() && revmove.flag != Flag::Enpassant {
            self.p3 = Some((source, revmove.uncaptured.unwrap()))
        }
        self.last_moved = !self.last_moved;
        self.make_canonical();
    }
}