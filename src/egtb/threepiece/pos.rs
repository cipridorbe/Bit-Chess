use crate::{egtb::{KINGS_IDX_PAWNFUL, KINGS_IDX_PAWNLESS, NUM_KINGS_PAWNFUL, NUM_KINGS_PAWNLESS, threepiece::{reflection::Reflection, revmove::RevMoveList}}, movegen::{attacks::{knight_attacks, pawn_attacks}, r#move::MAX_MOVES}, repr::{bitboard::BB, board::{Board, BoardState}, castling::CastlingRights, colour::Colour, hash::Hash, piece::{Piece, PieceType}, square::{SEGMENT, SEGMENT_CARDINAL, SEGMENT_DIAGONAL, Square}}, test_assert};

// Removes already-placed squares from a raw square's numbering, giving a dense 0-based
// index. `exclude` must be squares (or pawn-offset values, if `value` is itself a pawn
// offset) always distinct from and comparable to `value`.
fn compact(value: usize, exclude: &[usize]) -> usize {
    value - exclude.iter().filter(|&&e| e < value).count()
}

// 2D triangular numbering for 0 <= a <= b < n (two identical pieces' compacted squares
// always satisfy this ordering, since compaction is order-preserving and canonicalization
// already enforces raw square order for identical pieces).
fn tri2(a: usize, b: usize, n: usize) -> usize {
    a * n - (a * a - a) / 2 + (b - a)
}

fn tetra(x: usize) -> usize {
    x * (x + 1) * (x + 2) / 6
}

// 3D "tetrahedral" numbering for 0 <= a <= b <= c < n (three identical pieces): peel off
// the a-layer via tetra(n)-tetra(n-a), then apply tri2 to the remaining (b,c) pair.
fn tri3(a: usize, b: usize, c: usize, n: usize) -> usize {
    tetra(n) - tetra(n - a) + tri2(b - a, c - a, n - a)
}

#[derive(Clone, PartialEq, Eq)]
pub struct Pos {
    pub king: [Square; 2],
    pub p1: (Square, Piece),
    pub p2: Option<(Square, Piece)>,
    pub p3: Option<(Square, Piece)>,
    pub last_moved: Colour,
    pub enpassant: Option<Square>,
}

impl Pos {
    pub const EDGES: BB = BB(Board::RANK_1.0 | Board::RANK_8.0 | Board::A_FILE.0 | Board::H_FILE.0);
    pub const CORNERS: BB = BB(Square::a1.bb().0 | Square::a8.bb().0 | Square::h1.bb().0 | Square::h8.bb().0);

    pub fn from_board(board: &Board) -> Self {
        let num_pieces = board.occupied().count_ones();
        if num_pieces > 5 {
            panic!("Pos can have at most 5 pieces, including kings");
        }
        if num_pieces <= 2 {
            panic!("Pos must have at least one non-king piece");
        }
        let king = [board[Piece::WhiteKing].lsb(), board[Piece::BlackKing].lsb()];
        let mut p = [None; 3];
        let mut i = 0;
        for square in (board.occupied() & !(king[0].bb() | king[1].bb())).squares() {
            p[i] = Some((square, board[square].unwrap()));
            i += 1;
        }
        let mut pos = Pos {
            king,
            p1: p[0].unwrap(),
            p2: p[1],
            p3: p[2],
            last_moved: !board.colour,
            enpassant: board.enpassant
        };
        pos.make_canonical();
        pos
    }

    // Converts a Pos to a Board, initializing the necessary fields
    // needed to generate the forward movelist
    fn into_board_partial(&self) -> Board {
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
                phase_unbounded: self.p1.1.phase_value()
                    + self.p2.map_or(0, |p| p.1.phase_value())
                    + self.p3.map_or(0, |p| p.1.phase_value()),
                xray_attacks: [BB::new(0); 2],
                pinners: [BB::new(0); 2]
            }
        };
        out.colour = !self.last_moved;
        out.enpassant = self.enpassant;

        out[Piece::WhiteKing] = self.king[Colour::White].bb();
        out[self.king[Colour::White]] = Some(Piece::WhiteKing);
        out[Piece::BlackKing] = self.king[Colour::Black].bb();
        out[self.king[Colour::Black]] = Some(Piece::BlackKing);
        for (sq, piece) in std::iter::once(self.p1).chain(self.p2).chain(self.p3) {
            out[piece] |= sq.bb();
            out[sq] = Some(piece);
        }

        out[Colour::White] = out[Piece::WhitePawn] | out[Piece::WhiteKnight] | out[Piece::WhiteBishop] | out[Piece::WhiteRook] | out[Piece::WhiteQueen] | out[Piece::WhiteKing];
        out[Colour::Black] = out[Piece::BlackPawn] | out[Piece::BlackKnight] | out[Piece::BlackBishop] | out[Piece::BlackRook] | out[Piece::BlackQueen] | out[Piece::BlackKing];

        out.state.attacks[Colour::White][PieceType::Leaper] = out.calculate_attacks(Colour::White, PieceType::Leaper);
        out.state.attacks[Colour::Black][PieceType::Leaper] = out.calculate_attacks(Colour::Black, PieceType::Leaper);
        out.state.attacks[Colour::White][PieceType::Slider] = out.calculate_attacks(Colour::White, PieceType::Slider);
        out.state.attacks[Colour::Black][PieceType::Slider] = out.calculate_attacks(Colour::Black, PieceType::Slider);

        out.state.checkers = out.calculate_checkers();

        let (white_xray, white_pinners) = out.compute_raw_xray_and_pinners(Colour::White);
        let (black_xray, black_pinners) = out.compute_raw_xray_and_pinners(Colour::Black);
        out.state.xray_attacks[Colour::White] = white_xray;
        out.state.xray_attacks[Colour::Black] = black_xray;
        out.state.pinners[Colour::Black] = white_pinners;
        out.state.pinners[Colour::White] = black_pinners;

        out
    }

    // Reflects the position by the given reflection
    #[inline]
    pub(crate) fn reflect(&mut self, reflection: Reflection) {
        self.king[0] = reflection.apply(self.king[0]);
        self.king[1] = reflection.apply(self.king[1]);
        self.p1.0 = reflection.apply(self.p1.0);
        if let Some(ref mut p2) = self.p2 {
            p2.0 = reflection.apply(p2.0);
            if let Some(ref mut p3) = self.p3 {
                p3.0 = reflection.apply(p3.0);
            }
            if let Some(enpassant) = self.enpassant {
                self.enpassant = Some(reflection.apply(enpassant));
            }
        }
    }

    // Whether an enpassant capture to ep_square is currently possible.
    pub(crate) fn enpassant_possible(&self, ep_square: Square, last_moved: Colour) -> bool {
        let (ep_rank, ep_file) = ep_square.rank_file();
        let (pawn_rank, origin_rank, pawn_piece, enemy_piece) = match last_moved {
            Colour::White if ep_rank == 2 => (3, 1, Piece::WhitePawn, Piece::BlackPawn),
            Colour::Black if ep_rank == 5 => (4, 6, Piece::BlackPawn, Piece::WhitePawn),
            _ => return false,
        };
        let pawn_square = Square::from_rank_file(pawn_rank, ep_file);
        let origin_square = Square::from_rank_file(origin_rank, ep_file);

        let pieces = || std::iter::once(self.p1).chain(self.p2).chain(self.p3);
        let has_own_pawn = pieces().any(|(sq, p)| sq == pawn_square && p == pawn_piece);
        let has_capturing_enemy = pieces().any(|(sq, p)| {
            p == enemy_piece && sq.rank() == pawn_rank && sq.file().abs_diff(ep_file) == 1
        });
        let occupied = self.occupied();

        has_own_pawn && has_capturing_enemy
            && occupied & ep_square.bb() == BB::new(0)
            && occupied & origin_square.bb() == BB::new(0)
    }

    #[inline]
    pub(crate) fn is_pawnful(&self) -> bool {
        self.p1.1.is_pawn()
        || self.p2.is_some_and(|p2| p2.1.is_pawn())
        || self.p3.is_some_and(|p3| p3.1.is_pawn())
    }

    pub(crate) fn file(&self) -> usize {
        let value = |p: Option<Piece>| match p {
            Some(p) => match p.colour() {
                Colour::White => p as usize,
                Colour::Black => p as usize - 1
            },
            None => 10,
        };
        value(Some(self.p1.1)) * 11 * 11
        + value(self.p2.map(|p| p.1)) * 11
        + value(self.p3.map(|p| p.1))
    }

    pub(crate) const NUM_FILES: usize = 5 * 11 * 11 + 10 * 11 + 10;

    pub(crate) fn index(&self) -> usize {
        let king_idx = if self.is_pawnful() {
            KINGS_IDX_PAWNFUL[self.king[Colour::White]][self.king[Colour::Black]]
        } else {
            KINGS_IDX_PAWNLESS[self.king[Colour::White] as usize][self.king[Colour::Black]]
        } as usize;
        let num_kings = if self.is_pawnful() { NUM_KINGS_PAWNFUL } else { NUM_KINGS_PAWNLESS };
        let side = !self.last_moved as usize;
        let wk = self.king[Colour::White] as usize;
        let bk = self.king[Colour::Black] as usize;
        let sq1 = self.p1.0 as usize;

        if self.p2.is_none() {
            let (range, idx) = if self.p1.1 == Piece::WhitePawn {
                (48, sq1 - 8)
            } else {
                (62, compact(sq1, &[wk, bk]))
            };
            return side * (num_kings * range) + king_idx * range + idx;
        }

        let p2 = self.p2.unwrap();
        let sq2 = p2.0 as usize;

        if self.p3.is_none() {
            let p1_pawn = self.p1.1 == Piece::WhitePawn;
            let p2_pawn = p2.1.is_pawn();

            // p1 pawn (lowest value) forces p2 pawn too if p2 outranks it in value order,
            // so "p1 pawn, p2 non-pawn" can never occur -- no branch needed for it.
            if p1_pawn && p2_pawn {
                let p1c = sq1 - 8;
                let p2_off = sq2 - 8;
                let p2c = p2_off - (p2_off > p1c) as usize;
                const P2_RANGE: usize = 47;
                const BASE_SIZE: usize = 48 * P2_RANGE;
                const EP_EXTRA: usize = 16;
                if p2.1 == Piece::WhitePawn {
                    const TRI_SIZE: usize = P2_RANGE * (P2_RANGE + 1) / 2;
                    let tri = tri2(p1c, p2c, P2_RANGE);
                    return side * (num_kings * TRI_SIZE) + king_idx * TRI_SIZE + tri;
                } else if self.enpassant.is_none() {
                    return side * (num_kings * (BASE_SIZE + EP_EXTRA)) + king_idx * (BASE_SIZE + EP_EXTRA) + p1c * P2_RANGE + p2c;
                } else {
                    let file1 = self.p1.0.file() as usize;
                    let file2 = p2.0.file() as usize;
                    let dir = (file2 > file1) as usize;
                    return side * (num_kings * (BASE_SIZE + EP_EXTRA)) + king_idx * (BASE_SIZE + EP_EXTRA) + BASE_SIZE + file1 * 2 + dir;
                }
            } else if p2_pawn {
                let p1c = compact(sq1, &[wk, bk]);
                let p2c = sq2 - 8;
                const P1_RANGE: usize = 62;
                const P2_RANGE: usize = 48;
                return side * (num_kings * P1_RANGE * P2_RANGE) + king_idx * (P1_RANGE * P2_RANGE) + p1c * P2_RANGE + p2c;
            } else {
                let p1c = compact(sq1, &[wk, bk]);
                let p2c = compact(sq2, &[wk, bk, sq1]);
                if self.p1.1 == p2.1 {
                    const N: usize = 61;
                    const TRI_SIZE: usize = N * (N + 1) / 2;
                    let tri = tri2(p1c, p2c, N);
                    return side * (num_kings * TRI_SIZE) + king_idx * TRI_SIZE + tri;
                } else {
                    const P1_RANGE: usize = 62;
                    const P2_RANGE: usize = 61;
                    return side * (num_kings * P1_RANGE * P2_RANGE) + king_idx * (P1_RANGE * P2_RANGE) + p1c * P2_RANGE + p2c;
                }
            }
        }

        let p3 = self.p3.unwrap();
        let sq3 = p3.0 as usize;
        let p1_pawn = self.p1.1 == Piece::WhitePawn;
        let p2_pawn = p2.1.is_pawn();
        let p3_pawn = p3.1.is_pawn();

        // Pawns always form a suffix of (p1,p2,p3) by value order, so the only reachable
        // pawn-shapes are: none; p3 only; p2+p3; or p1+p2+p3. Each branch below returns,
        // so falling past all the "!p1_pawn" branches means p1 (and therefore p2,p3) are
        // pawns.
        if !p1_pawn && !p2_pawn && !p3_pawn {
            let p1c = compact(sq1, &[wk, bk]);
            let p2c = compact(sq2, &[wk, bk, sq1]);
            let p3c = compact(sq3, &[wk, bk, sq1, sq2]);
            const P3_RANGE: usize = 60;

            if self.p1.1 == p2.1 && p2.1 == p3.1 {
                let idx = tri3(p1c, p2c, p3c, P3_RANGE);
                let size = tetra(P3_RANGE);
                return side * (num_kings * size) + king_idx * size + idx;
            } else if self.p1.1 == p2.1 {
                const N12: usize = 61;
                const TRI12: usize = N12 * (N12 + 1) / 2;
                let tri = tri2(p1c, p2c, N12);
                let block = TRI12 * P3_RANGE;
                return side * (num_kings * block) + king_idx * block + tri * P3_RANGE + p3c;
            } else if p2.1 == p3.1 {
                const TRI23: usize = P3_RANGE * (P3_RANGE + 1) / 2;
                let tri = tri2(p2c, p3c, P3_RANGE);
                const P1_RANGE: usize = 62;
                let block = P1_RANGE * TRI23;
                return side * (num_kings * block) + king_idx * block + p1c * TRI23 + tri;
            } else {
                const P1_RANGE: usize = 62;
                const P2_RANGE: usize = 61;
                let block = P1_RANGE * P2_RANGE * P3_RANGE;
                return side * (num_kings * block) + king_idx * block + p1c * P2_RANGE * P3_RANGE + p2c * P3_RANGE + p3c;
            }
        }

        if !p1_pawn && !p2_pawn && p3_pawn {
            let p1c = compact(sq1, &[wk, bk]);
            let p2c = compact(sq2, &[wk, bk, sq1]);
            let p3c = sq3 - 8;
            const P3_RANGE: usize = 48;

            if self.p1.1 == p2.1 {
                const N12: usize = 61;
                const TRI12: usize = N12 * (N12 + 1) / 2;
                let tri = tri2(p1c, p2c, N12);
                let block = TRI12 * P3_RANGE;
                return side * (num_kings * block) + king_idx * block + tri * P3_RANGE + p3c;
            } else {
                const P1_RANGE: usize = 62;
                const P2_RANGE: usize = 61;
                let block = P1_RANGE * P2_RANGE * P3_RANGE;
                return side * (num_kings * block) + king_idx * block + p1c * P2_RANGE * P3_RANGE + p2c * P3_RANGE + p3c;
            }
        }

        if !p1_pawn && p2_pawn && p3_pawn && p2.1 == p3.1 {
            let p1c = compact(sq1, &[wk, bk]);
            let p2c = sq2 - 8;
            let p3_off = sq3 - 8;
            let p3c = p3_off - (p3_off > p2c) as usize;
            const N23: usize = 47;
            const TRI23: usize = N23 * (N23 + 1) / 2;
            let tri = tri2(p2c, p3c, N23);
            const P1_RANGE: usize = 62;
            let block = P1_RANGE * TRI23;
            return side * (num_kings * block) + king_idx * block + p1c * TRI23 + tri;
        }

        if !p1_pawn && p2_pawn && p3_pawn {
            // "xPvP": p1 is a piece (unconstrained by enpassant), so nest the tail
            // inside p1's own block. Given enpassant is possible, p2's and p3's squares
            // are fully determined by (file, direction) alone -- no need for the general
            // compacted (p2c,p3c) encoding in that sub-case.
            let p1c = compact(sq1, &[wk, bk]);
            let p2c = sq2 - 8;
            let p3_off = sq3 - 8;
            let p3c_base = p3_off - (p3_off > p2c) as usize;
            const P2_RANGE: usize = 48;
            const P3_RANGE: usize = 47;
            const BASE23: usize = P2_RANGE * P3_RANGE;
            const EP_EXTRA: usize = 16;
            const P1_RANGE: usize = 62;
            const TAIL_BLOCK: usize = BASE23 + EP_EXTRA;
            let block = P1_RANGE * TAIL_BLOCK;
            if self.enpassant.is_none() {
                let inner = p2c * P3_RANGE + p3c_base;
                return side * (num_kings * block) + king_idx * block + p1c * TAIL_BLOCK + inner;
            } else {
                let file2 = p2.0.file() as usize;
                let file3 = p3.0.file() as usize;
                let dir = (file3 > file2) as usize;
                let tail = BASE23 + file2 * 2 + dir;
                return side * (num_kings * block) + king_idx * block + p1c * TAIL_BLOCK + tail;
            }
        }

        if self.p1.1 == p2.1 && p2.1 == p3.1 {
            let p1c = sq1 - 8;
            let p2_off = sq2 - 8;
            let p2c = p2_off - (p2_off > p1c) as usize;
            let p3_off = sq3 - 8;
            let p3c = p3_off - (p3_off > p1c) as usize - (p3_off > p2c) as usize;
            const N: usize = 46;
            let idx = tri3(p1c, p2c, p3c, N);
            let size = tetra(N);
            return side * (num_kings * size) + king_idx * size + idx;
        }

        // "PPvP": p1,p2 identical (triangular), p3 the opposite-colour pawn. Either p1 or
        // p2 could be the one adjacent to p3 that set enpassant, so the tail additionally
        // needs a "which" bit; the *other* (uninvolved) pawn is still unconstrained and
        // gets its own compacted square, excluding both p3 and the involved pawn.
        let p1c = sq1 - 8;
        let p2_off = sq2 - 8;
        let p2c = p2_off - (p2_off > p1c) as usize;
        const N12: usize = 47;
        const TRI12: usize = N12 * (N12 + 1) / 2;
        const P3_RANGE: usize = 46;
        const BASE: usize = TRI12 * P3_RANGE;
        const EP_TAIL: usize = 46 * 32;
        const BLOCK: usize = BASE + EP_TAIL;

        if self.enpassant.is_none() {
            let p3_off = sq3 - 8;
            let p3c = p3_off - (p3_off > p1c) as usize - (p3_off > p2c) as usize;
            let tri = tri2(p1c, p2c, N12);
            let inner = tri * P3_RANGE + p3c;
            side * (num_kings * BLOCK) + king_idx * BLOCK + inner
        } else {
            let ep = self.enpassant.unwrap();
            let ep_file = ep.file() as usize;
            let pawn_rank = if self.last_moved == Colour::White { 3 } else { 4 };
            let p1_involved = self.p1.0.file() as usize == ep_file && self.p1.0.rank() as usize == pawn_rank;
            let raw1 = sq1 - 8;
            let raw2 = sq2 - 8;
            let raw3 = sq3 - 8;
            let (which, involved_raw, free_raw, file) = if p1_involved {
                (0usize, raw1, raw2, self.p1.0.file() as usize)
            } else {
                (1usize, raw2, raw1, p2.0.file() as usize)
            };
            let free_c = free_raw - (free_raw > involved_raw) as usize - (free_raw > raw3) as usize;
            let dir = (p3.0.file() as usize > file) as usize;
            let tail = BASE + free_c * 32 + which * 16 + file * 2 + dir;
            side * (num_kings * BLOCK) + king_idx * BLOCK + tail
        }
    }

    // Swaps the colour of the pieces, and applies a vertical reflection if pawns are present
    #[inline]
    fn colour_swap(&mut self) {
        self.king.swap(0, 1);
        self.p1.1 = self.p1.1.colour_swap();
        if let Some(ref mut p2) = self.p2 {
            p2.1 = p2.1.colour_swap();
            if let Some(ref mut p3) = self.p3 {
                p3.1 = p3.1.colour_swap();
            }
        }
        if self.is_pawnful() {
            self.reflect(Reflection::Vertical);
        }
        self.last_moved = !self.last_moved;
    }

    // Corrects the order of p1, p2, and p3 to canonical order:
    // Orders p1, p2, p3 by p1 >= p2 >= p3 using piece value only
    #[inline]
    fn correct_piece_order(&mut self) {
        if self.p3.is_some() && self.p2.is_none() {
            self.p2 = self.p3;
            self.p3 = None;
        }
        match (self.p2, self.p3) {
            (None, None) => {},
            (Some(p2), None) => {
                if p2.1.abs_regular_value() > self.p1.1.abs_regular_value() {
                    self.p2 = Some(self.p1);
                    self.p1 = p2;
                }
            },
            (Some(mut p2), Some(mut p3)) => {
                let mut p = [self.p1, p2, p3];
                if p[1].1.abs_regular_value() > p[0].1.abs_regular_value() {
                    p.swap(0, 1);
                }
                if p[2].1.abs_regular_value() > p[1].1.abs_regular_value() {
                    p.swap(1, 2);
                    if p[1].1.abs_regular_value() > p[0].1.abs_regular_value() {
                        p.swap(0, 1);
                    }
                }
                self.p1 = p[0];
                self.p2 = Some(p[1]);
                self.p3 = Some(p[2]);
            }
            (None, Some(_)) => panic!("p3 is Some but p2 is None")
        }
    }

    // Calls colour_swap and swaps opposite coloured pieces to turn
    // the current pos canonical:
    // If p1 > p2 >= p3 --> colour_swap to make p1 white
    // If p1 > p2 = !p3 --> swap p2/p3 to make p2 white
    // If p1 = p2 = p3 --> colour_swap to make all white
    // If p1 = !p2 > p3 --> colour_swap to make p3 white and swaps p1/p2 to make p1 white
    // If same value different colours --> colour_swap to make majority white and swap all to make p1/p2 white
    // If p1 = !p2, no p3 --> colour_swap to make last_moved black and swap p1/p2 to make p1 white
    // (Assumes that p1 >= p2 >= p3 by piece value)
    #[inline]
    fn correct_colour(&mut self) {
        test_assert!({
            let mut tmp = self.clone(); tmp.correct_piece_order();
            *self == tmp
        });
        let p1 = self.p1;
        match (self.p2, self.p3) {
            (None, None) => {
                if p1.1.colour() == Colour::Black {
                    self.colour_swap();
                }
            },
            (Some(p2), None) => {
                if p1.1 == p2.1 || p1.1.abs_regular_value() > p2.1.abs_regular_value() {
                    if p1.1.colour() == Colour::Black {
                        self.colour_swap();
                    }
                } else {
                    // p1 == p2.colour_swap()
                    if self.last_moved == Colour::White {
                        self.colour_swap();
                    }
                    if self.p1.1.colour() == Colour::Black {
                        let tmp = self.p1;
                        self.p1 = self.p2.unwrap();
                        self.p2 = Some(tmp);
                    }
                }
            },
            (Some(p2), Some(p3)) => {
                if p1.1.abs_regular_value() > p2.1.abs_regular_value() {
                    if p1.1.colour() == Colour::Black {
                        self.colour_swap();
                    }
                    if p2.1 == p3.1.colour_swap() {
                        if self.p2.unwrap().1.colour() == Colour::Black {
                            let tmp = self.p2.unwrap();
                            self.p2 = self.p3;
                            self.p3 = Some(tmp);
                        }
                    }
                } else if p2.1.abs_regular_value() > p3.1.abs_regular_value() {
                    // p1 == p2 or p1 == p2.colour_swap()
                    if p1.1 == p2.1 {
                        if p1.1.colour() == Colour::Black {
                            self.colour_swap();
                        }
                    } else {
                        // p1 == p2.colour_swap()
                        if p3.1.colour() == Colour::Black {
                            self.colour_swap();
                        }
                        if self.p1.1.colour() == Colour::Black {
                            let tmp = self.p1;
                            self.p1 = self.p2.unwrap();
                            self.p2 = Some(tmp);
                        }
                    }
                } else {
                    // p1 == p2 == p3 up to colour
                    let white_count = 
                        (p1.1.colour() == Colour::White) as u8 +
                        (p2.1.colour() == Colour::White) as u8 +
                        (p3.1.colour() == Colour::White) as u8;
                    if white_count <= 1 {
                        self.colour_swap();
                    }
                    if white_count == 1 || white_count == 2 {
                        let mut p = [self.p1, self.p2.unwrap(), self.p3.unwrap()];
                        let black_idx = 
                            if p[0].1.colour() == Colour::Black { 0 }
                            else if p[1].1.colour() == Colour::Black { 1 }
                            else { 2 };
                        if black_idx < 2 {
                            p.swap(black_idx, 2);
                        }
                        self.p1 = p[0];
                        self.p2 = Some(p[1]);
                        self.p3 = Some(p[2]);
                    }
                }
            },
            (None, Some(_)) => panic!("p2 is None but p3 is Some")
        }
    }

    // Corrects the order of p1, p2, and p3 when they are the same piece
    // in increasing square order.
    // Assumes that piece order and colour are already canonical.
    #[inline]
    fn correct_subpiece_order(&mut self) {
        test_assert!({
            let mut tmp = self.clone();
            tmp.correct_piece_order(); tmp.correct_colour();
            tmp == *self
        });
        let p1 = self.p1;
        match (self.p2, self.p3) {
            (None, None) => {},
            (Some(p2), None) => {
                if p1.1 == p2.1 && p1.0 as u8 > p2.0 as u8 {
                    self.p1 = p2;
                    self.p2 = Some(p1);
                }
            },
            (Some(p2), Some(p3)) => {
                if p1.1 == p2.1 && p2.1 == p3.1 {
                    let mut p = [p1, p2, p3];
                    if (p[1].0 as u8) < p[0].0 as u8 {
                        p.swap(0, 1);
                    }
                    if (p[2].0 as u8) < p[1].0 as u8 {
                        p.swap(1, 2);
                        if (p[1].0 as u8) < p[0].0 as u8 {
                            p.swap(0, 1);
                        }
                    }
                    self.p1 = p[0];
                    self.p2 = Some(p[1]);
                    self.p3 = Some(p[2]);
                } else if p1.1 == p2.1 {
                    if (p1.0 as u8) > p2.0 as u8 {
                        self.p1 = p2;
                        self.p2 = Some(p1);
                    }
                } else if p2.1 == p3.1 {
                    if (p2.0 as u8) > p3.0 as u8 {
                        self.p2 = Some(p3);
                        self.p3 = Some(p2);
                    }
                }
            },
            (None, Some(_)) => panic!("p3 is Some but p2 is None")
        }
    }

    // Makes a position canonical by correcting major piece order, colour, reflections, and minor piece order.
    // A canonical positions satisfies the following:
    // If pawnful, the white king is in the left half
    // If pawnless, the white king is in the bottom-right triangle of the botom-left quarter
    //      Additionally, if white king is on diagonal, black king is at or below diagonal
    //      If both kings on diagonal, pos or diagonal(pos) is chosen by least key
    // Major piece order: p1 >= p2 >= p3 by value and p1 is white
    // Correct colour: look at correct_colour
    // Minor piece order: among equal pieces (type and colour), lower squares go first
    pub(crate) fn make_canonical(&mut self) {
        self.correct_piece_order();
        self.correct_colour();
        if self.is_pawnful() && self.king[Colour::White].file() >= 4 {
            self.reflect(Reflection::Horizontal);
            self.correct_subpiece_order();
            return;
        }
        // pawnless
        let on_diagonal = |s: Square| { let (rank, file) = s.rank_file(); rank == file };
        let above_diagonal = |s: Square| { let (rank, file) = s.rank_file(); rank > file };

        if self.king[Colour::White].file() >= 4 {
            self.reflect(Reflection::Horizontal);
        }
        if self.king[Colour::White].rank() >= 4 {
            self.reflect(Reflection::Vertical);
        }
        if above_diagonal(self.king[Colour::White]) {
            self.reflect(Reflection::Diagonal);
        }
        if !on_diagonal(self.king[Colour::White]) {
            self.correct_subpiece_order();
            return;
        }
        // pawnless, white king on diagonal
        if !on_diagonal(self.king[Colour::Black]) {
            if above_diagonal(self.king[Colour::Black]) {
                self.reflect(Reflection::Diagonal);
            }
            self.correct_subpiece_order();
            return;
        }
        // pawnless, both kings on diagonal
        let mut clone = self.clone();
        self.correct_subpiece_order();
        clone.correct_subpiece_order();
        clone.reflect(Reflection::Diagonal);
        self.correct_subpiece_order();
        clone.correct_subpiece_order();
        if clone.key() < self.key() {
            *self = clone;
        }
    }

    #[inline]
    fn key(&self) -> (u8, u8, u8, u8, u8, u8) {
        (
            !self.last_moved as u8, self.king[Colour::White] as u8, self.king[Colour::Black] as u8,
            self.p1.0 as u8,
            unsafe { std::mem::transmute(self.p2.map(|p| p.0)) },
            unsafe { std::mem::transmute(self.p3.map(|p| p.0)) },
        )
    }

    // A non-zero hash unique to every Pos.
    // Different to the Zobrist hash
    #[inline]
    pub(crate) fn unique_hash(&self) -> Hash {
        let encode_piece_sq = |p: Option<(Square, Piece)>| -> u64 {
            match p {
                None => 0xFFFF,
                Some(sq_piece) => unsafe { std::mem::transmute::<(Square, Piece), u16>(sq_piece) as u64 },
            }
        };
        let mut out = 0;
        out |= self.king[0] as u64;
        out |= (self.king[1] as u64) << 6;
        out |= (self.p1.1 as u64) << 12;
        out |= encode_piece_sq(self.p2) << 16;
        out |= encode_piece_sq(self.p3) << 32;
        out |= (self.p1.0 as u64) << 48;
        out |= self.enpassant.map_or(0xFF, |sq| sq as u64) << 54;
        out |= (self.last_moved as u64) << 62;
        Hash(out)
    }

    // The number of distinct canonical successors a position has
    pub(crate) fn count_distinct_canonical_successors(&self) -> usize {
        let mut board = self.into_board_partial();
        let movelist = board.generate_movelist(false);
        let count_duplicates = !self.is_pawnful() && {
            let (wr, wf) = self.king[0].rank_file();
            let (br, bf) = self.king[1].rank_file();
            (wr == wf && br + 1 >= bf) || (br == bf && wr + 1 >= wf)
        };
        if !count_duplicates {
            return movelist.num_total_moves();
        }

        let mut hashes = [Hash(0); MAX_MOVES];
        let mut count = 0;
        for i in 0..movelist.length {
            let unmake = board.makemove(movelist[i]);
            let hash = Pos::from_board(&board).unique_hash();
            if !hashes[..count].contains(&hash) {
                hashes[count] = hash;
                count += 1;
            }
            board.unmakemove(movelist[i], unmake);
        }
        count
    }

    // Occupancy bitboard of whole Pos
    #[inline]
    pub(crate) fn occupied(&self) -> BB {
        let mut out = BB::new(0);
        out |= self.king[0].bb() | self.king[1].bb();
        out |= self.p1.0.bb();
        if let Some(p2) = self.p2 {
            out |= p2.0.bb();
            if let Some(p3) = self.p3 {
                out |= p3.0.bb();
            }
        }
        out
    }

    // Returns true if the given colour is in check.
    // Does not take into account king-checks-king as it is illegal
    pub(crate) fn in_check(&self, colour: Colour) -> bool {
        let king = self.king[colour];
        let occupied = self.occupied();
        let checks_king = |(square, piece): (Square, Piece)| {
            piece.colour() != colour && match piece {
                Piece::WhitePawn => king & pawn_attacks(square.bb(), Colour::White) != 0,
                Piece::BlackPawn => king & pawn_attacks(square.bb(), Colour::Black) != 0,
                Piece::WhiteKnight | Piece::BlackKnight => king & knight_attacks(square.bb()) != 0,
                Piece::WhiteBishop | Piece::BlackBishop => {
                    let segment = SEGMENT_DIAGONAL[king][square];
                    segment != 0 && segment & occupied == king.bb()
                },
                Piece::WhiteRook | Piece::BlackRook => {
                    let segment = SEGMENT_CARDINAL[king][square];
                    segment != 0 && segment & occupied == king.bb()
                },
                Piece::WhiteQueen | Piece::BlackQueen => {
                    let segment = SEGMENT[king][square];
                    segment != 0 && segment & occupied == king.bb()
                },
                Piece::WhiteKing | Piece::BlackKing => panic!("illegal piece")
            }
        };
        checks_king(self.p1)
        || self.p2.is_some_and(checks_king)
        || self.p3.is_some_and(checks_king)
    }

    // Returns the square an enpassant capture would move to by the given side to move
    #[inline]
    fn enpassant_square(pawn_to_move: Square, other_pawn: Square, colour_to_move: Colour) -> Option<Square> {
        let neighbouring = pawn_to_move.file().abs_diff(other_pawn.file()) == 1;
        match colour_to_move {
            Colour::White => {
                if neighbouring && pawn_to_move.rank() == 3 && other_pawn.rank() == 3 {
                    Some(Square::from_u8(other_pawn as u8 - 8))
                } else {
                    None
                }
            },
            Colour::Black => {
                if neighbouring && pawn_to_move.rank() == 4 && other_pawn.rank() == 4 {
                    Some(Square::from_u8(other_pawn as u8 + 8))
                } else {
                    None
                }
            }
        }
    }

    // The targets squares an un-enpassant capture could land in
    #[inline]
    pub(crate) fn unenpassant_targets(square: Square, colour: Colour, occupied: BB) -> BB {
        let empty = square.bb() << 8 | square.bb() >> 8;
        let ep_rank = if colour == Colour::White { 5 } else { 2 };
        if square.rank() != ep_rank || empty & occupied != 0{
            return BB::new(0);
        }
        pawn_attacks(square.bb(), !colour) & !occupied
    }

    pub(crate) fn predecessors(self) -> impl Iterator<Item = Pos> {
        PosIter {
            revmovelist: self.generate_revmovelist(),
            pos: self,
            hashes: [Hash(0); RevMoveList::MAX_MOVES],
            index: 0
        }
    }
}

