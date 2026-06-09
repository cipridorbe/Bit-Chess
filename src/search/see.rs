use std::mem::MaybeUninit;

use crate::{bitboard::{Board, Piece, Side, Square}, movegen::{attacks::{king_attacks, knight_attacks, pawn_attacks, single_bishop_attacks, single_rook_attacks}, r#move::{Flag, Move}, tables::{BISHOP_EMPTY_ATTACKS, ROOK_EMPTY_ATTAKCS}}, util::{lsb_index, squares}};

const VALUE_ABS: [i16; 12] = [
    1, 3, 3, 5, 9, 99,
    1, 3, 3, 5, 9, 99,
];

pub fn see(board: &Board, initial_move: Move) -> i16 {
    let mut sides = board.sides.clone();
    let mut side = board.side;
    let mut idxs = [0, 0];
    let mut scores = [0; 32];
    let mut last_captured = Piece::WhitePawn;
    sides[board.side as usize] &= !(1 << initial_move.source_square() as u8);
    if initial_move.flag() == Flag::ENPASSANT {
        scores[0] = VALUE_ABS[Piece::WhitePawn as usize];
        if side == Side::White {
            sides[Side::Black as usize] &= !(1 << (initial_move.target_square() as u8 - 8));
        } else {
            sides[Side::White as usize] &= !(1 << (initial_move.target_square() as u8 + 8));
        }
    } else {
        last_captured = board.piece_at(initial_move.source_square()).unwrap();
        if let Some(captured) = board.piece_at(initial_move.target_square()) {
            scores[0] = VALUE_ABS[captured as usize];
            sides[side.other() as usize] &= !(1 << initial_move.target_square() as u8);
        }
        if initial_move.is_promotion() {
            scores[0] += VALUE_ABS[Piece::WhiteQueen as usize] - VALUE_ABS[Piece::WhitePawn as usize];
            last_captured = Piece::WhiteQueen;
        }
    }
    let mut seelists = SEEList::from_board(board, initial_move.target_square(), sides[0] | sides[1]);
    side = side.other();
    let mut i = 1;
    while idxs[side as usize] < seelists[side as usize].length {
        let idx = idxs[side as usize];
        let square = seelists[side as usize].list[idx];
        idxs[side as usize] += 1;
        if square == initial_move.source_square() {         
            continue;
        }
        let piece = board.piece_at(square).unwrap();
        scores[i] = VALUE_ABS[last_captured as usize] - scores[i - 1];
        last_captured = piece;
        let target_rank = initial_move.target_square().rank();
        if (piece == Piece::WhitePawn || piece == Piece::BlackPawn) && (target_rank == 0 || target_rank == 7) {
            scores[i] += VALUE_ABS[Piece::WhiteQueen as usize] - VALUE_ABS[Piece::WhitePawn as usize];
            last_captured = Piece::WhiteQueen;
        }
        
        if seelists[0].has_hidden() || seelists[1].has_hidden() {
            sides[side as usize] &= !(1 << square as u8);
            match piece {
                Piece::WhitePawn | Piece::BlackPawn | Piece::WhiteBishop | Piece::BlackBishop => {
                    add_hidden_bishop(board, initial_move.target_square(), Side::White, &mut seelists, idxs[Side::White as usize], sides[0] | sides[1]);
                    add_hidden_bishop(board, initial_move.target_square(), Side::Black, &mut seelists, idxs[Side::Black as usize], sides[0] | sides[1]);
                },
                Piece::WhiteRook | Piece::BlackRook => {
                    add_hidden_rook(board, initial_move.target_square(), Side::White, &mut seelists, idxs[Side::White as usize], sides[0] | sides[1]);
                    add_hidden_rook(board, initial_move.target_square(), Side::Black, &mut seelists, idxs[Side::Black as usize], sides[0] | sides[1]);
                },
                Piece::WhiteQueen | Piece::BlackQueen | Piece::WhiteKing | Piece::BlackKing => {
                    add_hidden_bishop(board, initial_move.target_square(), Side::White, &mut seelists, idxs[Side::White as usize], sides[0] | sides[1]);
                    add_hidden_bishop(board, initial_move.target_square(), Side::Black, &mut seelists, idxs[Side::Black as usize], sides[0] | sides[1]);
                    add_hidden_rook(board, initial_move.target_square(), Side::White, &mut seelists, idxs[Side::White as usize], sides[0] | sides[1]);
                    add_hidden_rook(board, initial_move.target_square(), Side::Black, &mut seelists, idxs[Side::Black as usize], sides[0] | sides[1]);
                },
                Piece::WhiteKnight | Piece::BlackKnight => {}
            }
        }

        i += 1;
        side = side.other();
    }

    i -= 1;
    while i > 0 {
        scores[i - 1] = -i16::max(-scores[i - 1], scores[i]);
        i -= 1;
    }
    return scores[0];
}

pub fn see_sign(board: &Board, initial_move: Move) -> i16 {
    let mut occupied = board.occupied;
    let mut last_moved = board.piece_at(initial_move.source_square()).unwrap();
    let mut captured_piece = board.piece_at(initial_move.target_square());
    if initial_move.flag() == Flag::ENPASSANT {
        captured_piece = Some(Piece::WhitePawn);
        if board.side == Side::White {
            occupied &= !(1 << (initial_move.target_square() as u8 - 8));
        } else {
            occupied &= !(1 << (initial_move.target_square() as u8 +8));
        }
    }
    if initial_move.is_capture() && VALUE_ABS[captured_piece.unwrap() as usize] > VALUE_ABS[last_moved as usize] {
        return VALUE_ABS[captured_piece.unwrap() as usize] - VALUE_ABS[last_moved as usize];
    }
    
    let prom_rank = initial_move.target_square().rank() == 0 || initial_move.target_square().rank() == 7;
    occupied &= !(1 << initial_move.source_square() as u8);
    let mut scores: [i16; 32] = unsafe { std::mem::transmute([MaybeUninit::<i16>::uninit(); 32]) };
    if let Some(captured) = captured_piece {
        scores[0] = VALUE_ABS[captured as usize];
    }
    let mut side = board.side.other();
    let mut i = 1;
    while let Some((mut attacker, bb)) = get_lva(board, initial_move.target_square(), side, occupied) {
        scores[i] = VALUE_ABS[last_moved as usize] - scores[i - 1];
        if scores[i] > VALUE_ABS[attacker as usize] {
            i += 1;
            break;
        }
        if (attacker == Piece::WhitePawn || attacker == Piece::BlackPawn) && prom_rank {
            attacker = Piece::WhiteQueen;
            scores[i] += VALUE_ABS[Piece::WhiteQueen as usize] - VALUE_ABS[Piece::WhitePawn as usize];
        }
        last_moved = attacker;
        occupied &= !(bb);
        side = side.other();
        i += 1;
    }
    i -= 1;
    while i > 0 {
        scores[i - 1] = -i16::max(-scores[i - 1], scores[i]);
        i -= 1;
    }
    return scores[0];
}

pub fn get_lva(board: &Board, square: Square, side: Side, occupied: u64) -> Option<(Piece, u64)> {
    let mut bb = pawn_attacks(1 << square as u8, side.other()) & board.pieces[Piece::pawn(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhitePawn, bb & bb.wrapping_neg()));
    }
    bb = knight_attacks(1 << square as u8) & board.pieces[Piece::knight(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhiteKnight, bb & bb.wrapping_neg()));
    }
    let bishop = single_bishop_attacks(square, occupied);
    bb = bishop & board.pieces[Piece::bishop(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhiteBishop, bb & bb.wrapping_neg()));
    }
    let rook = single_rook_attacks(square, occupied); 
    bb = rook & board.pieces[Piece::rook(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhiteRook, bb & bb.wrapping_neg()));
    }
    bb = (bishop | rook) & board.pieces[Piece::queen(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhiteQueen, bb & bb.wrapping_neg()));
    }
    bb = king_attacks(1 << square as u8) & board.pieces[Piece::king(side) as usize] & occupied;
    if bb != 0 {
        return Some((Piece::WhiteKing, bb & bb.wrapping_neg()));
    }
    None
}

