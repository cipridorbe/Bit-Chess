mod util;
mod bitboard;
mod movegen;
mod frontend;

#[tokio::main]
async fn main() {
    frontend::run().await;
}
