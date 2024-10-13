// #[path = "./../chain.rs"]
// mod chain;
// #[path = "./../transaction.rs"]
// mod transaction;
// #[path = "./../util.rs"]
// mod util;
// use transaction::Transaction;
// use util::{ZERO_U32, ZERO_U64, encode_bytes_to_hex};

// // RUST_LOG=info cargo test
// #[cfg(test)]
// mod block_tests {
    // use libp2p::{core::PublicKey, identity::{self, Keypair}};
    // use util::encode_pubk_to_hex;

    // use super::*;

    // /* transaction tests */
    // #[test]
    // fn test_valid_transaction() {
    //   let keys = identity::Keypair::generate_ed25519();
    //   let valid_txn = Transaction::random_transaction("£0".to_string(), keys);
    //   assert_eq!(Transaction::validate_transaction(&valid_txn), Ok(()));
    // }

    // #[test]
    // fn test_invalid_transaction() {
    //   let keys = identity::Keypair::generate_ed25519();
    //   let valid_txn: Transaction = Transaction::random_transaction("£0".to_string(), keys);

    //   let invalid_hash = Transaction {hash : encode_bytes_to_hex(ZERO_U32), .. valid_txn.clone()};
    //   assert!(matches!(Transaction::validate_transaction(&invalid_hash), Err(_)));

    //   let invalid_pubk = Transaction { sender_pubk : encode_pubk_to_hex(identity::Keypair::generate_ed25519().public()), .. valid_txn.clone()};
    //   assert!(matches!(Transaction::validate_transaction(&invalid_pubk), Err(_)));

    //   let invalid_siglen = Transaction {sig: encode_bytes_to_hex(ZERO_U32), .. valid_txn.clone()};
    //   assert!(matches!(Transaction::validate_transaction(&invalid_siglen), Err(_)));

    //   let invalid_sig = Transaction {sig: encode_bytes_to_hex(ZERO_U64), .. valid_txn.clone()};
    //   assert!(matches!(Transaction::validate_transaction(&invalid_sig), Err(_)));
    // }
// }