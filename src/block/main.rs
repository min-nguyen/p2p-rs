pub mod block;

use block::{
  Chain, Block
};

// RUST_LOG=info cargo run --bin block
#[tokio::main]
async fn main() {
  pretty_env_logger::init();

  let mut chain: Chain = Chain::new();

  let new_block = Block::new(1, "test".to_owned(), chain.blocks.last().expect("").hash);
  println!("{}", new_block);

  chain.try_push_block(new_block);
  let gen: &Block = chain.blocks.last().expect("");
  let new_block = Block::new(1, "test".to_owned(), gen.hash);
  chain.try_push_block(new_block);
}
