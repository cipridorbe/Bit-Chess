use std::{collections::VecDeque, io::{BufReader, BufWriter, Read, Write}, u8};
use crate::test_assert;

use crate::{egtb::{KINGS_IDX_PAWNFUL, KINGS_IDX_PAWNLESS, reflections::{DIAGONAL, HORIZONTAL, VERTICAL}, revmove::{MovingPiece, RevMove, RevMoveList}}, movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_knight_attacks, single_queen_attacks, single_rook_attacks}, tables::KING_ATTACKS}, repr::{bitboard::BB, board::{Board, BoardState}, castling::CastlingRights, colour::Colour, hash::Hash, piece::{Piece, PieceType}, square::{SEGMENT, SEGMENT_CARDINAL, SEGMENT_DIAGONAL, Square}}};

#[derive(Clone, PartialEq, Eq)]
pub struct Pos {
    king: [Square; 2],
    p1: (Square, Piece),
    p2: Option<(Square, Piece)>,
    last_moved: Colour
}

impl Pos {
    pub fn new(board: &Board) -> Self {
        let king = [board[Piece::WhiteKing].lsb(), board[Piece::BlackKing].lsb()];
        let mut remaining = board.occupied() & !(king[0].bb() | king[1].bb());
        if remaining.count_ones() == 0 {
            panic!("Position with only two kings");
        } else if remaining.count_ones() > 2 {
            panic!("Too many pieces remain");
        }

        let mut pos = if remaining.count_ones() == 1 {
            let p1 = (remaining.lsb(), board[remaining.lsb()].unwrap());
            Pos { king, p1, p2: None, last_moved: !board.colour }
        } else {
            let lsb1 = remaining.pop_lsb();
            let p1 = (lsb1, board[lsb1].unwrap());
            let lsb2 = remaining.lsb();
            let p2 = (lsb2, board[lsb2].unwrap());
            if p1.1.abs_regular_value() >= p2.1.abs_regular_value() {
                Pos { king, p1: p1, p2: Some(p2), last_moved: !board.colour }
            } else {
                Pos { king, p1: p2, p2: Some(p1), last_moved: !board.colour }
            }
        };
        if pos.p1.1.colour() == Colour::Black {
            pos.colour_swap();
        }
        pos.correct_reflection();
        if !pos.has_correct_king_diagonal() {
            pos.reflect(DIAGONAL);
        }

        pos

    }

    pub fn colour_swap(&mut self) {
        let tmp = self.king[0];
        self.king[0] = self.king[1];
        self.king[1] = tmp;
        self.last_moved = !self.last_moved;
        self.p1.1 = self.p1.1.colour_swap();
        if let Some(ref mut p2) = self.p2 {
            p2.1 = p2.1.colour_swap();
        }
        self.reflect(VERTICAL);
    }

    pub fn has_pawn(&self) -> bool {
        self.p1.1 == Piece::WhitePawn ||
        (self.p2.is_some() && (
            self.p2.unwrap().1 == Piece::WhitePawn ||
            self.p2.unwrap().1 == Piece::BlackPawn
        ))
    }

    pub fn reflect(&mut self, table: [Square; 64]) {
        self.king = [table[self.king[0]], table[self.king[1]]];
        self.p1.0 = table[self.p1.0];
        if let Some(ref mut p2) = self.p2 {
            p2.0 = table[p2.0];
        }
    }

    fn pos_hash(&self) -> u64 {
        use crate::repr::hash::Hash;
        let mut h = Hash::POSITION_PIECE[Piece::WhiteKing][self.king[0]].0
            ^ Hash::POSITION_PIECE[Piece::BlackKing][self.king[1]].0
            ^ Hash::POSITION_PIECE[self.p1.1][self.p1.0].0;
        if let Some((sq, pc)) = self.p2 {
            h ^= Hash::POSITION_PIECE[pc][sq].0;
        }
        if self.last_moved == Colour::White {
            h ^= Hash::SIDE_HASH.0;
        }
        h
    }

    fn canonical_key(&self) -> (u8, u8, u8, u8, u8) {
        (
            self.king[0] as u8,
            self.king[1] as u8,
            self.p1.0 as u8,
            self.p2.map(|p| p.0 as u8).unwrap_or(255),
            self.last_moved as u8,
        )
    }

    pub fn has_correct_king_diagonal(&self) -> bool {
        return true;
        if self.has_pawn() { return true; }
        let (wr, wf) = self.king[Colour::White].rank_file();
        if wr != wf { return true; }
        let (br, bf) = self.king[Colour::Black].rank_file();
        if br > bf { return false; }
        if br < bf { return true; }
        // BK also on diagonal — check p1
        let (p1r, p1f) = self.p1.0.rank_file();
        if p1r == p1f {
            // p1 on diagonal — check p2
            if let Some((p2sq, _)) = self.p2 {
                let (p2r, p2f) = p2sq.rank_file();
                if p2r > p2f { return false; }
            }
            return true;
        }
        // p1 off diagonal: canonical if reflecting doesn't reduce the min piece sq
        let p1_ref = p1f * 8 + p1r;
        let min_ref = if let Some((p2sq, _)) = self.p2 {
            let (p2r, p2f) = p2sq.rank_file();
            p1_ref.min(p2f * 8 + p2r)
        } else {
            p1_ref
        };
        min_ref >= self.p1.0 as u8
    }

    pub fn correct_reflection(&mut self) {
        self.correct_reflection_impl(true);
    }

    fn correct_reflection_impl(&mut self, normalize_colour: bool) {
        if let Some(p2) = self.p2 {
            if p2.1 == self.p1.1 && (self.p1.0 as u8) > (p2.0 as u8) {
                self.p2 = Some(self.p1);
                self.p1 = p2;
            }
        }
        if self.king[Colour::White].file() >= 4 {
            self.reflect(HORIZONTAL);
        }
        if !self.has_pawn() {
            if self.king[Colour::White].rank() >= 4 {
                self.reflect(VERTICAL);
            }
            let (wr, wf) = self.king[Colour::White].rank_file();
            if wr > wf {
                self.reflect(DIAGONAL);
            } else if wr == wf {
                let (br, bf) = self.king[Colour::Black].rank_file();
                if br > bf {
                    self.reflect(DIAGONAL);
                } else if br == bf {
                    // Reflections in steps 2/3 may have disrupted p1/p2 ordering;
                    // reorder self before comparison so canonical_key() is accurate.
                    if let Some(p2) = self.p2 {
                        if p2.1 == self.p1.1 && (self.p1.0 as u8) > (p2.0 as u8) {
                            self.p2 = Some(self.p1);
                            self.p1 = p2;
                        }
                    }
                    let mut clone = self.clone();
                    clone.reflect(DIAGONAL);
                    if let Some(p2c) = clone.p2 {
                        if p2c.1 == clone.p1.1 && (clone.p1.0 as u8) > (p2c.0 as u8) {
                            clone.p2 = Some(clone.p1);
                            clone.p1 = p2c;
                        }
                    }
                    if clone.canonical_key() < self.canonical_key() {
                        self.reflect(DIAGONAL);
                    }
                }
            }
        }
        // Re-apply same-colour ordering after reflections may have reversed it
        if let Some(p2) = self.p2 {
            if p2.1 == self.p1.1 && (self.p1.0 as u8) > (p2.0 as u8) {
                self.p2 = Some(self.p1);
                self.p1 = p2;
            }
        }
        // For same-kind opposite-colour pieces (KQvKQ, KRvKR, etc.): compare with
        // the colour-swapped perspective and keep whichever has the lower hash.
        if normalize_colour {
            if let Some(p2) = self.p2 {
                if !self.has_pawn() && p2.1 == self.p1.1.colour_swap() {
                    let mut clone = self.clone();
                    clone.colour_swap();
                    if clone.p1.1.colour() == Colour::Black {
                        let tmp = clone.p1;
                        clone.p1 = clone.p2.unwrap();
                        clone.p2 = Some(tmp);
                    }
                    clone.correct_reflection_impl(false);
                    if clone.canonical_key() < self.canonical_key() {
                        *self = clone;
                    }
                }
            }
        }
    }

    pub fn file(&self) -> (Piece, Option<Piece>) {
        (self.p1.1, self.p2.map(|p2| p2.1))
    }

    pub fn file_idx(&self) -> usize {
        self.p1.1 as usize + 5 * match self.p2 {
            None => 12,
            Some((_, p)) => p as usize
        }
    }

