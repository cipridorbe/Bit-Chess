use crate::{eval::{Eval, pst::{PIECE_VALUE_EG, PIECE_VALUE_MG, PST_EG, PST_MG}}, movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_knight_attacks, single_rook_attacks}, r#move::{Flag, Move, MoveList}}, repr::{bitboard::BB, board::{Board, BoardState}, castling::CastlingRights, colour::Colour, hash::Hash, piece::{Piece, PieceType}, square::{SEGMENT, SEGMENT_CARDINAL, SEGMENT_DIAGONAL, Square}}, test_assert};

fn makemove(board: &mut Board, mv: Move) -> UnmakeInfo {
    test_assert!(board.attacks(board.colour) & board[Piece::king(!board.colour)] == 0);
    let mut unmake_info = UnmakeInfo::read(board);

    let colour = board.colour;
    let piece = board[mv.source_square()].unwrap();
    let mut final_piece = piece;
    let mut final_square = mv.target_square();
    let mut captured = board[mv.target_square()];
    let original_occupancy = board.occupied();
    
    unmake_info.piece = piece;
    unmake_info.captured = captured;

    if let Some(enpassant_square) = board.enpassant {
        board.state.hash ^= Hash::ENPASSANT_HASH[enpassant_square.file() as usize];
    }
    board.enpassant = None;
    board.state.hash ^= Hash::CASTLING_HASH[board.castling_rights.0 as usize];

    board[piece] &= !mv.source_square().bb();
    board[mv.source_square()] = None;
    board.state.mg_eval -= PST_MG[piece as usize][mv.source_square() as usize];
    board.state.eg_eval -= PST_EG[piece as usize][mv.source_square() as usize];
    board.state.hash ^= Hash::POSITION_PIECE[piece as usize][mv.source_square() as usize];
    if mv.is_promotion() {
        let new_piece = mv.promoted_piece(colour);
        final_piece = new_piece;
        board[new_piece] |= mv.target_square().bb();
        board[mv.target_square()] = Some(new_piece);
        board.state.mg_eval += PST_MG[new_piece as usize][mv.target_square() as usize];
        board.state.eg_eval += PST_EG[new_piece as usize][mv.target_square() as usize];
        board.state.hash ^= Hash::POSITION_PIECE[new_piece as usize][mv.target_square() as usize];
        board.state.phase_unbounded += new_piece.phase_value();
    } else {
        board[piece] |= mv.target_square().bb();
        board[mv.target_square()] = Some(piece);
        board.state.mg_eval += PST_MG[piece as usize][mv.target_square() as usize];
        board.state.eg_eval += PST_EG[piece as usize][mv.target_square() as usize];
        board.state.hash ^= Hash::POSITION_PIECE[piece as usize][mv.target_square() as usize];
    }

    if mv.flag() == Flag::ENPASSANT {
        captured = Some(Piece::pawn(!colour));
        let captured_square = match colour {
            Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
            Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
        };
        board[captured.unwrap()] &= !captured_square.bb();
        board[captured_square] = None;
        board.state.mg_eval -= PST_MG[captured.unwrap() as usize][captured_square as usize] + PIECE_VALUE_MG[captured.unwrap() as usize];
        board.state.eg_eval -= PST_EG[captured.unwrap() as usize][captured_square as usize] + PIECE_VALUE_EG[captured.unwrap() as usize];
        board.state.hash ^= Hash::POSITION_PIECE[captured.unwrap() as usize][captured_square as usize];
    } else if mv.is_capture() {
        board[captured.unwrap()] &= !mv.target_square().bb();
        board.state.mg_eval -= PST_MG[captured.unwrap() as usize][mv.target_square() as usize] + PIECE_VALUE_MG[captured.unwrap() as usize];
        board.state.eg_eval -= PST_EG[captured.unwrap() as usize][mv.target_square() as usize] + PIECE_VALUE_EG[captured.unwrap() as usize];
        board.state.hash ^= Hash::POSITION_PIECE[captured.unwrap() as usize][mv.target_square() as usize];
        match mv.target_square() {
            Square::a1 => board.castling_rights.unset_white_queen(),
            Square::h1 => board.castling_rights.unset_white_king(),
            Square::a8 => board.castling_rights.unset_black_queen(),
            Square::h8 => board.castling_rights.unset_black_king(),
            _ => {}
        }
    }

    if piece == Piece::WhiteKing {
        board.castling_rights.unset_white();
    } else if piece == Piece::BlackKing {
        board.castling_rights.unset_black();
    } else if piece == Piece::WhiteRook {
        match mv.source_square() {
            Square::a1 => board.castling_rights.unset_white_queen(),
            Square::h1 => board.castling_rights.unset_white_king(),
            _ => {}
        }
    } else if piece == Piece::BlackRook {
        match mv.source_square() {
            Square::a8 => board.castling_rights.unset_black_queen(),
            Square::h8 => board.castling_rights.unset_black_king(),
            _ => {}
        }
    }

    match mv.flag() {
        Flag::PAWNPUSH => {
            board.enpassant = Some(match colour {
                Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
                Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
            });
            board.state.hash ^= Hash::ENPASSANT_HASH[board.enpassant.unwrap().file() as usize];
        },
        Flag::KINGCASTLE => {
            let rook = Piece::rook(colour);
            final_piece = rook;
            let (source, target) = match colour {
                Colour::White => {
                    board.castling_rights.unset_white_king();
                    (Square::h1, Square::f1)
                },
                Colour::Black => {
                    board.castling_rights.unset_black_king();
                    (Square::h8, Square::f8)
                }
            };
            final_square = target;
            board[rook] &= !source.bb();
            board[rook] |= target.bb();
            board[source] = None;
            board[target] = Some(rook);
            board.state.mg_eval += PST_MG[rook as usize][target as usize] - PST_MG[rook as usize][source as usize];
            board.state.eg_eval += PST_EG[rook as usize][target as usize] - PST_EG[rook as usize][source as usize];
            board.state.hash ^= Hash::POSITION_PIECE[rook as usize][target as usize] ^ Hash::POSITION_PIECE[rook as usize][source as usize];
        },
        Flag::QUEENCASTLE => {
            let rook = Piece::rook(colour);
            final_piece = rook;
            let (source, target) = match colour {
                Colour::White => {
                    board.castling_rights.unset_white_queen();
                    (Square::a1, Square::d1)
                },
                Colour::Black => {
                    board.castling_rights.unset_black_queen();
                    (Square::a8, Square::d8)
                }
            };
            final_square = target;
            board[rook] &= !source.bb();
            board[rook] |= target.bb();
            board[source] = None;
            board[target] = Some(rook);
            board.state.mg_eval += PST_MG[rook as usize][target as usize] - PST_MG[rook as usize][source as usize];
            board.state.eg_eval += PST_EG[rook as usize][target as usize] - PST_EG[rook as usize][source as usize];
            board.state.hash ^= Hash::POSITION_PIECE[rook as usize][target as usize] ^ Hash::POSITION_PIECE[rook as usize][source as usize];
        }
        _ => {}
    }

    board[Colour::White] = board[Piece::WhitePawn] | board[Piece::WhiteKnight] | board[Piece::WhiteBishop] | board[Piece::WhiteRook] | board[Piece::WhiteQueen] | board[Piece::WhiteKing];
    board[Colour::Black] = board[Piece::BlackPawn] | board[Piece::BlackKnight] | board[Piece::BlackBishop] | board[Piece::BlackRook] | board[Piece::BlackQueen] | board[Piece::BlackKing];

    if mv.is_capture() || piece == Piece::WhitePawn || piece == Piece::BlackPawn {
        board.halfmove_clock = 0;
    } else {
        board.halfmove_clock += 1;
    }
    if colour == Colour::Black {
        board.fullmoves += 1;
    }

    let diff = original_occupancy ^ board.occupied();


    match piece.piece_type() {
        PieceType::Slider => board.state.attacks[colour as usize][PieceType::Slider as usize] = board.calculate_attacks(colour, PieceType::Slider),
        PieceType::Leaper => {
            board.state.attacks[colour as usize][PieceType::Leaper as usize] = board.calculate_attacks(colour, PieceType::Leaper);
            if diff & board.state.attacks[colour as usize][PieceType::Slider as usize] != 0 || final_piece.piece_type() == PieceType::Slider {
                board.state.attacks[colour as usize][PieceType::Slider as usize] = board.calculate_attacks(colour, PieceType::Slider);
            }
        }
    }
    let mut calculated_other_slider = false;
    if diff & board.state.attacks[!colour as usize][PieceType::Slider as usize] != 0 {
        board.state.attacks[!colour as usize][PieceType::Slider as usize] = board.calculate_attacks(!colour, PieceType::Slider);
        calculated_other_slider = true;
    }
    if let Some(captured_piece) = captured {
        board.state.phase_unbounded -= captured_piece.phase_value();
        match captured_piece.piece_type() {
            PieceType::Slider => board.state.attacks[!colour as usize][PieceType::Slider as usize] = board.calculate_attacks(!colour, PieceType::Slider),
            PieceType::Leaper => {
                board.state.attacks[!colour as usize][PieceType::Leaper as usize] = board.calculate_attacks(!colour, PieceType::Leaper);
                if !calculated_other_slider && diff & board.state.attacks[!colour as usize][PieceType::Slider as usize] != 0 {
                    board.state.attacks[!colour as usize][PieceType::Slider as usize] = board.calculate_attacks(!colour, PieceType::Slider);
                }
            }
        }
    }

    board.state.checkers = BB::new(0);
    if board.attacks(colour) & board[Piece::king(!colour)] != 0 {
        let king = board[Piece::king(!colour)];
        let moved_checks = match final_piece {
            Piece::WhiteKing | Piece::BlackKing => false,
            Piece::WhitePawn | Piece::BlackPawn => pawn_attacks(mv.target_square().bb(), colour) & king != 0,
            Piece::WhiteKnight | Piece::BlackKnight => single_knight_attacks(mv.target_square()) & king != 0,
            Piece::WhiteBishop | Piece::BlackBishop => SEGMENT_DIAGONAL[mv.target_square() as usize][king.lsb() as usize] & board.occupied() == mv.target_square().bb(),
            Piece::WhiteRook | Piece::BlackRook => SEGMENT_CARDINAL[final_square as usize][king.lsb() as usize] & board.occupied() == final_square.bb(),
            Piece::WhiteQueen | Piece::BlackQueen => SEGMENT[mv.target_square() as usize][king.lsb() as usize] & board.occupied() == mv.target_square().bb(),
        };
        if moved_checks {
            board.state.checkers |= final_square;
        }
        let enpassant_check = mv.flag() == Flag::ENPASSANT && {
            let captured_square = match colour {
                Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
                Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
            };
            SEGMENT[captured_square as usize][king.lsb() as usize] != 0
        };
        if SEGMENT[mv.source_square() as usize][king.lsb() as usize] != 0 || enpassant_check {
            let bishop_attacks = single_bishop_attacks(king.lsb(), board.occupied());
            if bishop_attacks & (board[Piece::bishop(colour)] | board[Piece::queen(colour)]) != 0 {
                board.state.checkers |= bishop_attacks & (board[Piece::bishop(colour)] | board[Piece::queen(colour)]);
            }
            let rook_attacks = single_rook_attacks(king.lsb(), board.occupied());
            if rook_attacks & (board[Piece::rook(colour)] | board[Piece::queen(colour)]) != 0 {
                board.state.checkers |= rook_attacks & (board[Piece::rook(colour)] | board[Piece::queen(colour)]);
            }
        }
        test_assert!(board.state.checkers != 0);
    }

    board.colour = !colour;

    board.state.hash ^= Hash::SIDE_HASH;
    board.state.hash ^= Hash::CASTLING_HASH[board.castling_rights.0 as usize];
    board.add_hash_to_history(board.state.hash);

    board.move_history.push(mv);

    unmake_info
}