struct PosIter {
    pos: Pos,
    revmovelist: RevMoveList,
    hashes: [Hash; RevMoveList::MAX_MOVES],
    index: usize
}

impl Iterator for PosIter {
    type Item = Pos;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.revmovelist.length {
            return None;
        }
        let mut revmove = self.revmovelist.list[self.index];
        let mut pos = self.pos.clone();
        pos.make_revmove(revmove);
        let mut hash = pos.unique_hash();
        while self.hashes[..self.index].contains(&hash) || pos.in_check(pos.last_moved) {
            self.index += 1;
            if self.index >= self.revmovelist.length {
                return None;
            }
            revmove = self.revmovelist.list[self.index];
            pos = self.pos.clone();
            pos.make_revmove(revmove);
            hash = pos.unique_hash();
        }

        if revmove.enpassant.is_none() {
            for candidate in Pos::enpassant_candidates(&pos).into_iter().flatten() {
                if pos.enpassant_possible(candidate, pos.last_moved) {
                    let mut ep_revmove = revmove;
                    ep_revmove.enpassant = Some(candidate);
                    self.revmovelist.add(ep_revmove);
                }
            }
        }

        self.hashes[self.index] = hash;
        self.index += 1;
        Some(pos)
    }
}

#[cfg(test)]
mod index_bijection_tests {
    use super::*;

    #[test]
    fn tri2_is_a_bijection() {
        for n in [46usize, 47, 60, 61] {
            let total = n * (n + 1) / 2;
            let mut seen = vec![false; total];
            for a in 0..n {
                for b in a..n {
                    let idx = tri2(a, b, n);
                    assert!(idx < total, "n={n} a={a} b={b} idx={idx} out of range {total}");
                    assert!(!seen[idx], "n={n} a={a} b={b} idx={idx} collides");
                    seen[idx] = true;
                }
            }
            assert!(seen.iter().all(|&s| s), "n={n}: not all {total} slots covered");
        }
    }

    #[test]
    fn tri3_is_a_bijection() {
        for n in [46usize, 60] {
            let total = tetra(n);
            let mut seen = vec![false; total];
            for a in 0..n {
                for b in a..n {
                    for c in b..n {
                        let idx = tri3(a, b, c, n);
                        assert!(idx < total, "n={n} a={a} b={b} c={c} idx={idx} out of range {total}");
                        assert!(!seen[idx], "n={n} a={a} b={b} c={c} idx={idx} collides");
                        seen[idx] = true;
                    }
                }
            }
            assert!(seen.iter().all(|&s| s), "n={n}: not all {total} slots covered");
        }
    }
}