pub fn add_hidden_bishop(board: &Board, square: Square, side: Side, seelists: &mut [SEEList; 2], idx: usize, occupancy: u64) {
    if seelists[side as usize].hidden_bishops != 0 {
        let attacks = single_bishop_attacks(square, occupancy);
        let new_bishop = attacks & seelists[side as usize].hidden_bishops;
        if new_bishop != 0 {
            seelists[side as usize].hidden_bishops &= !(new_bishop);
            let new_square = unsafe { std::mem::transmute(lsb_index(new_bishop)) };
            seelists[side as usize].insert_in_order(board, new_square, idx);
        }
    }
}

pub fn add_hidden_rook(board: &Board, square: Square, side: Side, seelists: &mut [SEEList; 2], idx: usize, occupancy: u64) {
    if seelists[side as usize].hidden_rooks != 0 {
        let attacks = single_rook_attacks(square, occupancy);
        let new_rook = attacks & seelists[side as usize].hidden_rooks;
        if new_rook != 0 {
            seelists[side as usize].hidden_rooks &= !(new_rook);
            let new_square = unsafe { std::mem::transmute(lsb_index(new_rook)) };
            seelists[side as usize].insert_in_order(board, new_square, idx);
        }
    }
}

pub struct SEEList {
    list: [Square; 16],
    length: usize,
    hidden_rooks: u64,
    hidden_bishops: u64,
}