fn null_makemove(board: &mut Board) -> NullUnmakeInfo {
    let null_unmake_info = NullUnmakeInfo::read(board);
    let colour = board.colour;

    if colour == Colour::Black {
        board.fullmoves += 1;
    }
    board.halfmove_clock = 0;

    board.state.hash ^= Hash::SIDE_HASH;

    if let Some(enpassant) = board.enpassant {
        board.state.hash ^= Hash::ENPASSANT_HASH[enpassant.file() as usize];
    }
    board.enpassant = None;

    board.colour = !colour;
    board.hash_history.push(board.state.hash);
    board.move_history.push(Move::NULL_MOVE);
    board.state.repetitions = 1;
    board.state.checkers = BB::new(0);

    null_unmake_info
}

fn unmakemove(board: &mut Board, mv: Move, mv_score: Eval, unmake_info: UnmakeInfo, movelist: Option<&mut MoveList>) {
    let colour = board.colour;

    let piece = unmake_info.piece;
    let captured = unmake_info.captured;
    unmake_info.write(board);

    board[piece] &= !mv.target_square().bb();
    board[piece] |= mv.source_square().bb();
    board[mv.target_square()] = captured;
    board[mv.source_square()] = Some(piece);

    if mv.flag() == Flag::ENPASSANT {
        let square = match !colour {
            Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
            Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
        };
        board[Piece::pawn(colour)] |= square;
        board[square] = Some(Piece::pawn(colour));
    } else if mv.is_queen_promotion() && mv_score == 0 {
        if let Some(movelist) = movelist {
            if mv.is_capture() {
                movelist.add_to_end(Move::new(Flag::BISHOPPROMCAP, mv.target_square(), mv.source_square()));
                movelist.add_to_end(Move::new(Flag::ROOKPROMCAP, mv.target_square(), mv.source_square()));
            } else {
                movelist.add_to_end(Move::new(Flag::BISHOPPROM, mv.target_square(), mv.source_square()));
                movelist.add_to_end(Move::new(Flag::ROOKPROM, mv.target_square(), mv.source_square()));
            }
        } else {
            println!("Unmaking queen promotion stalemate didn't add underpromotions");
        }
    }

    if let Some(captured_piece) = captured {
        board[captured_piece] |= mv.target_square();
    }

    if mv.is_promotion() {
        board[mv.promoted_piece(!colour)] &= !mv.target_square().bb();
    }

    if mv.flag() == Flag::KINGCASTLE {
        let rook = Piece::rook(!colour);
        let (start, end) = match !colour {
            Colour::White => (Square::h1, Square::f1),
            Colour::Black => (Square::h8, Square::f8),
        };
        board[rook] &= !end.bb();
        board[rook] |= start;
        board[end] = None;
        board[start] = Some(rook);
    } else if mv.flag() == Flag::QUEENCASTLE {
        let rook = Piece::rook(!colour);
        let (start, end) = match !colour {
            Colour::White => (Square::a1, Square::d1),
            Colour::Black => (Square::a8, Square::d8),
        };
        board[rook] &= !end.bb();
        board[rook] |= start;
        board[end] = None;
        board[start] = Some(rook);
    }

    board[Colour::White] = board[Piece::WhitePawn] | board[Piece::WhiteKnight] | board[Piece::WhiteBishop] | board[Piece::WhiteRook] | board[Piece::WhiteQueen] | board[Piece::WhiteKing];
    board[Colour::Black] = board[Piece::BlackPawn] | board[Piece::BlackKnight] | board[Piece::BlackBishop] | board[Piece::BlackRook] | board[Piece::BlackQueen] | board[Piece::BlackKing];

    board.hash_history.pop();
    board.move_history.pop();

    if colour == Colour::White {
        board.fullmoves -= 1;
    }
    
    board.colour = !colour;
}

