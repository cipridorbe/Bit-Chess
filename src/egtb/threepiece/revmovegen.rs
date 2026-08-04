use crate::{egtb::threepiece::{pos::Pos, reflection::{Reflection, reflect_bb, reflect_sq}, revmove::{Flag, MovingPiece, RevMove, RevMoveList}}, movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_knight_attacks, single_queen_attacks, single_rook_attacks}, tables::KING_ATTACKS}, repr::{bitboard::BB, colour::Colour, piece::Piece, square::Square}};

const WHITE: [Option<Piece>; 6] = [None, Some(Piece::WhitePawn), Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen)];
const BLACK: [Option<Piece>; 6] = [None, Some(Piece::BlackPawn), Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)];
const UNCAPTURES: [[Option<Piece>; 6]; 2] = [BLACK, WHITE];

const WHITE_SOME: [Option<Piece>; 5] = [Some(Piece::WhitePawn), Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen)];
const BLACK_SOME: [Option<Piece>; 5] = [Some(Piece::BlackPawn), Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)];
const UNCAPTURES_SOME: [[Option<Piece>; 5]; 2] = [BLACK_SOME, WHITE_SOME];

const WHITE_PAWNLESS_SOME: [Option<Piece>; 4] = [Some(Piece::WhiteKnight), Some(Piece::WhiteBishop), Some(Piece::WhiteRook), Some(Piece::WhiteQueen)];
const BLACK_PAWNLESS_SOME: [Option<Piece>; 4] = [Some(Piece::BlackKnight), Some(Piece::BlackBishop), Some(Piece::BlackRook), Some(Piece::BlackQueen)];
const UNCAPTURES_PAWNLESS_SOME: [[Option<Piece>; 4]; 2] = [BLACK_PAWNLESS_SOME, WHITE_PAWNLESS_SOME];

impl Pos {
    pub(crate) fn generate_revmovelist(&self) -> RevMoveList {
        let mut out = RevMoveList::new();
        let colour = self.last_moved;
        let moving = [MovingPiece::P1, MovingPiece::P2, MovingPiece::P3];
        if let Some(enpassant) = self.enpassant {
            let (source, target) = match colour {
                Colour::White => (Square::from_u8(enpassant as u8 + 8), Square::from_u8(enpassant as u8 - 8)),
                Colour::Black => (Square::from_u8(enpassant as u8 - 8), Square::from_u8(enpassant as u8 + 8)),
            };
            for (i, &p) in [Some(self.p1), self.p2, self.p3].iter().enumerate() {
                if p.is_none() {
                    return out;
                }
                let (square, piece) = p.unwrap();
                if square != source {
                    continue;
                }
                out.add(RevMove::new_quiet(target, None, moving[i]));
                return out;
            }
            return out;
        }
        let is_full = self.p3.is_some();
        self.generate_king_moves(&mut out, !is_full);
        self.generate_piece_moves(&mut out, MovingPiece::P1);
        self.generate_piece_moves(&mut out, MovingPiece::P2);
        self.generate_piece_moves(&mut out, MovingPiece::P3);
        out
    }

    #[inline]
    fn generate_piece_moves(&self, revmovelist: &mut RevMoveList, moving: MovingPiece) {
        let Some((source, piece)) = (match moving {
            MovingPiece::P1 => Some(self.p1),
            MovingPiece::P2 => self.p2,
            MovingPiece::P3 => self.p3,
            _ => panic!("cannot do king moves in piece moves")
        }) else {
            return;
        };
        if piece.colour() != self.last_moved {
            return;
        }
        let occupied = self.occupied();
        let include_uncaptures = self.p3.is_none();
        let colour = self.last_moved;
        if !piece.is_pawn() {
            let targets = Pos::non_pawn_king_targets(source, piece, occupied);
            self.add_non_pawn_king_moves(revmovelist, targets, source, include_uncaptures, moving);
            self.generate_unpromotions(revmovelist, source, moving, include_uncaptures);
        } else {
            let start_rank = if colour == Colour::White { 1 } else { 6 };
            if source.rank() == start_rank {
                return;
            }
            let (double_push_rank, ep_capture_rank) = match colour {
                Colour::White => (3, 5),
                Colour::Black => (4, 2)
            };
            let square = source;
            let (back, back2, captures) = match colour {
                Colour::White => (Square::from_u8(square as u8 - 8), Square::from_u8(square as u8 - 16), pawn_attacks(square.bb(), Colour::Black)),
                Colour::Black => (Square::from_u8(square as u8 + 8), Square::from_u8(square as u8 + 16), pawn_attacks(square.bb(), Colour::White)),
            };
            if back.bb() & occupied == 0 {
                revmovelist.add(RevMove::new_quiet(back, None, moving));
                if square.rank() == double_push_rank && back2.bb() & occupied == 0 && !self.enpassant_possible(back, colour){
                    revmovelist.add(RevMove::new_quiet(back2, None, moving));
                }
            }
            if include_uncaptures {
                for target in (captures & !occupied).squares() {
                    for uncaptured in UNCAPTURES_SOME[colour] {
                        revmovelist.add(RevMove::new_quiet(target, uncaptured, moving));
                    }
                }
                for target in Pos::unenpassant_targets(square, colour, occupied).squares() {
                    revmovelist.add(RevMove::new_full(target, Some(Piece::pawn(!colour)), moving, Flag::Enpassant, Some(square), None));
                }
            }
        }
    }

