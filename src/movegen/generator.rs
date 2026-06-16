use crate::{movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_king_attacks, single_knight_attacks, single_queen_attacks, single_rook_attacks}, r#move::{Flag::{self, CAPTURE}, Move, MoveList}}, repr::{bitboard::BB, board::Board, colour::Colour, piece::{Piece, PieceType}, square::{RAY, SEGMENT, Square}}};

const NO_PINS: [BB; 64] = [BB::new(!0); 64];

/// Creates list of pseudo-legal moves.
fn generate_movelist(board: &Board, captures_only: bool, mut legal: bool) -> MoveList {
    let mut movelist = MoveList::new();
    let colour = board.colour;
    let checkers = board.state.checkers;
    let enemy_attacks = board.attacks(!colour);
    let king = board[Piece::king(colour)].lsb();
    let (pinned, pinners) = board.pinned_and_pinners();
    movelist.pinned = pinned;
    if pinned == 0 { legal = false; }
    let mut pinned_moves = NO_PINS;
    if legal {
        for pin in pinned.squares() {
            pinned_moves[pin as usize] = RAY[king as usize][pin as usize];
        } 
    }

    // If in check by multiple pieces, only escape is by moving the king
    if checkers.count_ones() >= 2 {
        generate_piece_movelist::<false>(&mut movelist,
            board[Piece::king(colour)],
            single_king_attacks,
            board[colour] | enemy_attacks,
            board[!colour],
            captures_only, &NO_PINS, false
        );
        return movelist;
    }

    let mut check_segment = !BB::new(0);
    if checkers.count_ones() == 1 {
        let checker = checkers.lsb();
        if board[checker].unwrap().piece_type() == PieceType::Leaper {
            generate_attackers_movelist(&mut movelist, board, checker, &pinned_moves, legal);
            generate_piece_movelist::<false>(&mut movelist,
                board[Piece::king(colour)],
                single_king_attacks,
                board[colour] | enemy_attacks,
                board[!colour],
                captures_only, &pinned_moves, legal
            );
            if let Some(enpassant) = board.enpassant {
                if board[checker].unwrap() == Piece::pawn(!colour) {
                    for pawn in BB::squares(pawn_attacks(enpassant.bb(), !colour) & board[Piece::pawn(colour)]) {
                        let mv = Move::new(Flag::ENPASSANT, enpassant, pawn);
                        if legal && !enpassant_legal(board, mv, &pinned_moves) { continue; }
                        movelist.add(mv);
                    }
                }
            }
            return movelist;
        }
        check_segment = SEGMENT[checker as usize][board[Piece::king(colour)].lsb() as usize];
    }
    generate_castling_movelist(&mut movelist, board, captures_only);
    generate_pawn_movelist(&mut movelist, board, !check_segment | board[colour], captures_only, &pinned_moves, legal);
    generate_piece_movelist::<true>(&mut movelist,
        board[Piece::knight(colour)],
        single_knight_attacks,
        !check_segment | board[colour],
        board[!colour],
        captures_only, &pinned_moves, legal
    );
    generate_piece_movelist::<false>(&mut movelist,
        board[Piece::bishop(colour)],
        |square| { single_bishop_attacks(square, board.occupied()) },
        !check_segment | board[colour],
        board[!colour],
        captures_only, &pinned_moves, legal
    );
    generate_piece_movelist::<false>(&mut movelist,
        board[Piece::queen(colour)],
        |square| { single_queen_attacks(square, board.occupied()) },
        !check_segment | board[colour],
        board[!colour],
        captures_only, &pinned_moves, legal
    );
    generate_piece_movelist::<false>(&mut movelist,
        board[Piece::rook(colour)],
        |square| { single_rook_attacks(square, board.occupied()) },
        !check_segment | board[colour],
        board[!colour],
        captures_only, &pinned_moves, legal
    );
    generate_piece_movelist::<false>(&mut movelist,
        board[Piece::king(colour)],
        single_king_attacks,
        board[colour] | enemy_attacks,
        board[!colour],
        captures_only, &pinned_moves, legal
    );
    movelist
}

impl Board {
    pub fn generate_movelist(&self, captures_only: bool) -> MoveList {
        generate_movelist(self, captures_only, true)
    }
}

