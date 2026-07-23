use crate::repr::square::Square;

pub mod reflections;
pub mod pos;
pub mod revmove;
pub mod compression;


// [white king][black king]
pub const KINGS_IDX_PAWNLESS: [[u16; 64]; 32] = {
    let mut out = [[u16::MAX; 64]; 32];
    let mut idx = 0;
    let mut _wk = 0;
    while _wk < 64 {
        let wk = Square::from_u8(_wk);
        let (wr, wf) = wk.rank_file();
        if wf >= 4 || wr >= 4 || wr > wf {
            _wk += 1;
            continue;
        }

        let mut _bk = 0;
        while _bk < 64 {
            let bk = Square::from_u8(_bk);
            let (br, bf) = bk.rank_file();
            if wr.abs_diff(br) <= 1 && wf.abs_diff(bf) <= 1 {
                _bk += 1;
                continue;
            }
            if wr == wf && br > bf {
                _bk += 1;
                continue;
            }

            out[wk as usize][bk as usize] = idx;
            idx += 1;

            _bk += 1;
        }
        _wk += 1;
    }
    out
};

// [white king][black king]
pub const KINGS_IDX_PAWNFUL: [[u16; 64]; 64] = {
    let mut out = [[u16::MAX; 64]; 64];
    let mut idx = 0;
    let mut _wk = 0;
    while _wk < 64 {
        let wk = Square::from_u8(_wk);
        let (wr, wf) = wk.rank_file();
        if wf >= 4 {
            _wk += 1;
            continue;
        }

        let mut _bk = 0;
        while _bk < 64 {
            let bk = Square::from_u8(_bk);
            let (br, bf) = bk.rank_file();
            if wr.abs_diff(br) <= 1 && wf.abs_diff(bf) <= 1 {
                _bk += 1;
                continue;
            }

            out[wk as usize][bk as usize] = idx;
            idx += 1;

            _bk += 1;
        }
        _wk += 1;
    }
    out
};