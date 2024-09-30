pub mod block;

use block::{
  Chain, Block, BlockHeader
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_first_block() {
      let mut chain: Chain = Chain::new();
      let new_block = Block::new(1, "test".to_owned(), chain.blocks.last().expect("").hash);

      assert_eq!(Ok(()), chain.try_push_block(new_block));
    }

    #[test]
    fn test_invalid_first_block() {
      let mut chain: Chain = Chain::new();
      let valid_block = Block::new(1, "test".to_owned(), chain.blocks.last().expect("").hash);

      let invalid_idx = Block {idx : 0, .. valid_block.clone()};
      assert_ne!(Ok(()), chain.try_push_block(invalid_idx));

      let invalid_prev_hash = Block {header: BlockHeader { prev_hash : [0;32], .. valid_block.header.clone()}, .. valid_block.clone() };
      assert_ne!(Ok(()), chain.try_push_block(invalid_prev_hash));

      let invalid_hash = Block {hash : [0;32], .. valid_block.clone()};
      assert_ne!(Ok(()), chain.try_push_block(invalid_hash));

      let invalid_difficulty_prefix = Block {hash : [1;32], .. valid_block.clone()};
      assert_ne!(Ok(()), chain.try_push_block(invalid_difficulty_prefix));
    }
}