    pub fn p1_attacks(&self) -> BB {
        let sq = self.p1.0;
        let occupied = self.king[0].bb() | self.king[1] | sq | self.p2.map(|p2| p2.0.bb()).unwrap_or(BB::new(0));
        match self.p1.1 {
            Piece::WhitePawn => pawn_attacks(sq.bb(), Colour::White),
            Piece::WhiteKnight => single_knight_attacks(sq),
            Piece::WhiteBishop => single_bishop_attacks(sq, occupied),
            Piece::WhiteRook => single_rook_attacks(sq, occupied),
            Piece::WhiteQueen => single_queen_attacks(sq, occupied),
            _ => panic!("should have p1 attacks")
        }
    }

    pub fn p2_attacks(&self) -> Option<BB> {
        if let Some(p2) = self.p2 {
            let sq = p2.0;
            let occupied = self.king[0].bb() | self.king[1] | self.p1.0 | sq;
            let out = match p2.1 {
                Piece::WhitePawn => pawn_attacks(sq.bb(), Colour::White),
                Piece::BlackPawn => pawn_attacks(sq.bb(), Colour::Black),
                Piece::WhiteKnight | Piece::BlackKnight => single_knight_attacks(sq),
                Piece::WhiteBishop | Piece::BlackBishop => single_bishop_attacks(sq, occupied),
                Piece::WhiteRook | Piece::BlackRook => single_rook_attacks(sq, occupied),
                Piece::WhiteQueen | Piece::BlackQueen => single_queen_attacks(sq, occupied),
                _ => panic!("should have p1 attacks")
            };
            Some(out)
        } else {
            None
        }
    }

    // assumes kings can't check themselves
    pub fn in_check_simple(&self, colour: Colour) -> bool {
        match colour {
            Colour::Black => {
                let king = self.king[Colour::Black];
                let mut attacks = self.p1_attacks();
                if self.p2.is_some() && self.p2.unwrap().1.colour() == Colour::White {
                    attacks |= self.p2_attacks().unwrap();
                }
                king & attacks != 0
            },
            Colour::White => {
                let king = self.king[Colour::White];
                if self.p2.is_some() && self.p2.unwrap().1.colour() == Colour::Black {
                    king & self.p2_attacks().unwrap() != 0
                } else {
                    false
                }
            }
        }
    }

    pub fn index(&self) -> usize {
        let king_idx = if self.has_pawn() {
            KINGS_IDX_PAWNFUL[self.king[Colour::White]][self.king[Colour::Black]]
        } else {
            KINGS_IDX_PAWNLESS[self.king[Colour::White] as usize][self.king[Colour::Black]]
        } as usize;
        if self.p2.is_none() {
            if self.p1.1 == Piece::WhitePawn {
                return king_idx * 48 * 2 + (self.p1.0 as usize - 8) * 2 + !self.last_moved as usize;
            } else {
                return king_idx * 64 * 2 + self.p1.0 as usize * 2 + !self.last_moved as usize;
            }
        }
        let sq1 = self.p1.0 as usize;
        let sq2 = self.p2.unwrap().0 as usize;
        return match (self.p1.1.to_white(), self.p2.unwrap().1.to_white()) {
            (Piece::WhitePawn, Piece::WhitePawn) => king_idx * 48 * 48 * 2 + (sq1 - 8) * 48 * 2 + (sq2 - 8) * 2 + !self.last_moved as usize,
            (Piece::WhitePawn, _) => king_idx * 48 * 64 * 2 + (sq1 - 8) * 64 * 2 + sq2 * 2 + !self.last_moved as usize,
            (_, Piece::WhitePawn) => king_idx * 64 * 48 * 2 + sq1 * 48 * 2 + (sq2 - 8) * 2 + !self.last_moved as usize,
            (_, _) => king_idx * 64 * 64 * 2 + sq1 * 64 * 2 + sq2 * 2 + !self.last_moved as usize
        }
    }

    pub fn make_revmove(&mut self, revmove: RevMove) {
        if self.p2.is_some() {
            self.make_revmove_twopiece(revmove);
        } else {
            self.make_revmove_onepiece(revmove);
        }
    }

    fn make_revmove_twopiece(&mut self, revmove: RevMove) {
        let moving = revmove.moving_piece();
        if revmove.is_unpromotion() {
            let sq = if revmove.moving_piece() == MovingPiece::P1 { self.p1.0 } else { self.p2.unwrap().0 };
            let rank = sq.rank();
            let file = sq.file();
            let is_corner = (rank == 0 || rank == 7) && (file == 0 || file == 7);
            if !is_corner || !revmove.unpromote_diagonal() {
                if rank == 7 {}
                else if rank == 0 { self.reflect(VERTICAL); }
                else if file == 0 { self.reflect(DIAGONAL); self.reflect(VERTICAL); }
                else { self.reflect(DIAGONAL); }
                if self.last_moved == Colour::Black { self.reflect(VERTICAL); }
            } else {
                self.reflect(DIAGONAL);
                let new_sq = if revmove.moving_piece() == MovingPiece::P1 { self.p1.0 } else { self.p2.unwrap().0 };
                let target_rank = if self.last_moved == Colour::White { 7 } else { 0 };
                if new_sq.rank() != target_rank {
                    self.reflect(VERTICAL);
                }
            }
        }
        match moving {
            MovingPiece::WhiteKing => {
                self.king[Colour::White] = revmove.to;
            },
            MovingPiece::BlackKing => self.king[Colour::Black] = revmove.to,
            MovingPiece::P1 => {
                self.p1.0 = revmove.to;
                if revmove.is_unpromotion() {
                    self.p1.1 = Piece::WhitePawn;
                }
            },
            MovingPiece::P2 => {
                let mut p2 = self.p2.unwrap();
                p2.0 = revmove.to;
                if revmove.is_unpromotion() {
                    p2.1 = Piece::pawn(p2.1.colour());
                }
                self.p2 = Some(p2);
            }
        }

        if self.p2.unwrap().1.abs_regular_value() > self.p1.1.abs_regular_value() {
            let tmp = self.p1;
            self.p1 = self.p2.unwrap();
            self.p2 = Some(tmp);
        }
        if self.p1.1.colour() == Colour::Black {
            self.colour_swap();
        }
        self.last_moved = !self.last_moved;
        self.correct_reflection();
    }

    fn make_revmove_onepiece(&mut self, revmove: RevMove) {
        let moving = revmove.moving_piece();
        let from = match moving {
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
                if revmove.is_unpromotion() {
                    self.p1.1 = Piece::WhitePawn;
                    let rank = self.p1.0.rank();
                    let file = self.p1.0.file();
                    let is_corner = (rank == 0 || rank == 7) && (file == 0 || file == 7);
                    if !is_corner || !revmove.unpromote_diagonal() {
                        if rank == 7 {}
                        else if rank == 0 { self.reflect(VERTICAL); }
                        else if file == 0 { self.reflect(DIAGONAL); self.reflect(VERTICAL); }
                        else if file == 7 { self.reflect(DIAGONAL); }
                        else { panic!("unreachable herexx") }
                    } else {
                        self.reflect(DIAGONAL);
                        if self.p1.0.rank() != 7 { self.reflect(VERTICAL); }
                    }
                }
                let out = self.p1.0;
                self.p1.0 = revmove.to;
                out
            },
            MovingPiece::P2 => panic!("Should not have p2"),
        };

        if revmove.is_unenpassant() {
            let new_piece = revmove.uncaptured_piece().unwrap();
            let new_square = match self.last_moved {
                Colour::White => Square::from_u8(from as u8 - 8),
                Colour::Black => Square::from_u8(from as u8 + 8),
            };
            self.p2 = Some((new_square, new_piece));
        } else if revmove.is_uncapture() {
            let new_piece = revmove.uncaptured_piece().unwrap();
            self.p2 = Some((from, new_piece));
        }

        if self.p2.is_some() {
            if self.p1.1.abs_regular_value() < self.p2.unwrap().1.abs_regular_value() {
                let tmp = self.p1;
                self.p1 = self.p2.unwrap();
                self.p2 = Some(tmp);
            }
            if self.p1.1.colour() == Colour::Black {
                self.colour_swap();
            }
        }

