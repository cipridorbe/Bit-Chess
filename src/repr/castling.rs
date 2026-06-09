use crate::repr::colour::Colour;

#[derive(Clone, Copy, PartialEq, Eq)]
/// 0 0 0 0 bq bk wq wk
pub struct CastlingRights(pub u8);

impl CastlingRights {
    const BQ: u8 = 1 << 3;
    const BK: u8 = 1 << 2;
    const WQ: u8 = 1 << 1;
    const WK: u8 = 1 << 0;

    /// Returns true if and only if there are any castling rights remaining
    pub fn any(self) -> bool {
        self.0 != 0
    }

    pub fn get(self, colour: Colour) -> (bool, bool) {
        match colour {
            Colour::White => self.get_white(),
            Colour::Black => self.get_black()
        }
    }

    pub fn get_black(self) -> (bool, bool) {
        (self.0 & CastlingRights::BQ != 0, self.0 & CastlingRights::BK != 0)
    }
    
    pub fn get_white(self) -> (bool, bool) {
        (self.0 & CastlingRights::WQ != 0, self.0 & CastlingRights::WK != 0)
    }

    pub fn unset_black_queen(&mut self) {
        self.0 &= !CastlingRights::BQ;
    }

    pub fn unset_black_king(&mut self) {
        self.0 &= !CastlingRights::BK;
    }

    pub fn unset_black(&mut self) {
        self.0 &= !(CastlingRights::BK | CastlingRights:: BQ);
    }

    pub fn unset_white_queen(&mut self) {
        self.0 &= !CastlingRights::WQ;
    }

    pub fn unset_white_king(&mut self) {
        self.0 &= !CastlingRights::WK;
    }

    pub fn unset_white(&mut self) {
        self.0 &= !(CastlingRights::WK | CastlingRights:: WQ);
    }

    pub fn to_fen(self) -> String {
        if !self.any() {
            return "-".to_string();
        }

        let mut out = String::new();
        let (wq, wk) = self.get_white();
        if wk { out.push('K') }
        if wq { out.push('Q') }
        let (bq, bk) = self.get_black();
        if bk { out.push('k') }
        if bq { out.push('q') }
        out
    }

    pub fn from_fen(fen: &str) -> Self {
        if fen == "-" {
            return CastlingRights(0);
        }

        let mut out = CastlingRights(0);
        for c in fen.chars() {
            match c {
                'K' => out.0 |= CastlingRights::WK,
                'Q' => out.0 |= CastlingRights::WQ,
                'k' => out.0 |= CastlingRights::BK,
                'q' => out.0 |= CastlingRights::BQ,
                _ => panic!("Unexpected fen castling rights {}", fen)
            } 
        }
        out
    }
}