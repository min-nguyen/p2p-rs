#[path = "./../block.rs"]
mod block;
use block::{
    Chain, Block
};

// RUST_LOG=info cargo test
#[cfg(test)]
mod tests {
    use super::*;

    /* low-level block tests */
    #[test]
    fn test_valid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", gen.hash);

      assert_eq!(true, Block::valid_block(&gen, &valid_block));
    }

    #[test]
    fn test_invalid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", gen.hash);

      let invalid_idx = Block {idx : 0, .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_idx));

      let invalid_prev_hash = Block { prev_hash : [0;32], .. valid_block.clone() };
      assert_eq!(false, Block::valid_block(&gen, &invalid_prev_hash));

      let invalid_hash = Block {hash : [0;32], .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_hash));

      let invalid_difficulty_prefix = Block {hash : [1;32], .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_difficulty_prefix));
    }

    /* high-level chain tests */
    #[test]
    fn test_extend_chain_once() {
      let mut chain: Chain = Chain::new();
      chain.make_new_valid_block("test".to_string());
      assert_eq!(true, Chain::valid_chain(&chain))
    }

    #[test]
    fn test_extend_chain_many() {
      let mut chain: Chain = Chain::new();
      for _ in 0 .. 10 {
        chain.make_new_valid_block("test".to_string());
      }
      assert_eq!(true, Chain::valid_chain(&chain));
    }

}
