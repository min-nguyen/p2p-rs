
/******************
      TESTS
********************/
#[cfg(test)]
mod transaction_tests {
    use libp2p::identity;
    use crate::{
      crypt::{encode_bytes_to_hex, encode_pubk_to_hex, ZERO_U32, ZERO_U64},
      util::trace,
      transaction::{Transaction, TransactionErr}
    };

    /* transaction tests */
    #[test]
    fn test_valid_transaction() {
      let keys = identity::Keypair::generate_ed25519();
      let valid_txn = Transaction::random_transaction("£0".to_string(), keys);
      assert!(matches!(
        Transaction::validate_transaction(&valid_txn),
        Ok(())));
    }

    #[test]
    fn test_invalid_transaction() {
      let keys = identity::Keypair::generate_ed25519();
      let valid_txn: Transaction = Transaction::random_transaction("£0".to_string(), keys);

      let invalid_hash = Transaction {hash : encode_bytes_to_hex(ZERO_U32), .. valid_txn.clone()};
      assert!(matches!(
        trace(Transaction::validate_transaction(&invalid_hash))
        , Err(TransactionErr::HashMismatch { .. })));

      let invalid_pubk = Transaction { sender_pubk : encode_pubk_to_hex(identity::Keypair::generate_ed25519().public()), .. valid_txn.clone()};
      assert!(matches!(
        trace(Transaction::validate_transaction(&invalid_pubk)),
        Err(TransactionErr::HashMismatch { .. })));

      let invalid_siglen = Transaction {sig: encode_bytes_to_hex(ZERO_U32), .. valid_txn.clone()};
      assert!(matches!(
        trace(Transaction::validate_transaction(&invalid_siglen)),
        Err(TransactionErr::SigDecodeError { .. })));

      let invalid_sig = Transaction {sig: encode_bytes_to_hex(ZERO_U64), .. valid_txn.clone()};
      assert!(matches!(
        trace(Transaction::validate_transaction(&invalid_sig)),
        Err(TransactionErr::SigInvalid { .. })));
    }
  }