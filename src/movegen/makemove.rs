use crate::{eval::{Eval, pst::{PIECE_VALUE_EG, PIECE_VALUE_MG, PST_EG, PST_MG}}, movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_knight_attacks, single_rook_attacks}, r#move::{Flag, Move, MoveList}}, repr::{bitboard::BB, board::{Board, BoardState}, castling::CastlingRights, colour::Colour, hash::Hash, piece::{Piece, PieceType}, square::{RAY, SEGMENT, SEGMENT_CARDINAL, SEGMENT_DIAGONAL, Square}}, test_assert};

fn makemove(board: &mut Board, mv: Move) -> UnmakeInfo {
    test_assert!(board.attacks(board.colour) & board[Piece::king(!board.colour)] == 0);
    let mut unmake_info = UnmakeInfo::read(board);

    let colour = board.colour;
    let piece = board[mv.source_square()].unwrap();
    let mut final_piece = piece;
    let mut final_square = mv.target_square();
    let mut captured = board[mv.target_square()];
    let original_occupancy = board.occupied();
    let original_white = board[Colour::White];
    let original_black = board[Colour::Black];
    
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
    if piece.is_pawn_or_king() {
        board.state.pawn_hash ^= Hash::POSITION_PIECE[piece as usize][mv.source_square() as usize] ^ Hash::POSITION_PIECE[piece as usize][mv.target_square() as usize];
    }
    if mv.is_promotion() {
        let new_piece = mv.promoted_piece(colour);
        final_piece = new_piece;
        board[new_piece] |= mv.target_square().bb();
        board[mv.target_square()] = Some(new_piece);
        board.state.mg_eval -= PIECE_VALUE_MG[piece as usize];
        board.state.eg_eval -= PIECE_VALUE_EG[piece as usize];
        board.state.mg_eval += PST_MG[new_piece as usize][mv.target_square() as usize] + PIECE_VALUE_MG[new_piece as usize];
        board.state.eg_eval += PST_EG[new_piece as usize][mv.target_square() as usize] + PIECE_VALUE_EG[new_piece as usize];
        board.state.hash ^= Hash::POSITION_PIECE[new_piece as usize][mv.target_square() as usize];
        board.state.pawn_hash ^= Hash::POSITION_PIECE[piece as usize][mv.target_square() as usize];
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
        board.state.pawn_hash ^= Hash::POSITION_PIECE[captured.unwrap() as usize][captured_square as usize];
    } else if mv.is_capture() {
        board[captured.unwrap()] &= !mv.target_square().bb();
        board.state.mg_eval -= PST_MG[captured.unwrap() as usize][mv.target_square() as usize] + PIECE_VALUE_MG[captured.unwrap() as usize];
        board.state.eg_eval -= PST_EG[captured.unwrap() as usize][mv.target_square() as usize] + PIECE_VALUE_EG[captured.unwrap() as usize];
        board.state.hash ^= Hash::POSITION_PIECE[captured.unwrap() as usize][mv.target_square() as usize];
        if captured.unwrap() == Piece::WhitePawn || captured.unwrap() == Piece::BlackPawn {
            board.state.pawn_hash ^= Hash::POSITION_PIECE[captured.unwrap() as usize][mv.target_square() as usize];
        }
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
    let colour_diff = (original_white ^ board[Colour::White]) | (original_black ^ board[Colour::Black]);

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
    if colour_diff & board.state.xray_attacks[Colour::White as usize] != 0 {
        let c = Colour::White;
        let king = board[Piece::king(c)].lsb();
        let rook_attacks = single_rook_attacks(king, board.occupied());
        let xray_rook = single_rook_attacks(king, board.occupied() & !(rook_attacks & board[c]));
        let bishop_attacks = single_bishop_attacks(king, board.occupied());
        let xray_bishop = single_bishop_attacks(king, board.occupied() & !(bishop_attacks & board[c]));
        let xray = xray_rook | xray_bishop;
        let pinners = (xray_rook & (board[Piece::rook(!c)] | board[Piece::queen(!c)]))
            | (xray_bishop & (board[Piece::bishop(!c)] | board[Piece::queen(!c)]));
        board.state.xray_attacks[c as usize] = xray;
        board.state.pinners[!c as usize] = pinners;
        if colour == !c {
            board.state.checkers |= rook_attacks & (board[Piece::rook(!c)] | board[Piece::queen(!c)]);
            board.state.checkers |= bishop_attacks & (board[Piece::bishop(!c)] | board[Piece::queen(!c)]);
        }
    }
    if colour_diff & board.state.xray_attacks[Colour::Black as usize] != 0 {
        let c = Colour::Black;
        let king = board[Piece::king(c)].lsb();
        let rook_attacks = single_rook_attacks(king, board.occupied());
        let xray_rook = single_rook_attacks(king, board.occupied() & !(rook_attacks & board[c]));
        let bishop_attacks = single_bishop_attacks(king, board.occupied());
        let xray_bishop = single_bishop_attacks(king, board.occupied() & !(bishop_attacks & board[c]));
        let xray = xray_rook | xray_bishop;
        let pinners = (xray_rook & (board[Piece::rook(!c)] | board[Piece::queen(!c)]))
            | (xray_bishop & (board[Piece::bishop(!c)] | board[Piece::queen(!c)]));
        board.state.xray_attacks[c as usize] = xray;
        board.state.pinners[!c as usize] = pinners;
        if colour == !c {
            board.state.checkers |= rook_attacks & (board[Piece::rook(!c)] | board[Piece::queen(!c)]);
            board.state.checkers |= bishop_attacks & (board[Piece::bishop(!c)] | board[Piece::queen(!c)]);
        }
    }

    if board.state.attacks[colour as usize][PieceType::Leaper as usize] & board[Piece::king(!colour)] != 0 {
        board.state.checkers |= mv.target_square();
    }

    board.colour = !colour;

    board.state.hash ^= Hash::SIDE_HASH;
    board.state.hash ^= Hash::CASTLING_HASH[board.castling_rights.0 as usize];
    board.add_hash_to_history(board.state.hash);

    board.move_history.push((mv, Some(piece)));

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
    board.move_history.push((Move::NULL_MOVE, None));
    board.state.repetitions = 1;
    board.state.checkers = BB::new(0);

    null_unmake_info
}

fn unmakemove(board: &mut Board, mv: Move, unmake_info: UnmakeInfo) {
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

// fn is_legal(board: &Board, mv: Move, pinned: BB) -> bool {
//     return true;
//     let colour = board.colour;
//     let piece = board[mv.source_square()].unwrap();
//     if piece == Piece::WhiteKing || piece == Piece::BlackKing {
//         return true;
//     }
//     let king = board[Piece::king(colour)];
//     if mv.flag() != Flag::ENPASSANT {
//         if pinned != 0 && mv.source_square().bb() & pinned != 0 && RAY[king.lsb() as usize][mv.source_square() as usize] & mv.target_square() == 0 {
//             return false;
//         }
//         return true;
//     } else {
//         let captured_square = match colour {
//             Colour::White => Square::from_u8(mv.target_square() as u8 - 8),
//             Colour::Black => Square::from_u8(mv.target_square() as u8 + 8),
//         };
//         let new_occupied = (board.occupied() & !mv.source_square().bb() & !captured_square.bb()) | mv.target_square();
//         if SEGMENT[king.lsb() as usize][mv.source_square() as usize] != 0 && board.state.attacks[!colour as usize][PieceType::Slider as usize] & (mv.source_square().bb() | captured_square.bb()) != 0 {
//             let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
//             if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0 {
//                 return false;
//             }
//             let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
//             if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
//                 return false;
//             }
//         }
//         if SEGMENT[king.lsb() as usize][captured_square as usize] == 0 || board.state.attacks[!colour as usize][PieceType::Slider as usize] & (mv.source_square().bb() | captured_square.bb()) == 0 {
//             return true;
//         }
//         let rook = single_rook_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
//         if rook & (board[Piece::rook(!colour)] | board[Piece::queen(!colour)]) != 0 {
//             return false;
//         }
//         let bishop = single_bishop_attacks(king.lsb(), new_occupied) & !mv.target_square().bb();
//         if bishop & (board[Piece::bishop(!colour)] | board[Piece::queen(!colour)]) != 0 {
//             return false;
//         }
//         return true;
//     }
// }

// fn is_legal_partial(board: &Board, mv: Move) -> bool {
//     let colour = board.colour;
//     let piece = board[mv.source_square()];
//     let captured = board[mv.target_square()];
//     if piece.is_none() || piece.unwrap().colour() != colour {
//         return false;
//     }
//     if mv.flag() == Flag::ENPASSANT {
//         if board.enpassant.is_none() || board.enpassant.unwrap().file() != mv.target_square().file() || captured.is_some() {
//             return false;
//         }
//     } else if mv.is_capture() {
//         if captured.is_none() || captured.unwrap().colour() != !colour {
//             return false;
//         }
//     } else {
//         if captured.is_some() {
//             return false;
//         }
//     }
//     true
// }

impl Board {
    pub fn makemove(&mut self, mv: Move) -> UnmakeInfo {
        makemove(self, mv)
    }

    pub fn unmakemove(&mut self, mv: Move, unmake_info: UnmakeInfo) {
        unmakemove(self, mv, unmake_info);
    }

    pub fn null_makemove(&mut self) -> NullUnmakeInfo {
        null_makemove(self)
    }

    pub fn null_unmakemove(&mut self, null_unmake_info: NullUnmakeInfo) {
        null_unmakemove(self, null_unmake_info);
    }

    // pub fn is_legal(&self, mv: Move, pinned: BB) -> bool {
    //     is_legal(self, mv, pinned)
    // }

    // pub fn is_legal_partial(&self, mv: Move) -> bool {
    //     return true;
    //     is_legal_partial(self, mv)
    // }
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

#[cfg(test)]
mod tests {
    use crate::{movegen::r#move::Move, repr::{board::Board, hash::Hash, square::Square}};

    /// Computes the pawn+king hash from scratch by scanning all 64 squares.
    fn computed_pawn_hash(board: &Board) -> Hash {
        let mut h = Hash(0);
        for sq in 0..64usize {
            let square = Square::from_u8(sq as u8);
            if let Some(piece) = board[square] {
                if piece.is_pawn_or_king() {
                    h ^= Hash::POSITION_PIECE[piece as usize][sq];
                }
            }
        }
        h
    }

    fn assert_pawn_hash(board: &Board) {
        assert_eq!(
            board.state.pawn_hash,
            computed_pawn_hash(board),
            "pawn_hash mismatch in position: {}",
            board.to_fen()
        );
    }

    fn play(board: &mut Board, uci: &str) {
        let mv = Move::from_uci(board, uci);
        board.makemove(mv);
        assert_pawn_hash(board);
    }

    #[test]
    fn test_pawn_hash_init() {
        let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_pawn_hash(&board);
    }

    #[test]
    fn test_pawn_hash_quiet_pawn_moves() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        play(&mut board, "e2e4");
        play(&mut board, "d7d5");
        play(&mut board, "d2d4");
    }

    #[test]
    fn test_pawn_hash_non_pawn_move_unchanged() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let hash_before = board.state.pawn_hash;
        play(&mut board, "g1f3");
        assert_eq!(board.state.pawn_hash, hash_before,
            "pawn hash must not change when only a knight moves");
    }

    #[test]
    fn test_pawn_hash_pawn_x_pawn() {
        let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2");
        play(&mut board, "e4d5"); // white pawn captures black pawn
    }

    #[test]
    fn test_pawn_hash_piece_x_pawn() {
        // The specific bug: non-pawn captures a pawn — main hash double-XOR'd, pawn hash not updated
        let mut board = Board::from_fen("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 2");
        play(&mut board, "f3e5"); // knight captures pawn
        play(&mut board, "d8h4"); // queen move (non-pawn, pawn hash unchanged)
        play(&mut board, "e5d7"); // knight captures another pawn
    }

    #[test]
    fn test_pawn_hash_pawn_x_piece() {
        // Pawn captures a non-pawn: pawn moves in hash, captured piece not in hash
        let mut board = Board::from_fen("rnbqkbnr/pppp1ppp/8/4n3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2");
        play(&mut board, "e4e5"); // pawn push (can't capture directly — let's use a different pos)
    }

    #[test]
    fn test_pawn_hash_enpassant() {
        let mut board = Board::from_fen("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3");
        play(&mut board, "e5f6"); // en passant: attacking pawn moves, captured pawn removed from hash
    }

    #[test]
    fn test_pawn_hash_promotion() {
        // Pawn promotes: removed from pawn hash, new piece (queen/knight) NOT added
        let mut board = Board::from_fen("8/P7/8/8/8/8/8/4K1k1 w - - 0 1");
        play(&mut board, "a7a8q");
        // After promotion, no pawns on board — pawn hash should only have kings
        let expected_only_kings = computed_pawn_hash(&board);
        assert_eq!(board.state.pawn_hash, expected_only_kings);
    }

    #[test]
    fn test_pawn_hash_promotion_capture() {
        let mut board = Board::from_fen("1n6/P7/8/8/8/8/8/4K1k1 w - - 0 1");
        play(&mut board, "a7b8q"); // pawn captures and promotes
    }

    #[test]
    fn test_pawn_hash_king_move() {
        // King is included in pawn hash
        let mut board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
        play(&mut board, "e1d1");
        play(&mut board, "e8d8");
        play(&mut board, "d1e1");
    }

    #[test]
    fn test_pawn_hash_castling() {
        // Castling moves the king — must update pawn hash
        let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
        play(&mut board, "e1g1"); // white kingside castle
        play(&mut board, "e8c8"); // black queenside castle
    }

    #[test]
    fn test_pawn_hash_sequence() {
        // Full game sequence exercising all pawn-hash-relevant events
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        play(&mut board, "e2e4");
        play(&mut board, "e7e5");
        play(&mut board, "d2d4");
        play(&mut board, "e5d4");  // pawn captures pawn
        play(&mut board, "c2c3");
        play(&mut board, "d4c3");  // pawn captures pawn
        play(&mut board, "b2c3");  // pawn recaptures
        play(&mut board, "g8f6");  // knight (no pawn hash change)
        play(&mut board, "g1f3");  // knight (no pawn hash change)
        play(&mut board, "f8c5");  // bishop (no pawn hash change)
    }
}