fn null_unmakemove(board: &mut Board, null_unmake_info: NullUnmakeInfo) {
    test_assert!(board.move_history.last().copied() == Some(Move::NULL_MOVE));
    null_unmake_info.write(board);
    let colour = board.colour;
    if colour == Colour::White {
        board.fullmoves -= 1;
    }
    board.colour = !colour;
    board.hash_history.pop();
    board.move_history.pop();
}

fn is_legal(board: &Board, mv: Move) -> bool {
    let colour = board.colour;
    let piece = board[mv.source_square()].unwrap();
    if piece == Piece::WhiteKing || piece == Piece::BlackKing {
        return true;
    }
    let king = board[Piece::king(colour)];
    if mv.flag() != Flag::ENPASSANT {
        if SEGMENT[king.lsb() as usize][mv.source_square() as usize] == 0 || board.state.attacks[!colour as usize][PieceType::Slider as usize] & mv.source_square().bb() == 0 {
            return true;
        }
        let new_occupied = board.occupied() & !mv.source_square().bb() | mv.target_square();
        let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
        if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0{
            return false;
        }
        let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
        if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
            return false;
        }
        return true;
    } else {
        let captured_square = match colour {
            Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
            Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
        };
        let new_occupied = (board.occupied() & !mv.source_square().bb() & !captured_square.bb()) | mv.target_square();
        if SEGMENT[king.lsb() as usize][mv.source_square() as usize] != 0 && board.state.attacks[!colour as usize][PieceType::Slider as usize] & (mv.source_square().bb() | captured_square.bb()) != 0 {
            let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
            if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0{
                return false;
            }
            let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
            if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
                return false;
            }
        }
        if SEGMENT[king.lsb() as usize][captured_square as usize] == 0 || board.state.attacks[!colour as usize][PieceType::Slider as usize] & (mv.source_square().bb() | captured_square.bb()) == 0 {
            return true;
        }
        let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
        if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0{
            return false;
        }
        let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
        if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
            return false;
        }
        return true;
    }
}

