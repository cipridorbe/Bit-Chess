use bitchess::egtb::pos::{Pos, save_tablebase};

fn main() {
    let status = Pos::generator();
    save_tablebase(&status, "tablebase").unwrap();
}