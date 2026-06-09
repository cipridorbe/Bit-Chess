use crate::repr::colour::Colour;

#[derive(Clone, Copy, PartialEq, Eq)]
/// 0 0 0 0 bq bk wq wk
pub struct CastlingRights(u8);

impl CastlingRights {
    const BQ: u8 = 1 << 3;
    const BK: u8 = 1 << 2;
    const WQ: u8 = 1 << 1;
    const WK: u8 = 1 << 0;

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
}