    #[inline]
    fn generate_king_moves(&self, revmovelist: &mut RevMoveList, include_uncaptures: bool) {
        let is_pawnful = self.is_pawnful();
        let colour = self.last_moved;
        let occupied = self.occupied();
        let moving = if colour == Colour::White { MovingPiece::WhiteKing } else { MovingPiece::BlackKing };
        let king = self.king[colour];
        let other_king = self.king[!colour];
        let targets = KING_ATTACKS[king] & !KING_ATTACKS[other_king] & !occupied;
        for target in targets.squares() {
            if include_uncaptures {
                for uncaptured in UNCAPTURES[colour] {
                    let revmove = RevMove::new_quiet(target, uncaptured, moving);
                    Pos::add_with_reflections(revmovelist, revmove, king, is_pawnful);
                }
            } else {
                revmovelist.add(RevMove::new_quiet(target, None, moving));
            }
        }
    }

    #[inline]
    fn add_non_pawn_king_moves(&self, revmovelist: &mut RevMoveList, targets: BB, source: Square, include_uncaptures: bool, moving: MovingPiece) {
        let is_pawnful = self.is_pawnful();
        for target in targets.squares() {
            if include_uncaptures {
                for uncaptured in UNCAPTURES[self.last_moved] {
                    let revmove = RevMove::new_quiet(target, uncaptured, moving);
                    Pos::add_with_reflections(revmovelist, revmove, source, is_pawnful);
                }
            } else {
                revmovelist.add(RevMove::new_quiet(target, None, moving));
            }
        }
    }

    #[inline]
    fn non_pawn_king_targets(square: Square, piece: Piece, occupied: BB) -> BB {
        match piece {
            Piece::WhiteKnight | Piece::BlackKnight => single_knight_attacks(square),
            Piece::WhiteBishop | Piece::BlackBishop => single_bishop_attacks(square, occupied),
            Piece::WhiteRook | Piece::BlackRook => single_rook_attacks(square, occupied),
            Piece::WhiteQueen | Piece::BlackQueen => single_queen_attacks(square, occupied),
            _ => panic!("Cannot have pawn/king in non_pawn_king targets")
        }
    }

    fn generate_unpromotions(&self, revmovelist: &mut RevMoveList, square: Square, moving: MovingPiece, include_uncaptures: bool) {
        if square & Pos::EDGES == 0 {
            return;
        }
        let colour = self.last_moved;
        let is_pawnful = self.is_pawnful();
        let occupied = self.occupied();
        let target_rank = if colour == Colour::White { 7 } else { 0 };
        let (rank, file) = square.rank_file();
        if is_pawnful {
            if rank != target_rank {
                return;
            }
            Self::add_unpromotions(revmovelist, colour, square, occupied, moving, None, include_uncaptures);
        } else {
            let reflection = if rank == target_rank { None }
                else if rank == 7 - target_rank { Some(Reflection::Vertical) }
                else if file == target_rank { Some(Reflection::Diagonal) }
                else { Some(Reflection::Rotation) };
            Self::add_unpromotions(revmovelist, colour, square, occupied, moving, reflection, include_uncaptures);
            if square & Pos::CORNERS != 0 {
                let reflection =
                    if file == target_rank { Some(Reflection::Diagonal) }
                    else { Some(Reflection::Rotation) };
                Self::add_unpromotions(revmovelist, colour, square, occupied, moving, reflection, include_uncaptures);
            }
        }
    }

    #[inline]
    fn add_unpromotions(revmovelist: &mut RevMoveList, colour: Colour, square: Square, occupied: BB, moving: MovingPiece, reflection: Option<Reflection>, include_uncaptures: bool) {
        let square = reflect_sq(square, reflection);
        let occupied = reflect_bb(occupied, reflection);
        let (push, captures) = match colour {
            Colour::White => (Square::from_u8(square as u8 - 8), pawn_attacks(square.bb(), Colour::Black)),
            Colour::Black => (Square::from_u8(square as u8 + 8), pawn_attacks(square.bb(), Colour::White)),
        };
        if push.bb() & occupied == 0 {
            revmovelist.add(RevMove::new_full(push, None, moving, Flag::Promotion, None, reflection));
        }
        if include_uncaptures {
            for square in (captures & !occupied).squares() {
                for uncaptured in UNCAPTURES_PAWNLESS_SOME[colour] {
                    revmovelist.add(RevMove::new_full(square, uncaptured, moving, Flag::Promotion, None, reflection));
                } 
            }
        }
    }

    #[inline]
    fn add_with_reflections(revmovelist: &mut RevMoveList, mut revmove: RevMove, source: Square, is_pawnful: bool) {
        if revmove.uncaptured.is_none_or(|p| !p.is_pawn()) || revmove.flag != Flag::Quiet {
            revmovelist.add(revmove);
            return;
        }
        if is_pawnful {
            if source.rank() != 0 && source.rank() != 7 {
                revmovelist.add(revmove);
            }
        } else {
            for reflection in [None, Some(Reflection::Vertical), Some(Reflection::Diagonal), Some(Reflection::Rotation)] {
                let new_square = reflect_sq(source, reflection);
                if new_square.rank() != 0 && new_square.rank() != 7 {
                    revmove.reflection = reflection;
                    revmovelist.add(revmove);
                }
            }
        }
    }
}