impl Board {
    pub fn makemove(&mut self, mv: Move) -> UnmakeInfo {
        makemove(self, mv)
    }

    pub fn unmakemove(&mut self, mv: Move, mv_score: Eval, unmake_info: UnmakeInfo, movelist: Option<&mut MoveList>) {
        unmakemove(self, mv, mv_score, unmake_info, movelist);
    }

    pub fn null_makemove(&mut self) -> NullUnmakeInfo {
        null_makemove(self)
    }

    pub fn null_unmakemove(&mut self, null_unmake_info: NullUnmakeInfo) {
        null_unmakemove(self, null_unmake_info);
    }

    pub fn is_legal(&self, mv: Move) -> bool {
        is_legal(self, mv)
    }
}

/// Information stored when making a move to later unmake it
pub struct UnmakeInfo {
    state: BoardState,
    castling_rights: CastlingRights,
    enpassant: Option<Square>,
    halfmove_clock: u8,
    piece: Piece,
    captured: Option<Piece>
}

impl UnmakeInfo {
    fn read(board: &Board) -> Self {
        UnmakeInfo {
            state: board.state.clone(),
            castling_rights: board.castling_rights,
            enpassant: board.enpassant,
            halfmove_clock: board.halfmove_clock,
            piece: Piece::WhitePawn,
            captured: None
        }
    }

    fn write(self, board: &mut Board) {
        board.state = self.state;
        board.castling_rights = self.castling_rights;
        board.enpassant = self.enpassant;
        board.halfmove_clock = self.halfmove_clock
    }
}

pub struct NullUnmakeInfo {
    enpassant: Option<Square>,
    halfmove_clock: u8,
    hash: Hash,
    checkers: BB,
    repetitions: u8
}

impl NullUnmakeInfo {
    pub fn read(board: &Board) -> Self {
        NullUnmakeInfo {
            enpassant: board.enpassant,
            halfmove_clock: board.halfmove_clock,
            hash: board.state.hash,
            checkers: board.state.checkers,
            repetitions: board.state.repetitions
        }
    }

    pub fn write(self, board: &mut Board) {
        board.enpassant = self.enpassant;
        board.halfmove_clock = self.halfmove_clock;
        board.state.hash = self.hash;
        board.state.checkers = self.checkers;
        board.state.repetitions = self.repetitions;
    }
}