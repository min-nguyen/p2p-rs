#[path = "./../chain.rs"]
mod chain;
#[path = "./../transaction.rs"]
mod transaction;
use chain::{
    Chain, Block
};
use transaction::{
    Transaction
};

// RUST_LOG=info cargo test
#[cfg(test)]
mod block_tests {
    use libp2p::identity;

    use super::*;

    /* transaction tests */
    #[test]
    fn test_transaction() {
      let keys = identity::Keypair::generate_ed25519();
      let txn = Transaction::random_transaction(keys);
      assert_eq!(true, Transaction::verify_transaction(txn));
    }

    /* low-level block tests */
    #[test]
    fn test_valid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      assert_eq!(true, Block::valid_block(&gen, &valid_block));
    }

    #[test]
    fn test_invalid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      let invalid_idx = Block {idx : 0, .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_idx));

      let invalid_prev_hash = Block { prev_hash : hex::encode([0;32]), .. valid_block.clone() };
      assert_eq!(false, Block::valid_block(&gen, &invalid_prev_hash));

      let invalid_hash = Block {hash :  hex::encode([0;32]), .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_hash));

      let invalid_difficulty_prefix = Block {hash :  hex::encode([1;32]), .. valid_block.clone()};
      assert_eq!(false, Block::valid_block(&gen, &invalid_difficulty_prefix));
    }

    /* high-level chain tests */
    #[test]
    fn test_extend_chain_once() {
      let mut chain: Chain = Chain::new();
      chain.make_new_valid_block("test");
      assert_eq!(true, Chain::valid_chain(&chain))
    }

    #[test]
    fn test_extend_chain_many() {
      let mut chain: Chain = Chain::new();
      for _ in 0 .. 10 {
        chain.make_new_valid_block("test");
      }
      assert_eq!(true, Chain::valid_chain(&chain));
    }

}
