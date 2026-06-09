mod util;
mod bitboard;
mod movegen;
mod frontend;
mod eval;
mod search;

#[tokio::main]
async fn main() {
    frontend::run().await;
}
