#[path = "./../chain.rs"]
mod chain;
#[path = "./../transaction.rs"]
mod transaction;
#[path = "./../util.rs"]
mod util;
use chain::{Chain, Block};
use transaction::Transaction;
use util::{ZERO_U32, ZERO_U64, encode_hex};

// RUST_LOG=info cargo test
#[cfg(test)]
mod block_tests {
    use libp2p::{core::PublicKey, identity::{self, Keypair}};
    use util::encode_pubk;

    use super::*;

    /* transaction tests */
    #[test]
    fn test_valid_transaction() {
      let keys = identity::Keypair::generate_ed25519();
      let valid_txn = Transaction::random_transaction("£0".to_string(), keys);
      assert_eq!(Transaction::validate_transaction(&valid_txn), Ok(()));
    }

    #[test]
    fn test_invalid_transaction() {
      let keys = identity::Keypair::generate_ed25519();
      let valid_txn: Transaction = Transaction::random_transaction("£0".to_string(), keys);

      let invalid_hash = Transaction {hash : encode_hex(ZERO_U32), .. valid_txn.clone()};
      assert!(matches!(Transaction::validate_transaction(&invalid_hash), Err(_)));

      let invalid_pubk = Transaction { sender_pubk : encode_pubk(identity::Keypair::generate_ed25519().public()), .. valid_txn.clone()};
      assert!(matches!(Transaction::validate_transaction(&invalid_pubk), Err(_)));

      let invalid_siglen = Transaction {sig: encode_hex(ZERO_U32), .. valid_txn.clone()};
      assert!(matches!(Transaction::validate_transaction(&invalid_siglen), Err(_)));

      let invalid_sig = Transaction {sig: encode_hex(ZERO_U64), .. valid_txn.clone()};
      assert!(matches!(Transaction::validate_transaction(&invalid_sig), Err(_)));
    }

    /* low-level block tests */
    #[test]
    fn test_valid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      assert!(matches!(Block::validate_block(&gen, &valid_block), Ok(())));
    }

    #[test]
    fn test_invalid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      let invalid_idx = Block {idx : 0, .. valid_block.clone()};
      assert!(matches!(Block::validate_block(&gen, &invalid_idx), Err(_)));

      let invalid_prev_hash = Block { prev_hash : encode_hex(ZERO_U32), .. valid_block.clone() };
      assert!(matches!(Block::validate_block(&gen, &invalid_prev_hash), Err(_)));

      let invalid_hash = Block {hash : encode_hex(ZERO_U32), .. valid_block.clone()};
      assert!(matches!(Block::validate_block(&gen, &invalid_hash), Err(_)));

      let invalid_difficulty_prefix = Block {hash :  hex::encode([1;32]), .. valid_block.clone()};
      assert!(matches!(Block::validate_block(&gen, &invalid_difficulty_prefix), Err(_)));
    }

    /* high-level chain tests */
    #[test]
    fn test_extend_chain_once() {
      let mut chain: Chain = Chain::new();
      chain.make_new_valid_block("test");
      assert_eq!(Chain::validate_chain(&chain), Ok(()));
    }

    #[test]
    fn test_extend_chain_many() {
      let mut chain: Chain = Chain::new();
      for _ in 0 .. 10 {
        chain.make_new_valid_block("test");
      }
      assert_eq!(Chain::validate_chain(&chain), Ok(()));
    }

}
