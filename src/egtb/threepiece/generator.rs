use std::{collections::VecDeque, u8};

use crate::{egtb::{KINGS_IDX_PAWNFUL, threepiece::{Files, pos::Pos}}, repr::{colour::Colour, piece::Piece, square::Square}, test_assert};

impl Pos {
    pub fn generate() -> Files<Status> {
        let (mut moves_left, mut status, mut queue) = Self::init();
        while let Some(pos) = queue.pop_front() {
            let state = *get_and_fill(&mut status[pos.file()], pos.index(), Status::UNKOWN);
            let next_state = state.next();
            for new_pos in pos.predecessors() {
                let file = new_pos.file();
                let index = new_pos.index();
                let left = get_and_fill(&mut moves_left[file], index, u8::MAX);
                test_assert!(*left > 0);
                test_assert!(*left != u8::MAX);
                *left -= 1;
                if state.is_loss() || *left == 0 {
                    let current_state = get_and_fill(&mut status[file], index, Status::UNKOWN);
                    if *current_state == Status::UNKOWN {
                        *current_state = next_state;
                        queue.push_back(new_pos);
                    }
                }
            }
        }
        status
    }

    fn init() -> (Files<u8>, Files<Status>, VecDeque<Pos>) {
        let mut moves_left = [const { Vec::new() }; Pos::NUM_FILES];
        let mut status = [const { Vec::new() }; Pos::NUM_FILES];
        let mut queue = VecDeque::new();
        for last_moved in Self::last_moved_iter() {
            for king in Self::king_iter() {
                for p1 in Self::p1_iter(king) {
                    for p2 in Self::p2_iter(king, p1) {
                        for p3 in Self::p3_iter(king, p1, p2) {
                            let mut pos = Pos { last_moved, king, p1, p2, p3, enpassant: None };
                            let hash = pos.unique_hash();
                            pos.make_canonical();
                            if hash != pos.unique_hash() || pos.in_check(pos.last_moved) {
                                continue;
                            }
                            for enpassant in Self::enpassant_iter(pos.clone()) {
                                pos.enpassant = enpassant;
                                let file = pos.file();
                                let index = pos.index();
                                let num_moves = pos.count_distinct_canonical_successors() as u8;
                                insert_with_default(&mut moves_left[file], index, num_moves, u8::MAX);
                                if num_moves == 0 {
                                    if pos.in_check(!pos.last_moved) {
                                        insert_with_default(&mut status[file], index, Status::CHECKMATED, Status::UNKOWN);
                                        queue.push_back(pos.clone());
                                    } else {
                                        insert_with_default(&mut status[file], index, Status::DRAW, Status::UNKOWN);
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

    // iterator over all valid last_moved values
    fn last_moved_iter() -> impl Iterator<Item = Colour> {
        [Colour::White, Colour::Black].into_iter()
    }

    // iterator over all valid king values
    fn king_iter() -> impl Iterator<Item = [Square; 2]> {
        Square::all()
            .filter(|wk| wk.file() < 4)
            .flat_map(|wk| Square::all()
                .filter(move |bk| KINGS_IDX_PAWNFUL[wk as usize][*bk] != u16::MAX)
                .map(move |bk| [wk, bk]))
    }

    // iterator over all valid p1 values, dependent on king values
    fn p1_iter(king: [Square; 2]) -> impl Iterator<Item = (Square, Piece)> {
        Square::all()
            .flat_map(|square| Piece::ALL.into_iter().map(move |piece| (square, piece)))
            .filter(move |(square, piece)| {
                piece.colour() == Colour::White && *square != king[0] && *square != king[1]
                && (
                    (*piece == Piece::WhitePawn && square.rank() > 0 && square.rank() < 7)
                    || *piece != Piece::WhitePawn
                )
            })
    }

    // iterator over all valid p2 values, dependent on king and p1
    fn p2_iter(king: [Square; 2], p1: (Square, Piece)) -> impl Iterator<Item = Option<(Square, Piece)>> {
        let above_diagonal = { let (rank, file) = king[Colour::White].rank_file(); rank > file };
        let some_iter = Square::all()
            .flat_map(|square| Piece::ALL.into_iter().map(move |piece| (square, piece)))
            .filter(move |(square, piece)| {
                *square != king[0] && *square != king[1] && *square != p1.0
                && p1.1.abs_regular_value() >= piece.abs_regular_value()
                && (
                    (piece.is_pawn() && square.rank() > 0 && square.rank() < 7)
                    || !piece.is_pawn()
                )
            })
            .map(|p2| Some(p2));
        let none_option = if above_diagonal && p1.1 != Piece::WhitePawn { None } else { Some(None) };
        none_option.into_iter().chain(some_iter)
    }

    // iterator over all valid p3 values, dependent on king, p1, and p2
    fn p3_iter(king: [Square; 2], p1: (Square, Piece), p2: Option<(Square, Piece)>) -> impl Iterator<Item = Option<(Square, Piece)>> {
        let above_diagonal = { let (rank, file) = king[Colour::White].rank_file(); rank > file };
        let has_pawn = p1.1 == Piece::WhitePawn || p2.is_some_and(|p2| p2.1.is_pawn());
        let some_iter = Square::all()
            .flat_map(|square| Piece::ALL.into_iter().map(move |piece| (square, piece)))
            .filter(move |(square, piece)| {
                *square != king[0] && *square != king[1] && *square != p1.0 && *square != p2.unwrap().0
                && p2.unwrap().1.abs_regular_value() >= piece.abs_regular_value()
                && (
                    (piece.is_pawn() && square.rank() > 0 && square.rank() < 7)
                    || (!piece.is_pawn() && !above_diagonal)
                )
            })
            .map(|p3| Some(p3));
        let none_option = if above_diagonal && !has_pawn { None } else { Some(None) };
        none_option.into_iter().chain(some_iter)
    }

    // at most 3 candidate enpassant squares, one per piece that could be the pawn that
    // just double-pushed (right colour, right rank)
    pub(crate) fn enpassant_candidates(pos: &Pos) -> [Option<Square>; 3] {
        let (ep_rank, pawn_rank, pawn_piece) = match pos.last_moved {
            Colour::White => (2, 3, Piece::WhitePawn),
            Colour::Black => (5, 4, Piece::BlackPawn),
        };
        let candidate = |p: (Square, Piece)| {
            let (rank, file) = p.0.rank_file();
            (rank == pawn_rank && p.1 == pawn_piece).then(|| Square::from_rank_file(ep_rank, file))
        };
        [candidate(pos.p1), pos.p2.and_then(candidate), pos.p3.and_then(candidate)]
    }

    // iterator over all valid enpassant values given a position
    fn enpassant_iter(pos: Pos) -> impl Iterator<Item = Option<Square>> {
        let some_iter = Self::enpassant_candidates(&pos).into_iter()
            .flatten()
            .filter(move |&sq| pos.enpassant_possible(sq, pos.last_moved))
            .map(Some);
        std::iter::once(None).chain(some_iter)
    }
}

fn insert_with_default<T: Copy>(file: &mut Vec<T>, index: usize, value: T, default: T) {
    if file.len() <= index {
        file.resize(index + 1, default);
    }
    file[index] = value;
}

fn get_and_fill<T: Copy>(file: &mut Vec<T>, index: usize, default: T) -> &mut T {
    if file.len() <= index {
        file.resize(index + 1, default);
    }
    &mut file[index]
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(i8);

impl Status {
    pub const CHECKMATED: Status = Status(-1);
    pub const DRAW: Status = Status(0);
    pub const UNKOWN: Status = Status(i8::MIN);

    fn next(self) -> Self {
        if self.0.abs() == 127 {
            Self(-self.0)
        } else if self.0 > 0 {
            Self(-self.0 - 1)
        } else if self.0 < 0 && self != Self::UNKOWN {
            Self(-self.0 + 1)
        } else {
            panic!("cannot call next on draw/unkown")
        }
    }

    pub fn is_win(self) -> bool {
        self.0 > 0
    }

    pub fn is_loss(self) -> bool {
        self.0 < 0 && self != Self::UNKOWN
    }
}