        self.last_moved = !self.last_moved;
        self.correct_reflection();
    }

    fn make_revmovelist_onepiece(&self) -> RevMoveList {
        let mut out = RevMoveList::new();
        let colour = self.last_moved;
        let occupied = self.king[0].bb() | self.king[1].bb() | self.p1.0.bb();
        if colour == Colour::Black {
            let mut targets = KING_ATTACKS[self.king[Colour::Black]];
            targets &= !KING_ATTACKS[self.king[Colour::White]];
            targets &= !occupied;
            if self.p1.1 == Piece::WhitePawn && self.p1.0.rank() == 1 {
                targets &= !pawn_attacks(self.p1.0.bb(), Colour::White);
            }
            if self.king[Colour::Black].rank() == 0 || self.king[Colour::Black].rank() == 7 {
                for target in targets.squares() {
                    for uncaptured in RevMove::WHITEPAWNLESS {
                        out.add(RevMove::new(target, uncaptured, MovingPiece::BlackKing, false, false));
                    }
                }
            } else {
                for target in targets.squares() {
                    for uncaptured in RevMove::WHITE {
                        out.add(RevMove::new(target, uncaptured, MovingPiece::BlackKing, false, false));
                    }
                }
            }
            return out;
        }
        let mut king_targets = KING_ATTACKS[self.king[Colour::White]];
        king_targets &= !KING_ATTACKS[self.king[Colour::Black]];
        king_targets &= !occupied;
        if self.king[Colour::White].rank() == 0 || self.king[Colour::White].rank() == 7 {
            for target in king_targets.squares() {
                for uncaptured in RevMove::BLACKPAWNLESS {
                    out.add(RevMove::new(target, uncaptured, MovingPiece::WhiteKing, false, false));
                }
            }
        } else {
            for target in king_targets.squares() {
                for uncaptured in RevMove::BLACK {
                    out.add(RevMove::new(target, uncaptured, MovingPiece::WhiteKing, false, false));
                }
            }
        }
        if self.p1.1 == Piece::WhitePawn && self.p1.0.rank() == 1 {
            return out;
        }
        if self.p1.1 == Piece::WhitePawn {
            let back = Square::from_u8(self.p1.0 as u8 - 8);
            if back & occupied == 0 && pawn_attacks(back.bb(), Colour::White) & self.king[Colour::Black] == 0 {
                out.add(RevMove::new(back, None, MovingPiece::P1, false, false));
            }
            let back2 = Square::from_u8(self.p1.0 as u8 - 16);
            if back2.rank() == 1 && (back.bb() | back2.bb()) & occupied == 0 && pawn_attacks(back2.bb(), Colour::White) & self.king[Colour::Black] == 0 {
                out.add(RevMove::new(back2, None, MovingPiece::P1, false, false));
            }
            let back_attacks = pawn_attacks(self.p1.0.bb(), Colour::Black) & !occupied;
            for target in back_attacks.squares() {
                if pawn_attacks(target.bb(), Colour::White) & self.king[Colour::Black] != 0 {
                    continue;
                }
                for uncaptured in RevMove::BLACK {
                    if uncaptured == None { continue; }
                    out.add(RevMove::new(target, uncaptured, MovingPiece::P1, false, false));
                }
            }
            return out;
        }
        let mut targets = match self.p1.1 {
            Piece::WhiteKnight => single_knight_attacks(self.p1.0),
            Piece::WhiteBishop => single_bishop_attacks(self.p1.0, occupied),
            Piece::WhiteRook => single_rook_attacks(self.p1.0, occupied),
            Piece::WhiteQueen => single_queen_attacks(self.p1.0, occupied),
            _ => panic!("should not reach here ever")
        };
        targets &= !occupied;
        for target in targets.squares() {
            let bk = self.king[Colour::Black];
            let wk = self.king[Colour::White];
            if match self.p1.1 {
                Piece::WhiteKnight => single_knight_attacks(target) & bk != 0,
                Piece::WhiteBishop => SEGMENT_DIAGONAL[target][bk] != 0 && SEGMENT_DIAGONAL[target][bk] & wk == 0,
                Piece::WhiteRook => SEGMENT_CARDINAL[target][bk] != 0 && SEGMENT_CARDINAL[target][bk] & wk == 0,
                Piece::WhiteQueen => SEGMENT[target][bk] != 0 && SEGMENT[target][bk] & wk == 0,
                _ => panic!("should never reach")
            } {
                continue;
            }
            if self.p1.0.rank() == 0 || self.p1.0.rank() == 7 {
                for uncaptured in RevMove::BLACKPAWNLESS {
                    out.add(RevMove::new(target, uncaptured, MovingPiece::P1, false, false));
                }
            } else {
                for uncaptured in RevMove::BLACK {
                    out.add(RevMove::new(target, uncaptured, MovingPiece::P1, false, false));
                }
            }
        }
        let rank = self.p1.0.rank();
        let file = self.p1.0.file();
        if !(rank == 0 || file == 0 || rank == 7 || file == 7) {
            return out;
        }
        let is_corner = (rank == 0 || rank == 7) && (file == 0 || file == 7);
        let mut transformed = self.clone();
        if rank == 7 {}
        else if rank == 0 { transformed.reflect(VERTICAL); }
        else if file == 0 { transformed.reflect(DIAGONAL); transformed.reflect(VERTICAL); }
        else { transformed.reflect(DIAGONAL); }
        transformed.add_top_rank_proms(&mut out, false);
        if !is_corner {
            return out;
        }
        let mut transformed = self.clone();
        transformed.reflect(DIAGONAL);
        if transformed.p1.0.rank() != 7 {
            transformed.reflect(VERTICAL);
        }
        transformed.add_top_rank_proms(&mut out, true);
        out
    }

    fn add_top_rank_proms(&self, revmovelist: &mut RevMoveList, flag: bool) {
        let sq = self.p1.0;
        let occupied = self.king[0].bb() | self.king[1].bb() | self.p1.0.bb();
        let back = Square::from_u8(sq as u8 - 8);
        if back & occupied == 0 && pawn_attacks(back.bb(), Colour::White) & self.king[Colour::Black] == 0 {
            revmovelist.add(RevMove::new(back, None, MovingPiece::P1, true, flag));
        }
        let targets = pawn_attacks(sq.bb(), Colour::Black) & !occupied;
        for target in targets.squares() {
            if pawn_attacks(target.bb(), Colour::White) & self.king[Colour::Black] != 0 {
                continue;
            }
            for uncaptured in RevMove::BLACKPAWNLESS {
                if uncaptured == None { continue; }
                revmovelist.add(RevMove::new(target, uncaptured, MovingPiece::P1, true, flag));
            }
        }
    }

    fn make_revmovelist_twopiece(&self) -> RevMoveList {
        let mut out = RevMoveList::new();
        let colour = self.last_moved;
        let p1 = self.p1;
        let p2 = self.p2.unwrap();
        let occupied = self.king[0].bb() | self.king[1] | p1.0 | p2.0;
        if colour == Colour::Black {
            let king = self.king[Colour::Black];
            let mut targets = KING_ATTACKS[king] & !occupied & !KING_ATTACKS[self.king[Colour::White]];
            if p1.1 == Piece::WhitePawn && p1.0.rank() == 1 {
                targets &= !pawn_attacks(p1.0.bb(), Colour::White);
            }
            if p2.1 == Piece::WhitePawn && p2.0.rank() == 1 {
                targets &= !pawn_attacks(p2.0.bb(), Colour::White);
            }
            for target in targets.squares() {
                out.add(RevMove::new(target, None, MovingPiece::BlackKing, false, false));
            }
        } else {
            let king = self.king[Colour::White];
            let mut targets = KING_ATTACKS[king] & !occupied & !KING_ATTACKS[self.king[Colour::Black]];
            if p2.1 == Piece::BlackPawn && p2.0.rank() == 6 {
                targets &= !pawn_attacks(p2.0.bb(), Colour::Black);
            }
            for target in targets.squares() {
                out.add(RevMove::new(target, None, MovingPiece::WhiteKing, false, false));
            }
            if p1.1 == Piece::WhitePawn && p1.0.rank() > 1 {
                let back = Square::from_u8(p1.0 as u8 - 8);
                if back & occupied == 0 && pawn_attacks(back.bb(), Colour::White) & self.king[Colour::Black] == 0 {
                    out.add(RevMove::new(back, None, MovingPiece::P1, false, false));
                }
                let back2 = Square::from_u8(p1.0 as u8 - 16);
                if (back.bb() | back2.bb()) & occupied == 0 && pawn_attacks(back2.bb(), Colour::White) & self.king[Colour::Black] == 0 && back2.rank() == 1 {
                    out.add(RevMove::new(back2, None, MovingPiece::P1, false, false));
                }
            } else if p1.1 != Piece::WhitePawn {
                let targets = self.p1_attacks() & !occupied;
                for target in targets.squares() {
                    out.add(RevMove::new(target, None, MovingPiece::P1, false, false));
                }
                let rank = self.p1.0.rank();
                let file = self.p1.0.file();
                let is_corner = (rank == 0 || rank == 7) && (file == 0 || file == 7);
                let p2_is_pawn = p2.1 == Piece::WhitePawn || p2.1 == Piece::BlackPawn;
                if p2_is_pawn {
                    if rank == 7 {
                        self.add_top_rank_proms_twopiece(&mut out, true, false);
                    }
                } else if rank == 0 || rank == 7 || file == 0 || file == 7 {
                    let mut transformed = self.clone();
                    if rank == 7 {}
                    else if rank == 0 { transformed.reflect(VERTICAL); }
                    else if file == 0 { transformed.reflect(DIAGONAL); transformed.reflect(VERTICAL); }
                    else { transformed.reflect(DIAGONAL); }
                    transformed.add_top_rank_proms_twopiece(&mut out, true, false);
                    if is_corner {
                        let mut transformed = self.clone();
                        transformed.reflect(DIAGONAL);
                        if transformed.p1.0.rank() != 7 {
                            transformed.reflect(VERTICAL);
                        }
                        transformed.add_top_rank_proms_twopiece(&mut out, true, true);
                    }
                }
            }
        }
        if p2.1.colour() == colour {
            let rank = p2.0.rank();
            if (p2.1 == Piece::WhitePawn && rank > 1) || (p2.1 == Piece::BlackPawn && rank < 6) {
                let back = match colour {
                    Colour::White => Square::from_u8(p2.0 as u8 - 8),
                    Colour::Black => Square::from_u8(p2.0 as u8 + 8),
                };
                if back & occupied == 0 {
                    out.add(RevMove::new(back, None, MovingPiece::P2, false, false));
                }
                let back2 = match colour {
                    Colour::White => Square::from_u8(p2.0 as u8 - 16),
                    Colour::Black => Square::from_u8(p2.0 as u8 + 16),
                };
                if (back.bb() | back2) & occupied == 0 && (back2.rank() == 1 || back2.rank() == 6)  {
                    out.add(RevMove::new(back2, None, MovingPiece::P2, false, false));
                }
            } else if p2.1 != Piece::WhitePawn && p2.1 != Piece::BlackPawn {
                let targets = self.p2_attacks().unwrap() & !occupied;
                for target in targets.squares() {
                    out.add(RevMove::new(target, None, MovingPiece::P2, false, false));
                }
                let rank = p2.0.rank();
                let file = p2.0.file();
                let is_corner = (rank == 0 || rank == 7) && (file == 0 || file == 7);
                if p1.1 == Piece::WhitePawn {
                    let target_rank = if colour == Colour::White { 7u8 } else { 0u8 };
                    if rank == target_rank {
                        self.add_top_rank_proms_twopiece(&mut out, false, false);
                    }
                } else if rank == 0 || rank == 7 || file == 0 || file == 7 {
                    let mut transformed = self.clone();
                    if rank == 7 {}
                    else if rank == 0 { transformed.reflect(VERTICAL); }
                    else if file == 0 { transformed.reflect(DIAGONAL); transformed.reflect(VERTICAL); }
                    else { transformed.reflect(DIAGONAL); }
                    if self.last_moved == Colour::Black {
                        transformed.reflect(VERTICAL);
                    }
                    transformed.add_top_rank_proms_twopiece(&mut out, false, false);
                    if is_corner {
                        let mut transformed = self.clone();
                        transformed.reflect(DIAGONAL);
                        let target_rank = if self.last_moved == Colour::White { 7 } else { 0 };
                        if transformed.p2.unwrap().0.rank() != target_rank {
                            transformed.reflect(VERTICAL);
                        }
                        transformed.add_top_rank_proms_twopiece(&mut out, false, true);
                    }
                }
            }
        }
        out
    }

    fn add_top_rank_proms_twopiece(&self, revmovelist: &mut RevMoveList, is_p1: bool, flag: bool) {
        let occupied = self.king[0].bb() | self.king[1] | self.p1.0 | self.p2.unwrap().0;
        let moving_piece = if is_p1 { MovingPiece::P1 } else { MovingPiece::P2 };
        let p = if is_p1 { self.p1 } else { self.p2.unwrap() };
        let p_other = if is_p1 { self.p2.unwrap() } else { self.p1 };
        if (p_other.1 == Piece::WhitePawn || p_other.1 == Piece::BlackPawn) && (p_other.0.rank() == 0 || p_other.0.rank() == 7) {
            return;
        }
        let back = match self.last_moved {
            Colour::White => Square::from_u8(p.0 as u8 - 8),
            Colour::Black => Square::from_u8(p.0 as u8 + 8),
        };
        if back & occupied != 0 {
            return;
        }
        revmovelist.add(RevMove::new(back, None, moving_piece, true, flag));
    }

    fn to_board_partial(&self) -> Board {
        let mut out = Board {
            pieces: [BB::new(0); 12],
            colour: Colour::White,
            colours: [BB::new(0); 2],
            mailbox: [None; 64],
            castling_rights: CastlingRights(0),
            enpassant: None,
            fullmoves: 0,
            halfmove_clock: 0,
            hash_history: Vec::new(),
            move_history: Vec::new(),
            state: BoardState {
                hash: Hash(0),
                pawn_hash: Hash(0),
                attacks: [[BB::new(0); 2]; 2],
                checkers: BB::new(0),
                mg_eval: 0,
                eg_eval: 0,
                repetitions: 0,
                phase_unbounded: 0,
                xray_attacks: [BB::new(0); 2],
                pinners: [BB::new(0); 2]
            }
        };
        out.colour = !self.last_moved;
        out[Piece::WhiteKing] = self.king[Colour::White].bb();
        out[self.king[Colour::White]] = Some(Piece::WhiteKing);
        out[Piece::BlackKing] = self.king[Colour::Black].bb();
        out[self.king[Colour::Black]] = Some(Piece::BlackKing);
        out[self.p1.1] = self.p1.0.bb();
        out[self.p1.0] = Some(self.p1.1);
        if let Some(p2) = self.p2 {
            out[p2.1] |= p2.0.bb();
            out[p2.0] = Some(p2.1);
        }
        out[Colour::White] = out[Piece::WhitePawn] | out[Piece::WhiteKnight] | out[Piece::WhiteBishop] | out[Piece::WhiteRook] | out[Piece::WhiteQueen] | out[Piece::WhiteKing];
        out[Colour::Black] = out[Piece::BlackPawn] | out[Piece::BlackKnight] | out[Piece::BlackBishop] | out[Piece::BlackRook] | out[Piece::BlackQueen] | out[Piece::BlackKing];

        out.state.attacks[Colour::White][PieceType::Leaper] = KING_ATTACKS[self.king[Colour::White]];
        out.state.attacks[Colour::Black][PieceType::Leaper] = KING_ATTACKS[self.king[Colour::Black]];
        if self.p1.1.piece_type() == PieceType::Leaper {
            out.state.attacks[Colour::White][PieceType::Leaper] |= self.p1_attacks();
        }
        if let Some(p2) = self.p2 {
            if p2.1.piece_type() == PieceType::Leaper {
                out.state.attacks[p2.1.colour()][PieceType::Leaper] |= self.p2_attacks().unwrap();
            }
        }
        out.state.attacks[Colour::White][PieceType::Slider] = out.calculate_attacks(Colour::White, PieceType::Slider);
        out.state.attacks[Colour::Black][PieceType::Slider] = out.calculate_attacks(Colour::Black, PieceType::Slider);
        if out.colour == Colour::Black && self.p1_attacks() & out[Piece::BlackKing] != 0 {
            out.state.checkers = self.p1.0.bb();
        }
        if let Some(p2) = self.p2 {
            if out.colour == !p2.1.colour() && self.p2_attacks().unwrap() & out[Piece::king(out.colour)] != 0 {
                out.state.checkers |= p2.0.bb();
            }
        }
        let (white_xray, white_pinners) = out.compute_raw_xray_and_pinners(Colour::White);
        let (black_xray, black_pinners) = out.compute_raw_xray_and_pinners(Colour::Black);
        out.state.xray_attacks[Colour::White] = white_xray;
        out.state.xray_attacks[Colour::Black] = black_xray;
        out.state.pinners[Colour::Black] = white_pinners;
        out.state.pinners[Colour::White] = black_pinners;

        out
    }

    pub fn make_revmovelist(&self) -> RevMoveList {
        if self.p2.is_none() {
            self.make_revmovelist_onepiece()
        } else {
            self.make_revmovelist_twopiece()
        }
    }

    pub fn generator() -> [Vec<Status>; 100] {
        let (mut moves_left, mut status, mut queue) = Self::init_backwards_gen();
        #[cfg(feature = "assertions")]
        let mut debug_target_predecessors: Vec<(Pos, RevMove, u8)> = Vec::new();
        while let Some(pos) = queue.pop_front() {
            let state = *pos.index_file(&mut status, Status::UNKOWN);
            if state == Status::UNKOWN {
                panic!("shouldn't be unkown");
            }
            let revmovelist = pos.make_revmovelist();
            let mut seen_hashes: [u64; 255] = [0; 255];
            let mut seen_len: usize = 0;
            for i in 0..revmovelist.length {
                let revmove = revmovelist.list[i];
                let mut next = pos.clone(); next.make_revmove(revmove);
                let next_hash = next.pos_hash();
                if seen_hashes[..seen_len].contains(&next_hash) { continue; }
                seen_hashes[seen_len] = next_hash;
                seen_len += 1;
                test_assert!({
                    let p1_ok = next.p1.1 != Piece::WhitePawn || (next.p1.0.rank() >= 1 && next.p1.0.rank() <= 6);
                    let p2_ok = next.p2.map_or(true, |(sq, pc)| pc != Piece::WhitePawn && pc != Piece::BlackPawn || (sq.rank() >= 1 && sq.rank() <= 6));
                    if !p1_ok || !p2_ok {
                        eprintln!("BAD PAWN RANK: next p1={}@{} p2={} from pos p1={}@{} p2={} lm={}, revmove=to:{};flag:{}",
                            next.p1.1.to_fen(), next.p1.0.to_fen(),
                            next.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
                            pos.p1.1.to_fen(), pos.p1.0.to_fen(),
                            pos.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
                            pos.last_moved.to_fen(),
                            revmove.to.to_fen(), revmove.flag
                        );
                    }
                    p1_ok && p2_ok
                });
                if next.in_check_simple(next.last_moved) {
                    continue;
                }
                if !next.has_correct_king_diagonal() {
                    continue;
                }
                let left = next.index_file(&mut moves_left, u8::MAX);
                #[cfg(feature = "assertions")]
                {
                    let is_debug_target = next.king[0] == Square::a1
                        && next.king[1] == Square::e3
                        && next.p1 == (Square::f1, Piece::WhiteKnight)
                        && next.p2 == Some((Square::g2, Piece::BlackPawn))
                        && next.last_moved == Colour::White;
                    if is_debug_target {
                        debug_target_predecessors.push((pos.clone(), revmove, *left));
                    }
                    // Track what generates pred2 (should be impossible after correct_reflection)
                    let is_pred2 = next.king[0] == Square::d4
                        && next.king[1] == Square::h8
                        && next.p1 == (Square::a1, Piece::WhiteQueen)
                        && next.p2 == Some((Square::f7, Piece::WhiteQueen))
                        && next.last_moved == Colour::Black;
                    if is_pred2 {
                        eprintln!("PRED2 GENERATED from pos={} revmove: to={} moving={:?} unprom={} diag={}",
                            pos.to_board_partial().to_fen(),
                            revmove.to.to_fen(), revmove.moving_piece(),
                            revmove.is_unpromotion(), revmove.unpromote_diagonal());
                        eprintln!("  pos: WK={} BK={} p1={}@{} p2={} lm={}",
                            pos.king[0].to_fen(), pos.king[1].to_fen(),
                            pos.p1.1.to_fen(), pos.p1.0.to_fen(),
                            pos.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
                            pos.last_moved.to_fen());
                    }
                    let kind = if *left == u8::MAX { "UNINIT" } else if *left == 0 { "UNDERFLOW" } else { "" };
                    if !kind.is_empty() {
                        eprintln!("SPURIOUS REVMOVE ({kind}):");
                        eprintln!("  from pos: WK={} BK={} p1={}@{} p2={} lm={}",
                            pos.king[0].to_fen(), pos.king[1].to_fen(),
                            pos.p1.1.to_fen(), pos.p1.0.to_fen(),
                            pos.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
                            pos.last_moved.to_fen());
                        eprintln!("  from fen: {}", pos.to_board_partial().to_fen());
                        eprintln!("  revmove: to={} moving={:?} unprom={} diag={} uncap={:?} flag={:08b}",
                            revmove.to.to_fen(),
                            revmove.moving_piece(),
                            revmove.is_unpromotion(),
                            revmove.unpromote_diagonal(),
                            revmove.uncaptured_piece(),
                            revmove.flag);
                        eprintln!("  next pos: WK={} BK={} p1={}@{} p2={} lm={}",
                            next.king[0].to_fen(), next.king[1].to_fen(),
                            next.p1.1.to_fen(), next.p1.0.to_fen(),
                            next.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
                            next.last_moved.to_fen());
                        eprintln!("  next fen: {}", next.to_board_partial().to_fen());
                        if is_debug_target {
                            eprintln!("  --- all {} predecessors of target before this underflow ---", debug_target_predecessors.len());
                            for (pred, pred_rm, left_at_time) in &debug_target_predecessors {
                                eprintln!("    pred fen: {}  revmove: to={} moving={:?} unprom={} left_was={}",
                                    pred.to_board_partial().to_fen(),
                                    pred_rm.to.to_fen(),
                                    pred_rm.moving_piece(),
                                    pred_rm.is_unpromotion(),
                                    left_at_time);
                            }
                        }
                        panic!("spurious revmove");
                    }
                }
                *left -= 1;
                if state.is_loss() {
                    let mut new_status = Status(-state.0 + 1);
                    if new_status.0 > 120 {
                        new_status.0 = 120;
                    }
                    let slot = next.index_file(&mut status, Status::UNKOWN);
                    if *slot == Status::UNKOWN {
                        *slot = new_status;
                        queue.push_back(next);
                    }
                } else if *left == 0 {
                    let mut new_status = Status(-state.0 - 1);
                    if new_status.0 < -120 {
                        new_status.0 = -120;
                    }
                    let slot = next.index_file(&mut status, Status::UNKOWN);
                    if *slot == Status::UNKOWN {
                        *slot = new_status;
                        queue.push_back(next);
                    }
                }
            }
        }
        for i in 0..status.len() {
            let mut true_len = 0;
            for j in (0..status[i].len()).rev() {
                if status[i][j] == Status::UNKOWN || status[i][j] == Status::DRAW {
                    continue;
                }
                true_len = j + 1;
                break;
            }
            unsafe { status[i].set_len(true_len); }
            status[i].shrink_to_fit();
        }
        status
    }

    fn init_backwards_gen() -> ([Vec<u8>; 100], [Vec<Status>; 100], VecDeque<Pos>) {
        let mut moves_left = [const { Vec::new() }; 100];
        let mut status = [const { Vec::new() }; 100];
        let mut queue = VecDeque::<Pos>::new();
        for white_king in Square::all() {
            if white_king.file() >= 4 {
                continue;
            }
            for black_king in Square::all() {
                if KINGS_IDX_PAWNFUL[white_king][black_king] == u16::MAX {
                    continue;
                }
                let king = [white_king, black_king];
                for square1 in Square::all() {
                    if square1 == white_king || square1 == black_king {
                        continue;
                    }
                    for kind1 in [Piece::WhitePawn, Piece::WhiteKnight, Piece::WhiteBishop, Piece::WhiteRook, Piece::WhiteQueen] {
                        if kind1 == Piece::WhitePawn && (square1.rank() == 0 || square1.rank() == 7) {
                            continue;
                        }
                        for square2 in Square::all() {
                            for kind2 in [None, Some(Piece::WhitePawn), Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen), Some(Piece::BlackPawn), Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)] {
                                if kind2 == None && square2 != Square::a1 {
                                    continue;
                                }
                                if kind2.is_some() && (square2 == square1 || square2 == white_king || square2 == black_king) {
                                    continue;
                                }
                                if kind2.is_some() && kind2.unwrap().abs_regular_value() > kind1.abs_regular_value() {
                                    continue;
                                }
                                if kind2 == Some(Piece::WhitePawn) || kind2 == Some(Piece::BlackPawn) {
                                    if square2.rank() == 0 || square2.rank() == 7 {
                                        continue;
                                    }
                                }
                                let p1 = (square1, kind1);
                                let p2 = kind2.map(|kind2| (square2, kind2));
                                for last_moved in [Colour::White, Colour::Black] {
                                    let mut pos = Pos { king, p1, p2, last_moved };
                                    pos.correct_reflection();
                                    if pos.in_check_simple(pos.last_moved) {
                                        continue;
                                    }
                                    let prev_moves_left = *pos.index_file(&mut moves_left, u8::MAX);
                                    if prev_moves_left == u8::MAX {
                                        let num_moves = pos.count_distinct_canonical_successors();
                                        pos.insert_to_file(&mut moves_left, num_moves as u8, u8::MAX);
                                        if num_moves == 0 {
                                            if pos.in_check_simple(!pos.last_moved) {
                                                pos.insert_to_file(&mut status, Status::CHECKMATED, Status::UNKOWN);
                                                queue.push_back(pos);
                                            } else {
                                                pos.insert_to_file(&mut status, Status::DRAW, Status::UNKOWN);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        (moves_left, status, queue)
    }

    fn count_distinct_canonical_successors(&self) -> usize {
        let mut board = self.to_board_partial();
        let movelist = board.generate_movelist(false);
        let raw_moves = movelist.num_total_moves();
        // Diagonal collapse only occurs when both kings are on the main diagonal: then
        // correct_reflection applies a diagonal reflection to successors, which can make
        // two forward moves (to diagonal-mirror squares) produce the same canonical.
        let needs_dedup = !self.has_pawn() && {
            let (wr, wf) = self.king[0].rank_file();
            let (br, bf) = self.king[1].rank_file();
            wr == wf && br == bf
        };
        if !needs_dedup {
            return raw_moves;
        }
        let mut seen_hashes = [0u64; 128];
        let mut seen_len = 0usize;
        // In pawnless EGTB there are no pawns, so no queen-proms or en-passants;
        // movelist.length == num_total_moves() and movelist[i] covers every move.
        for i in 0..movelist.length {
            let mv = movelist[i];
            let unmake = board.makemove(mv);
            let remaining = board.occupied() & !(board[Piece::WhiteKing] | board[Piece::BlackKing]);
            let h = if remaining.count_ones() == 0 {
                u64::MAX // K-vs-K draw: never enters BFS queue, keeps moves_left > 0
            } else {
                Pos::new(&board).pos_hash()
            };
            board.unmakemove(mv, unmake);
            if !seen_hashes[..seen_len].contains(&h) {
                seen_hashes[seen_len] = h;
                seen_len += 1;
            }
        }
        seen_len
    }

    fn insert_to_file<T: Copy>(&self, files: &mut [Vec<T>], value: T, fill: T) {
        let file_idx = self.file_idx();
        let idx = self.index();
        if files[file_idx].len() <= idx {
            files[file_idx].resize(idx + 1, fill);
        }
        files[file_idx][idx] = value;
    }

    fn index_file<'a, T: Copy>(&self, files: &'a mut [Vec<T>], fill: T) -> &'a mut T {
        let file_idx = self.file_idx();
        let idx = self.index();
        if files[file_idx].len() <= idx {
            files[file_idx].resize(idx + 1, fill);
        }
        &mut files[file_idx][idx]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Status(pub i8);

impl Status {
    const UNKOWN: Self = Self(i8::MAX);
    const DRAW: Self = Self(0);
    const CHECKMATED: Self = Self(-1);

    pub fn is_win(self) -> bool {
        self.0 > 0
    }

    pub fn is_loss(self) -> bool {
        self.0 < 0
    }
}

pub fn save_tablebase(status: &[Vec<Status>; 100], path: &str) -> std::io::Result<()> {
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    for s in status {
        w.write_all(&(s.len() as u32).to_le_bytes())?;
    }
    for s in status {
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len()) };
        w.write_all(bytes)?;
    }
    Ok(())
}

pub fn load_tablebase(path: &str) -> std::io::Result<[Vec<Status>; 100]> {
    let mut r = BufReader::new(std::fs::File::open(path)?);
    let mut lengths = [0u32; 100];
    for l in &mut lengths {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        *l = u32::from_le_bytes(buf);
    }
    let mut status: [Vec<Status>; 100] = std::array::from_fn(|_| Vec::new());
    for (s, &len) in status.iter_mut().zip(lengths.iter()) {
        let mut buf = vec![0u8; len as usize];
        r.read_exact(&mut buf)?;
        *s = unsafe { std::mem::transmute(buf) };
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    static TB: OnceLock<[Vec<Status>; 100]> = OnceLock::new();

    fn tb() -> &'static [Vec<Status>; 100] {
        TB.get_or_init(|| {
            if let Ok(loaded) = load_tablebase("tablebase") {
                return loaded;
            }
            Pos::generator()
        })
    }

    fn probe(pos: &Pos) -> Status {
        let tb = tb();
        let idx = pos.index();
        let file = pos.file_idx();
        if idx < tb[file].len() && tb[file][idx] != Status::UNKOWN { tb[file][idx] } else { Status(0) }
    }

    fn file_of(p1: Piece, p2: Option<Piece>) -> usize {
        p1 as usize + 5 * p2.map_or(12, |p| p as usize)
    }

    fn pos_to_chess(pos: &Pos) -> Option<shakmaty::Chess> {
        use shakmaty::{Bitboard, Board as ShakmBoard, CastlingMode, Chess, Color, FromSetup, Piece as ShakmPiece, Role, Setup, Square as ShakmSquare};
        use std::num::NonZeroU32;

        // Both engines use #[repr(u8)] with A1=0..H8=63 (rank*8+file), so transmute is safe.
        let sq = |s: Square| -> ShakmSquare { unsafe { std::mem::transmute::<u8, ShakmSquare>(s as u8) } };
        let pc = |p: Piece| -> ShakmPiece {
            ShakmPiece {
                color: match p.colour() {
                    Colour::White => Color::White,
                    Colour::Black => Color::Black,
                },
                role: match p {
                    Piece::WhitePawn   | Piece::BlackPawn   => Role::Pawn,
                    Piece::WhiteKnight | Piece::BlackKnight => Role::Knight,
                    Piece::WhiteBishop | Piece::BlackBishop => Role::Bishop,
                    Piece::WhiteRook   | Piece::BlackRook   => Role::Rook,
                    Piece::WhiteQueen  | Piece::BlackQueen  => Role::Queen,
                    Piece::WhiteKing   | Piece::BlackKing   => Role::King,
                },
            }
        };

        let mut board = ShakmBoard::empty();
        board.set_piece_at(sq(pos.king[Colour::White]), pc(Piece::WhiteKing));
        board.set_piece_at(sq(pos.king[Colour::Black]), pc(Piece::BlackKing));
        board.set_piece_at(sq(pos.p1.0), pc(pos.p1.1));
        if let Some(p2) = pos.p2 {
            board.set_piece_at(sq(p2.0), pc(p2.1));
        }

        let setup = Setup {
            board,
            promoted: Bitboard::EMPTY,
            pockets: None,
            turn: if pos.last_moved == Colour::White { Color::Black } else { Color::White },
            castling_rights: Bitboard::EMPTY,
            ep_square: None,
            remaining_checks: None,
            halfmoves: 0,
            fullmoves: NonZeroU32::new(1).unwrap(),
        };

        Chess::from_setup(setup, CastlingMode::Standard).ok()
    }

    #[test]
    fn test_insufficient_material_empty() {
        let tb = tb();
        for piece in [Piece::WhiteKnight, Piece::WhiteBishop] {
            let f = file_of(piece, None);
            assert!(tb[f].is_empty(), "file {f} should be empty after trim (no forced mates)");
        }
    }

    // WK=f6,BK=h8,WQ=g7 is checkmate; canonical form (horizontal+vertical flip): WK=c2,BK=a1,WQ=b2.
    // Verify: BK in check from WQ diagonally; a2 controlled by WQ (rank), b1 by WK, b2 by WK.
    #[test]
    fn test_kqk_checkmate() {
        let pos = Pos {
            king: [Square::c2, Square::a1],
            p1: (Square::b2, Piece::WhiteQueen),
            p2: None,
            last_moved: Colour::White,
        };
        assert_eq!(probe(&pos), Status::CHECKMATED);
    }

    #[test]
    fn test_kqk_win_in_1() {
        let pos = Pos {
            king: [Square::c1, Square::a1],
            p1: (Square::h2, Piece::WhiteQueen),
            p2: None,
            last_moved: Colour::Black,
        };
        assert_eq!(probe(&pos), Status(2));
    }

    #[test]
    fn test_krk_checkmate() {
        let pos = Pos {
            king: [Square::c2, Square::a1],
            p1: (Square::a8, Piece::WhiteRook),
            p2: None,
            last_moved: Colour::White,
        };
        assert_eq!(probe(&pos), Status::CHECKMATED);
    }

    #[test]
    fn test_kqk_black_never_wins() {
        let tb = tb();
        for (idx, &s) in tb[file_of(Piece::WhiteQueen, None)].iter().enumerate() {
            if idx % 2 == 1 && s != Status::UNKOWN {
                assert!(!s.is_win(), "KQk idx={idx}: Black to move should not be win (status={})", s.0);
            }
        }
    }

    #[test]
    fn test_krk_black_never_wins() {
        let tb = tb();
        for (idx, &s) in tb[file_of(Piece::WhiteRook, None)].iter().enumerate() {
            if idx % 2 == 1 && s != Status::UNKOWN {
                assert!(!s.is_win(), "KRk idx={idx}: Black to move should not be win (status={})", s.0);
            }
        }
    }

    /// Compares every canonical position against Syzygy WDL tables.
    /// Set the SYZYGY_PATH environment variable to a directory containing .rtbw files.
    /// Run with: SYZYGY_PATH=syzygy cargo test --release -- test_syzygy_comparison --nocapture
    /// Positions where Syzygy does not have the relevant file are silently skipped.
    #[test]
    fn test_syzygy_comparison() {
        use shakmaty::Chess;
        use shakmaty_syzygy::{AmbiguousWdl, Tablebase};

        let syzygy_path = "./syzygy/";

        let syzygy = {
            let mut t: Tablebase<Chess> = Tablebase::new();
            t.add_directory(&syzygy_path).expect("failed to load Syzygy directory");
            t
        };

        let mut mismatches = 0u64;
        let mut checked = 0u64;

        for white_king in Square::all() {
            if white_king.file() >= 4 { continue; }
            for black_king in Square::all() {
                if KINGS_IDX_PAWNFUL[white_king][black_king] == u16::MAX { continue; }
                let king = [white_king, black_king];
                for square1 in Square::all() {
                    if square1 == white_king || square1 == black_king { continue; }
                    for kind1 in [Piece::WhitePawn, Piece::WhiteKnight, Piece::WhiteBishop, Piece::WhiteRook, Piece::WhiteQueen] {
                        if kind1 == Piece::WhitePawn && (square1.rank() == 0 || square1.rank() == 7) { continue; }
                        for square2 in Square::all() {
                            for kind2 in [None, Some(Piece::WhitePawn), Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen), Some(Piece::BlackPawn), Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)] {
                                if kind2 == None && square2 != Square::a1 { continue; }
                                if kind2.is_some() && (square2 == square1 || square2 == white_king || square2 == black_king) { continue; }
                                if kind2.is_some() && kind2.unwrap().abs_regular_value() > kind1.abs_regular_value() { continue; }
                                if kind2 == Some(Piece::WhitePawn) || kind2 == Some(Piece::BlackPawn) {
                                    if square2.rank() == 0 || square2.rank() == 7 { continue; }
                                } else if kind1 != Piece::WhitePawn {
                                    if white_king as usize >= 32 || KINGS_IDX_PAWNLESS[white_king as usize][black_king] == u16::MAX { continue; }
                                }
                                let p2 = kind2.map(|k| (square2, k));
                                for last_moved in [Colour::White, Colour::Black] {
                                    let mut pos = Pos { king, p1: (square1, kind1), p2, last_moved };
                                    pos.correct_reflection();
                                    if pos.in_check_simple(pos.last_moved) { continue; }

                                    let our = probe(&pos);
                                    let chess = match pos_to_chess(&pos) {
                                        Some(c) => c,
                                        None => continue,
                                    };

                                    let wdl = match syzygy.probe_wdl(&chess) {
                                        Ok(w) => w,
                                        Err(_) => continue,
                                    };

                                    let syzygy_win  = matches!(wdl, AmbiguousWdl::Win | AmbiguousWdl::CursedWin | AmbiguousWdl::MaybeWin);
                                    let syzygy_loss = matches!(wdl, AmbiguousWdl::Loss | AmbiguousWdl::BlessedLoss | AmbiguousWdl::MaybeLoss);
                                    checked += 1;

                                    if our.is_win() != syzygy_win || our.is_loss() != syzygy_loss {
                                        mismatches += 1;
                                        if mismatches <= 20 {
                                            eprintln!("MISMATCH  WK={} BK={} {}{} p2={}{} lm={}  ours={}  syzygy={:?}",
                                                white_king.to_fen(), black_king.to_fen(),
                                                kind1.to_fen(), square1.to_fen(),
                                                kind2.map_or("-".to_string(), |k| k.to_fen()),
                                                kind2.map_or(String::new(), |_| square2.to_fen()),
                                                last_moved.to_fen(), our.0, wdl);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(mismatches, 0, "{mismatches}/{checked} positions disagree with Syzygy");
    }

    fn chess_to_pos(chess: &shakmaty::Chess) -> Option<Pos> {
        use shakmaty::{Color, Position as ShakmPos, Role, Square as ShakmSquare};
        let board = chess.board();
        let last_moved = match chess.turn() {
            Color::White => Colour::Black,
            Color::Black => Colour::White,
        };
        let our_sq = |s: ShakmSquare| -> Square { unsafe { std::mem::transmute::<u8, Square>(s as u8) } };
        let our_piece = |pc: shakmaty::Piece| -> Piece {
            match (pc.color, pc.role) {
                (Color::White, Role::King)   => Piece::WhiteKing,
                (Color::Black, Role::King)   => Piece::BlackKing,
                (Color::White, Role::Pawn)   => Piece::WhitePawn,
                (Color::Black, Role::Pawn)   => Piece::BlackPawn,
                (Color::White, Role::Knight) => Piece::WhiteKnight,
                (Color::Black, Role::Knight) => Piece::BlackKnight,
                (Color::White, Role::Bishop) => Piece::WhiteBishop,
                (Color::Black, Role::Bishop) => Piece::BlackBishop,
                (Color::White, Role::Rook)   => Piece::WhiteRook,
                (Color::Black, Role::Rook)   => Piece::BlackRook,
                (Color::White, Role::Queen)  => Piece::WhiteQueen,
                (Color::Black, Role::Queen)  => Piece::BlackQueen,
            }
        };
        let mut wk = None;
        let mut bk = None;
        let mut others: Vec<(Square, Piece)> = Vec::new();
        for sq_s in ShakmSquare::ALL {
            if let Some(pc) = board.piece_at(sq_s) {
                let s = our_sq(sq_s);
                let p = our_piece(pc);
                match p {
                    Piece::WhiteKing => wk = Some(s),
                    Piece::BlackKing => bk = Some(s),
                    _ => others.push((s, p)),
                }
            }
        }
        let wk = wk?;
        let bk = bk?;
        if others.is_empty() || others.len() > 2 { return None; }
        if others.len() == 2 {
            let v0 = others[0].1.abs_regular_value();
            let v1 = others[1].1.abs_regular_value();
            if v0 < v1 || (v0 == v1 && others[0].1.colour() == Colour::Black) {
                others.swap(0, 1);
            }
        }
        let p1 = others[0];
        let p2 = others.get(1).copied();
        let mut pos = Pos { king: [wk, bk], p1, p2, last_moved };
        if pos.p1.1.colour() == Colour::Black { pos.colour_swap(); }
        if let Some(p2v) = pos.p2 {
            if p2v.1.abs_regular_value() > pos.p1.1.abs_regular_value() {
                let tmp = pos.p1; pos.p1 = p2v; pos.p2 = Some(tmp);
                if pos.p1.1.colour() == Colour::Black { pos.colour_swap(); }
            }
        }
        pos.correct_reflection();
        Some(pos)
    }

    fn pos_str(pos: &Pos) -> String {
        let p2 = pos.p2.map_or(String::new(), |(sq, pc)| format!(" {}{}", pc.to_fen(), sq.to_fen()));
        format!("WK={} BK={} {}{}{} lm={}",
            pos.king[Colour::White].to_fen(), pos.king[Colour::Black].to_fen(),
            pos.p1.1.to_fen(), pos.p1.0.to_fen(), p2, pos.last_moved.to_fen())
    }

    fn trace_wrong(pos: &Pos, syzygy: &shakmaty_syzygy::Tablebase<shakmaty::Chess>, depth: usize) {
        use shakmaty::Position as ShakmPos;
        use shakmaty_syzygy::AmbiguousWdl;
        let prefix = "  ".repeat(depth);
        let our = probe(pos);
        let chess = match pos_to_chess(pos) {
            Some(c) => c,
            None => { eprintln!("{prefix}IMPOSSIBLE_CHECK {}", pos_str(pos)); return; }
        };
        let wdl = match syzygy.probe_wdl(&chess) {
            Ok(w) => w, Err(_) => { eprintln!("{prefix}SYZYGY_ERR {}", pos_str(pos)); return; }
        };
        let syzygy_win  = matches!(wdl, AmbiguousWdl::Win | AmbiguousWdl::CursedWin | AmbiguousWdl::MaybeWin);
        let syzygy_loss = matches!(wdl, AmbiguousWdl::Loss | AmbiguousWdl::BlessedLoss | AmbiguousWdl::MaybeLoss);
        let wrong = our.is_win() != syzygy_win || our.is_loss() != syzygy_loss;
        eprintln!("{prefix}{} ours={} syzygy={:?} {}", pos_str(pos), our.0, wdl,
            if wrong { "WRONG" } else { "ok" });
        if !wrong || depth >= 8 { return; }
        let mut first_loss_mv = None;   // for wrong-Win: follow the syzygy-Loss child
        let mut first_not_win_mv = None; // for wrong-Loss: follow the first child our EGTB doesn't mark as Win
        for mv in chess.legal_moves() {
            let mut child_chess = chess.clone();
            child_chess.play_unchecked(&mv);
            let child_wdl = match syzygy.probe_wdl(&child_chess) {
                Ok(w) => w,
                Err(e) => { eprintln!("{prefix}  mv={mv} syzygy_err={e:?}"); continue; }
            };
            let child_pos = chess_to_pos(&child_chess);
            let child_our = child_pos.as_ref().map_or(Status(0), |p| probe(p));
            let child_syzygy_win  = matches!(child_wdl, AmbiguousWdl::Win | AmbiguousWdl::CursedWin | AmbiguousWdl::MaybeWin);
            let child_syzygy_loss = matches!(child_wdl, AmbiguousWdl::Loss | AmbiguousWdl::BlessedLoss | AmbiguousWdl::MaybeLoss);
            let child_wrong = child_our.is_win() != child_syzygy_win || child_our.is_loss() != child_syzygy_loss;
            eprintln!("{prefix}  mv={mv} ours={} syzygy={:?}{}", child_our.0, child_wdl,
                if child_wrong { " WRONG" } else { "" });
            if child_syzygy_loss && first_loss_mv.is_none() {
                first_loss_mv = Some((mv.clone(), child_pos.clone()));
            }
            // For a wrong-Loss: any child our EGTB doesn't mark as Win is the escape preventing Loss propagation
            if syzygy_loss && !child_our.is_win() && first_not_win_mv.is_none() {
                first_not_win_mv = Some((mv, child_pos));
            }
        }
        let to_trace = if syzygy_win {
            first_loss_mv
        } else if syzygy_loss {
            if first_not_win_mv.is_none() {
                eprintln!("{prefix}  (all children correctly Win in our EGTB - retrograde propagation bug)");
            }
            first_not_win_mv
        } else {
            None
        };
        if let Some((mv, Some(child_pos))) = to_trace {
            eprintln!("{prefix}-> tracing {mv}:");
            trace_wrong(&child_pos, syzygy, depth + 1);
        }
    }

    #[test]
    fn test_trace_wrong_position() {
        use shakmaty::Chess;
        use shakmaty_syzygy::Tablebase;
        let syzygy_path = "./syzygy/";
        let syzygy: Tablebase<Chess> = {
            let mut t = Tablebase::new(); t.add_directory(&syzygy_path).unwrap(); t
        };
        // MISMATCH  WK=a1 BK=c1 Rb1 p2=Bd1 lm=w  ours=0  syzygy=Loss
        let pos = Pos {
            king: [Square::a1, Square::c1],
            p1: (Square::b1, Piece::WhiteRook),
            p2: Some((Square::d1, Piece::WhiteBishop)),
            last_moved: Colour::White,
        };
        trace_wrong(&pos, &syzygy, 0);
    }

    #[test]
    fn test_debug_unpromotion() {
        use crate::repr::board::Board;
        // After Kd2-d3, Ka1-b2, g2-g1=Q: WK=b2 WN=b1 BK=d3 BQ=g1, White to move
        let board = Board::from_fen("8/8/8/8/8/3k4/1K6/1N4q1 w - - 0 1");
        let pos = Pos::new(&board);
        eprintln!("raw pos: WK={} BK={} p1={}@{} p2={} lm={}",
            pos.king[Colour::White].to_fen(), pos.king[Colour::Black].to_fen(),
            pos.p1.1.to_fen(), pos.p1.0.to_fen(),
            pos.p2.map_or("-".into(), |(sq, pc)| format!("{}@{}", pc.to_fen(), sq.to_fen())),
            pos.last_moved.to_fen());
        eprintln!("canonical pos: {} probe={}", pos_str(&pos), probe(&pos).0);
        let revmovelist = pos.make_revmovelist();
        eprintln!("revmovelist length: {}", revmovelist.length);
        let mut found_unprom = false;
        for i in 0..revmovelist.length {
            let rm = revmovelist.list[i];
            if rm.is_unpromotion() {
                found_unprom = true;
                eprintln!("  unpromotion [{}]: to={} moving={} diagonal={}", i, rm.to.to_fen(), rm.moving_piece() as u8, rm.unpromote_diagonal());
                let mut next = pos.clone();
                next.make_revmove(rm);
                eprintln!("  -> result: {} probe={}", pos_str(&next), probe(&next).0);
                // Verify: the forward revmovelist of `next` should contain pos
                eprintln!("  next: {} FEN={}", pos_str(&next), next.to_board_partial().to_fen());
                let next_revlist = next.make_revmovelist();
                for j in 0..next_revlist.length {
                    if !next_revlist.list[j].is_unpromotion() { continue; }
                    let mut candidate = next.clone();
                    candidate.make_revmove(next_revlist.list[j]);
                    let matches = candidate == pos;
                    eprintln!("    fwd-unpromotion [{}] -> {} FEN={} {}",
                        j, pos_str(&candidate), candidate.to_board_partial().to_fen(),
                        if matches { "MATCH" } else { "NO MATCH" });
                }
                eprintln!("  target: {} FEN={}", pos_str(&pos), pos.to_board_partial().to_fen());
            }
        }
        if !found_unprom {
            eprintln!("  NO unpromotion revmove found!");
        }
    }

    // Diagnose why positions with WK=a1 BK=c1 stay Unknown.
    //
    // For a failing position P (lm=White → Black to move), we check:
    //   (1) initial moves_left[P] — did count_distinct_canonical_successors return the right count?
    //   (2) final status of each forward successor — are they ever resolved by the BFS?
    //   (3) P ∈ succ.make_revmovelist()? — does the BFS ever see P as a predecessor of succ?
    //
    // If (2) is ok but (3) is false → revmovelist generation skips P (root cause).
    // If (2) is false → the bug is deeper in that subtree.
    // If (1) is wrong → count_distinct_canonical_successors overcounts.
    #[test]
    fn test_retrograde_trace_a1c1() {
        let (mut moves_left, _, _) = Pos::init_backwards_gen();

        let roots: &[Pos] = &[
            // KRK — simplest failing case
            Pos { king: [Square::a1, Square::c1], p1: (Square::b1, Piece::WhiteRook), p2: None, last_moved: Colour::White },
            // KRBK — also in the failure cluster
            Pos { king: [Square::a1, Square::c1], p1: (Square::b1, Piece::WhiteRook), p2: Some((Square::d1, Piece::WhiteBishop)), last_moved: Colour::White },
        ];

        for p in roots {
            let ml_p = *p.index_file(&mut moves_left, u8::MAX);
            eprintln!("\n=== P: {} ===", p.to_board_partial().to_fen());
            eprintln!("    initial moves_left={} | final status={}", ml_p, probe(p).0);

            let mut board = p.to_board_partial();
            let movelist = board.generate_movelist(false);
            let p_hash = p.pos_hash();
            eprintln!("    {} raw legal moves:", movelist.length);

            for i in 0..movelist.length {
                let mv = movelist[i];
                let unmake = board.makemove(mv);
                let remaining = board.occupied()
                    & !(board[Piece::WhiteKing] | board[Piece::BlackKing]);

                if remaining.count_ones() == 0 {
                    eprintln!("      succ[{i}]: K-vs-K draw");
                    board.unmakemove(mv, unmake);
                    continue;
                }

                let succ = Pos::new(&board);
                let succ_ml = *succ.index_file(&mut moves_left, u8::MAX);
                let succ_status = probe(&succ);
                eprintln!("      succ[{i}]: {} | ml={succ_ml} status={}",
                    succ.to_board_partial().to_fen(), succ_status.0);

                // Check whether P appears in succ's revmovelist.
                let revml = succ.make_revmovelist();
                let mut p_found = false;
                let mut valid_preds = 0usize;
                for j in 0..revml.length {
                    let mut pred = succ.clone();
                    pred.make_revmove(revml.list[j]);
                    if pred.in_check_simple(pred.last_moved) { continue; }
                    valid_preds += 1;
                    if pred.pos_hash() == p_hash {
                        p_found = true;
                    }
                }
                eprintln!("        revml: {} raw, {valid_preds} valid preds, P_found={p_found}",
                    revml.length);

                board.unmakemove(mv, unmake);
            }
        }

        assert!(false, "diagnostic complete — see output above");
    }
}