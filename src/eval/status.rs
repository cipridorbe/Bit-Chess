use crate::{eval::Eval, repr::colour::Colour, test_assert};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Open,
    WhiteOnly,
    BlackOnly,
    Closed
}

impl FileStatus {
    const FILE_STATUS_KING_BONUS_WHITE: [Eval; 4] = [-40, 0, -20, 10];
    const FILE_STATUS_KING_BONUS_BLACK: [Eval; 4] = [-40, -20, 0, 10];
    const FILE_STATUS_ROOK_BONUS_WHITE: [Eval; 4] = [30, -20, 20, -10];
    const FILE_STATUS_ROOK_BONUS_BLACK: [Eval; 4] = [30, 20, -20, -10];

    pub fn new(white_file: bool, black_file: bool) -> Self {
        unsafe { std::mem::transmute(((black_file as u8) << 1) | white_file as u8) }
    }

    pub fn from_files(white_files: u8, black_files: u8, file: u8) -> Self {
        test_assert!(file < 8);
        let mask = 1 << file;
        FileStatus::new(white_files & mask != 0, black_files & mask != 0)
    }
 
    pub fn king_bonus(self, colour: Colour) -> Eval {
        match colour {
            Colour::White => FileStatus::FILE_STATUS_KING_BONUS_WHITE[self as usize],
            Colour::Black => FileStatus::FILE_STATUS_KING_BONUS_BLACK[self as usize],
        }
    }

    pub fn rook_bonus(self, colour: Colour) -> Eval {
        match colour {
            Colour::White => FileStatus::FILE_STATUS_ROOK_BONUS_WHITE[self as usize],
            Colour::Black => FileStatus::FILE_STATUS_ROOK_BONUS_BLACK[self as usize],
        }
    }
}