impl SEEList {
    pub fn has_hidden(&self) -> bool {
        self.hidden_bishops != 0 || self.hidden_rooks != 0
    }

    pub fn from_board(board: &Board, square: Square, occupied: u64) -> [SEEList; 2] {
        let mut lists = [SEEList::new(), SEEList::new()];
        // White pawn attacks
        for sq in squares(pawn_attacks(1 << square as u8, Side::Black) & board.pieces[Piece::WhitePawn as usize]) {
            lists[Side::White as usize].append(sq);
        }
        // Black pawn attacks
        for sq in squares(pawn_attacks(1 << square as u8, Side::White) & board.pieces[Piece::BlackPawn as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        let knight_attacks = knight_attacks(1 << square as u8);
        // White knight
        for sq in squares(knight_attacks & board.pieces[Piece::WhiteKnight as usize]) {
            lists[Side::White as usize].append(sq);
        }
        // Black knight
        for sq in squares(knight_attacks & board.pieces[Piece::BlackKnight as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        // White Bishop
        let bishop_attacks = single_bishop_attacks(square, occupied);
        for sq in squares(bishop_attacks & board.pieces[Piece::WhiteBishop as usize]) {
            lists[Side::White as usize].append(sq);
        }
        // Black Bishop
        for sq in squares(bishop_attacks & board.pieces[Piece::BlackBishop as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        // White rook
        let rook_attacks = single_rook_attacks(square, occupied);
        for sq in squares(rook_attacks & board.pieces[Piece::WhiteRook as usize]) {
            lists[Side::White as usize].append(sq);
        }
        // Black rook
        for sq in squares(rook_attacks & board.pieces[Piece::BlackRook as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        // White queen
        for sq in squares((rook_attacks | bishop_attacks) & board.pieces[Piece::WhiteQueen as usize]) {
            lists[Side::White as usize].append(sq);
        }
        // Black Queen
        for sq in squares((rook_attacks | bishop_attacks) & board.pieces[Piece::BlackQueen as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        // Kings
        let king_atk = king_attacks(1 << square as u8);
        for sq in squares(king_atk & board.pieces[Piece::WhiteKing as usize]) {
            lists[Side::White as usize].append(sq);
        }
        for sq in squares(king_atk & board.pieces[Piece::BlackKing as usize]) {
            lists[Side::Black as usize].append(sq);
        }
        let missing_rook_attacks = ROOK_EMPTY_ATTAKCS[square as usize] & !rook_attacks;
        lists[Side::White as usize].hidden_rooks = missing_rook_attacks & (board.pieces[Piece::WhiteRook as usize] | board.pieces[Piece::WhiteQueen as usize]);
        lists[Side::Black as usize].hidden_rooks = missing_rook_attacks & (board.pieces[Piece::BlackRook as usize] | board.pieces[Piece::BlackQueen as usize]);
        let missing_bishop_attacks = BISHOP_EMPTY_ATTACKS[square as usize] & !bishop_attacks;
        lists[Side::White as usize].hidden_bishops = missing_bishop_attacks & (board.pieces[Piece::WhiteBishop as usize] | board.pieces[Piece::WhiteQueen as usize]);
        lists[Side::Black as usize].hidden_bishops = missing_bishop_attacks & (board.pieces[Piece::BlackBishop as usize] | board.pieces[Piece::BlackQueen as usize]);
        lists
    }

    pub fn new() -> Self {
        SEEList {
            list: [Square::a1; 16],
            length: 0,
            hidden_rooks: 0,
            hidden_bishops: 0,
        }
    }

    pub fn append(&mut self, square: Square) {
        self.list[self.length] = square;
        self.length += 1;
    }

    pub fn insert_in_order(&mut self, board: &Board, square: Square, already_explored: usize) {
        let piece_value = VALUE_ABS[board.piece_at(square).unwrap() as usize];
        let mut i = self.length as i8 - 1;
        while i >= already_explored as i8 && piece_value < VALUE_ABS[board.piece_at(self.list[i as usize]).unwrap() as usize] {
            self.list[i as usize + 1] = self.list[i as usize];
            i -= 1;
        }
        self.list[(i + 1) as usize] = square;
        self.length += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bitboard::Board,
        movegen::r#move::Move,
    };

    struct SeeTest {
        fen: &'static str,
        mv: &'static str,
        expected: i16,
    }

    const PAWN: i16 = 1;
    // const KNIGHT: i16 = 3;
    const BISHOP: i16 = 3;
    const ROOK: i16 = 5;
    const QUEEN: i16 = 9;

    const SEE_TESTS: &[SeeTest] = &[
        SeeTest {
            fen: "4R3/2r3p1/5bk1/1p1r3p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1",
            mv: "h5g4",
            expected: 0,
        },
        SeeTest {
            fen: "4R3/2r3p1/5bk1/1p1r1p1p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1",
            mv: "h5g4",
            expected: 0,
        },
        SeeTest {
            fen: "4r1k1/5pp1/nbp4p/1p2p2q/1P2P1b1/1BP2N1P/1B2QPPK/3R4 b - - 0 1",
            mv: "g4f3",
            expected: 0,
        },
        SeeTest {
            fen: "2r1r1k1/pp1bppbp/3p1np1/q3P3/2P2P2/1P2B3/P1N1B1PP/2RQ1RK1 b - - 0 1",
            mv: "d6e5",
            expected: PAWN,
        },
        SeeTest {
            fen: "7r/5qpk/p1Qp1b1p/3r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1",
            mv: "e1e8",
            expected: 0,
        },
        SeeTest {
            fen: "6rr/6pk/p1Qp1b1p/2n5/1B3p2/5p2/P1P2P2/4RK1R w - - 0 1",
            mv: "e1e8",
            expected: -ROOK,
        },
        SeeTest {
            fen: "7r/5qpk/2Qp1b1p/1N1r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1",
            mv: "e1e8",
            expected: -ROOK,
        },
        SeeTest {
            fen: "8/4kp2/2npp3/1Nn5/1p2PQP1/7q/1PP1B3/4KR1r b - - 0 1",
            mv: "h1f1",
            expected: 0,
        },
        SeeTest {
            fen: "8/4kp2/2npp3/1Nn5/1p2P1P1/7q/1PP1B3/4KR1r b - - 0 1",
            mv: "h1f1",
            expected: 0,
        },
        SeeTest {
            fen: "2r2r1k/6bp/p7/2q2p1Q/3PpP2/1B6/P5PP/2RR3K b - - 0 1",
            mv: "c5c1",
            expected: 2 * ROOK - QUEEN,
        },
        SeeTest {
            fen: "r2qk1nr/pp2ppbp/2b3p1/2p1p3/8/2N2N2/PPPP1PPP/R1BQR1K1 w kq - 0 1",
            mv: "f3e5",
            expected: PAWN,
        },
        SeeTest {
            fen: "6r1/4kq2/b2p1p2/p1pPb3/p1P2B1Q/2P4P/2B1R1P1/6K1 w - - 0 1",
            mv: "f4e5",
            expected: 0,
        },
        SeeTest {
            fen: "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R4B/PQ3P1P/3R2K1 w - h6 0 1",
            mv: "g5h6",
            expected: 0,
        },
        SeeTest {
            fen: "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R1B2B/PQ3P1P/3R2K1 w - h6 0 1",
            mv: "g5h6",
            expected: PAWN,
        },
        SeeTest {
            fen: "2r4r/1P4pk/p2p1b1p/7n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1",
            mv: "c3c8",
            expected: ROOK,
        },
        SeeTest {
            fen: "2r5/1P4pk/p2p1b1p/5b1n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1",
            mv: "c3c8",
            expected: ROOK,
        },
        SeeTest {
            fen: "2r4k/2r4p/p7/2b2p1b/4pP2/1BR5/P1R3PP/2Q4K w - - 0 1",
            mv: "c3c5",
            expected: BISHOP,
        },
        SeeTest {
            fen: "8/pp6/2pkp3/4bp2/2R3b1/2P5/PP4B1/1K6 w - - 0 1",
            mv: "g2c6",
            expected: PAWN - BISHOP,
        },
        SeeTest {
            fen: "4q3/1p1pr1k1/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1",
            mv: "e6e4",
            expected: PAWN - ROOK,
        },
        SeeTest {
            fen: "4q3/1p1pr1kb/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1",
            mv: "h7e4",
            expected: PAWN,
        },
    ];

    #[test]
    fn test_see_suite() {
        for test in SEE_TESTS {
            println!("{}", test.fen);
            let board = Board::from_fen(test.fen);
            let mv = Move::from_uci(&board, test.mv);
            let result = see(&board, mv);
            assert_eq!(
                result,
                test.expected,
                "SEE failed:\nFEN: {}\nMove: {}\nExpected: {}\nGot: {}",
                test.fen, test.mv, test.expected, result
            );
        }
    }

    #[test]
    fn test_see_sign_suite() {
        for test in SEE_TESTS {
            println!("{}", test.fen);
            let board = Board::from_fen(test.fen);
            let mv = Move::from_uci(&board, test.mv);
            let result = see_sign(&board, mv);
            assert_eq!(
                result.signum(),
                test.expected.signum(),
                "see_sign failed:\nFEN: {}\nMove: {}\nExpected sign: {}\nGot: {}",
                test.fen, test.mv, test.expected.signum(), result
            );
        }
    }
}