#[inline]
fn generate_piece_movelist<const is_knight: bool>(movelist: &mut MoveList, piece: BB, moves: impl Fn(Square) -> BB, avoid: BB, enemy_occupancy: BB, captures_only: bool, pinned_moves: &[BB; 64], legal: bool) {
    for source_square in piece.squares() {
        let targets = moves(source_square) & !avoid;
        let pin_moves = pinned_moves[source_square as usize];
        if legal && is_knight && pin_moves != !0 { continue; }
        for target_square in (targets & enemy_occupancy).squares() {
            if legal && pin_moves & target_square == 0 { continue; }
            movelist.add(Move::new(Flag::CAPTURE, target_square, source_square));
        }
        if !captures_only {
            for target_square in (targets & !enemy_occupancy).squares() {
                if legal && pin_moves & target_square == 0 { continue; }
                movelist.add(Move::new(Flag::QUIET, target_square, source_square));
            }
        }
    }
}

#[inline]
fn generate_attackers_movelist(movelist: &mut MoveList, board: &Board, square: Square, pinned_moves: &[BB; 64], legal: bool) {
    let colour = board.colour;
    for pawn in BB::squares(pawn_attacks(square.bb(), !colour) & board[Piece::pawn(colour)]) {
        if legal && pinned_moves[pawn as usize] & square == 0 { continue; }
        movelist.add(Move::new(Flag::CAPTURE, square, pawn));
    }
    for knight in BB::squares(single_knight_attacks(square) & board[Piece::knight(colour)]) {
        if legal && pinned_moves[knight as usize] & square == 0 { continue; }
        movelist.add(Move::new(Flag::CAPTURE, square, knight));
    }
    let rook_attacks = single_rook_attacks(square, board.occupied());
    let bishop_attacks = single_bishop_attacks(square, board.occupied());
    for rook in BB::squares(rook_attacks & board[Piece::rook(colour)]) {
        if legal && pinned_moves[rook as usize] & square == 0 { continue; }
        movelist.add(Move::new(CAPTURE, square, rook));
    }
    for bishop in BB::squares(bishop_attacks & board[Piece::bishop(colour)]) {
        if legal && pinned_moves[bishop as usize] & square == 0 { continue; }
        movelist.add(Move::new(CAPTURE, square, bishop));
    }
    for queen in BB::squares((rook_attacks | bishop_attacks) & board[Piece::queen(colour)]) {
        if legal && pinned_moves[queen as usize] & square == 0 { continue; }
        movelist.add(Move::new(CAPTURE, square, queen));
    }
}

#[inline]
fn generate_pawn_movelist(movelist: &mut MoveList, board: &Board, avoid: BB, captures_only: bool, pinned_moves: &[BB; 64], legal: bool) {
    let colour = board.colour;
    let pawns = board[Piece::pawn(colour)];
    match colour {
        Colour::White => {
            let attacks_left = (pawns & !Board::A_FILE) << 7;
            add_pawn_moves(movelist, attacks_left & board[!colour] & !avoid, 7, Flag::CAPTURE, pinned_moves, legal);
            let attacks_right = (pawns & !Board::H_FILE) << 9;
            add_pawn_moves(movelist, attacks_right & board[!colour] & !avoid, 9, Flag::CAPTURE, pinned_moves, legal);
            if !captures_only {
                let push = pawns << 8;
                add_pawn_moves(movelist, push & !board.occupied() & !avoid, 8, Flag::QUIET, pinned_moves, legal);
                let double_push = (push & Board::RANK_3 & !board.occupied()) << 8;
                add_pawn_moves(movelist, double_push & !board.occupied() & !avoid, 16, Flag::PAWNPUSH, pinned_moves, legal);
            }
        },
        Colour::Black => {
            let attacks_left = (pawns & !Board::A_FILE) >> 9;
            add_pawn_moves(movelist, attacks_left & board[!colour] & !avoid, -9, Flag::CAPTURE, pinned_moves, legal);
            let attacks_right = (pawns & !Board::H_FILE) >> 7;
            add_pawn_moves(movelist, attacks_right & board[!colour] & !avoid, -7, Flag::CAPTURE, pinned_moves, legal);
            if !captures_only {
                let push = pawns >> 8;
                add_pawn_moves(movelist, push & !board.occupied() & !avoid, -8, Flag::QUIET, pinned_moves, legal);
                let double_push = (push & Board::RANK_6 & !board.occupied()) >> 8;
                add_pawn_moves(movelist, double_push & !board.occupied() & !avoid, -16, Flag::PAWNPUSH, pinned_moves, legal);
            }
        },
    }

    if let Some(enp_square) = board.enpassant {
        if enp_square.bb() & !avoid != 0 {
            for pawn in BB::squares(pawn_attacks(enp_square.bb(), !colour) & pawns) {
                movelist.add(Move::new(Flag::ENPASSANT, enp_square, pawn));
            }
        }
    }
}

