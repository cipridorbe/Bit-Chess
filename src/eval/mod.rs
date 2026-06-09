use crate::search::MAX_PLY;

pub mod pst;

pub type Eval = i16;

pub const INF: Eval = 31000;
pub const MATE: Eval = INF - 1;
pub const MATE_CUTOFF: Eval = MATE - MAX_PLY as Eval * 2;