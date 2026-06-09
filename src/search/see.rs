use crate::{movegen::{attacks::{pawn_attacks, single_bishop_attacks, single_king_attacks, single_knight_attacks, single_rook_attacks}, r#move::{CAPTURE_BASE_SCORE, Flag, Move, MoveScore, POSITIVE_SEE_OFFSET}}, repr::{bitboard::BB, board::Board, colour::Colour, piece::Piece, square::Square}};

/// Returns mvvlva score if see >= 0, see approximation otherwise 
pub fn see_mvvlva(board: &Board, mv: Move) -> MoveScore {
    let mut colour = board.colour;
    let attacker = board[mv.source_square()].unwrap();
    let victim = if mv.flag() == Flag::ENPASSANT { Piece::WhitePawn } else {
        board[mv.target_square()].unwrap()
    };
    let attacker_score = Piece::MVVLVA_VALUES[attacker as usize];
    let victim_score = Piece::MVVLVA_VALUES[victim as usize];
    let mvvlva = victim_score * 16 - attacker_score;
    if victim_score > attacker_score || board.attacks(!colour) & mv.target_square() == 0 {
        return CAPTURE_BASE_SCORE + POSITIVE_SEE_OFFSET + mvvlva;
    }

    let mut occupied = board.occupied() & !mv.source_square().bb();
    if mv.flag() == Flag::ENPASSANT {
        match colour {
            Colour::White => occupied &= !Square::from_u8(mv.target_square() as u8 - 8).bb(),
            Colour::Black => occupied &= !Square::from_u8(mv.target_square() as u8 + 8).bb(),
        }
    }

    let is_promoting = mv.target_square().rank() == 7 || mv.target_square().rank() == 0;
    let mut last_moved = attacker;
    let mut scores = [0; 32];
    scores[0] = Piece::MVVLVA_VALUES[attacker as usize];
    colour = !colour;
    let mut i = 0;
    while let Some((source, mut attacker)) = get_lva(board, mv.target_square(), occupied, colour) {
        i += 1;
        scores[i] = -scores[i - 1] + Piece::MVVLVA_VALUES[last_moved as usize];
        if scores[i] > Piece::MVVLVA_VALUES[attacker as usize] {
            break;
        }
        if attacker == Piece::WhitePawn && is_promoting {
            scores[i] += Piece::MVVLVA_VALUES[Piece::WhiteQueen as usize] - Piece::MVVLVA_VALUES[Piece::WhitePawn as usize];
            attacker = Piece::WhiteQueen;
        }
        last_moved = attacker;
        colour = !colour;
        occupied &= !source.bb();
    }

    while i > 0 {
        scores[i - 1] = -MoveScore::max(scores[i], -scores[i - 1]);
        i -= 1;
    }

    if scores[0] > 0 { return CAPTURE_BASE_SCORE + POSITIVE_SEE_OFFSET + mvvlva }
    else if scores[0] == 0 { return CAPTURE_BASE_SCORE + mvvlva }
    else { return -CAPTURE_BASE_SCORE + scores[0] }
}

// TODO: implement skip pawns and skip knights
pub fn get_lva(board: &Board, target: Square, occupied: BB, colour: Colour) -> Option<(Square, Piece)> {
    let pawns = pawn_attacks(target.bb(), !colour) & occupied & board[Piece::pawn(colour)];
    if pawns != 0 { return Some((pawns.lsb(), Piece::WhitePawn)); }
    let knights = single_knight_attacks(target) & occupied & board[Piece::knight(colour)];
    if knights != 0 { return Some((knights.lsb(), Piece::WhiteKnight)); }
    let bishop_attacks = single_bishop_attacks(target, occupied);
    let bishops = bishop_attacks & occupied & board[Piece::bishop(colour)];
    if bishops != 0 { return Some((bishops.lsb(), Piece::WhiteBishop)); }
    let rook_attacks = single_rook_attacks(target, occupied);
    let rooks = rook_attacks & occupied & board[Piece::rook(colour)];
    if rooks != 0 { return Some((rooks.lsb(), Piece::WhiteRook)); }
    let queens = (rook_attacks | bishop_attacks) & occupied & board[Piece::queen(colour)];
    if queens != 0 { return Some((queens.lsb(), Piece::WhiteQueen)); }
    let kings = single_king_attacks(target) & occupied & board[Piece::king(colour)];
    if kings != 0 { return Some((kings.lsb(), Piece::WhiteKing)); }
    None
}