fn add_pawn_moves(movelist: &mut MoveList, targets: BB, offset: i8, flag: Flag, pinned_moves: &[BB; 64], legal: bool) {
    for target_square in targets.squares() {
        let source = Square::from_u8((target_square as i8 - offset) as u8);
        if legal && pinned_moves[source as usize] & target_square == 0 { continue; }
        let mut mv = Move::new(flag, target_square, source);
        if target_square.rank() == 7 || target_square.rank() == 0 {
            movelist.add(mv.into_queen_prom());
            movelist.add(mv.into_knight_prom());
        } else {
            movelist.add(mv);
        }
    }
}

#[inline]
fn generate_castling_movelist(movelist: &mut MoveList, board: &Board, captures_only: bool) {
    if captures_only {
        return;
    }
    let colour = board.colour;
    let (queenside, kingside) = board.castling_rights.get(colour);
    if queenside || kingside {
        let (queenlegal, kinglegal) = can_castle(board.attacks(!colour), board.occupied(), colour);
        if queenlegal && queenside {
            let (target, source) = match colour {
                Colour::White => (Square::c1, Square::e1),
                Colour::Black => (Square::c8, Square::e8)
            };
            movelist.add(Move::new(Flag::QUEENCASTLE, target, source));
        }
        if kinglegal && kingside {
            let (target, source) = match colour {
                Colour::White => (Square::g1, Square::e1),
                Colour::Black => (Square::g8, Square::e8)
            };
            movelist.add(Move::new(Flag::KINGCASTLE, target, source));
        }
    }
}

#[inline]
fn can_castle(attacks: BB, occupancy: BB, colour: Colour) -> (bool, bool) {
    const KING_SIDE_WHITE:          BB = BB::new((1 << (Square::e1 as u8)) | (1 << (Square::f1 as u8)) | (1 << (Square::g1 as u8)));
    const QUEEN_SIDE_WHITE_OCC:     BB = BB::new((1 << (Square::b1 as u8)) | (1 << (Square::c1 as u8)) | (1 << (Square::d1 as u8)));
    const QUEEN_SIDE_WHITE_ATTACKS: BB = BB::new((1 << (Square::c1 as u8)) | (1 << (Square::d1 as u8)) | (1 << (Square::e1 as u8)));
    const KING_SIDE_BLACK:          BB = BB::new((1 << (Square::e8 as u8)) | (1 << (Square::f8 as u8)) | (1 << (Square::g8 as u8)));
    const QUEEN_SIDE_BLACK_OCC:     BB = BB::new((1 << (Square::b8 as u8)) | (1 << (Square::c8 as u8)) | (1 << (Square::d8 as u8)));
    const QUEEN_SIDE_BLACK_ATTACKS: BB = BB::new((1 << (Square::c8 as u8)) | (1 << (Square::d8 as u8)) | (1 << (Square::e8 as u8)));

    if colour == Colour::White {
        (
            occupancy & QUEEN_SIDE_WHITE_OCC == 0 && attacks & QUEEN_SIDE_WHITE_ATTACKS == 0,
            occupancy & KING_SIDE_WHITE == Square::e1.bb() && attacks & KING_SIDE_WHITE == 0
        )
    } else {
        (
            occupancy & QUEEN_SIDE_BLACK_OCC == 0 && attacks & QUEEN_SIDE_BLACK_ATTACKS == 0,
            occupancy & KING_SIDE_BLACK == Square::e8.bb() && attacks & KING_SIDE_BLACK == 0
        )
    }
}

#[inline]
fn enpassant_legal(board: &Board, mv: Move, pinned_moves: &[BB; 64]) -> bool {
    let colour = board.colour;
    let king = board[Piece::king(colour)];
    let captured_square = match colour {
        Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
        Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
    };
    let new_occupied = (board.occupied() & !mv.source_square().bb() & !captured_square.bb()) | mv.target_square();
    if pinned_moves[mv.source_square() as usize] & mv.target_square() == 0 {
        return false;
    }
    if SEGMENT[king.lsb() as usize][captured_square as usize] == 0 || board.state.attacks[!colour as usize][PieceType::Slider as usize] & (mv.source_square().bb() | captured_square.bb()) == 0 {
        return true;
    }
    let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
    if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0 {
        return false;
    }
    let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
    if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
        return false;
    }
    return true;
}