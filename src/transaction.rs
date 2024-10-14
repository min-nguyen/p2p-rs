/*
    *Transaction*:
    - Transaction type.
    - Methods for generating and validating transactions.
*/

use core::panic;
use log::error;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime};
use libp2p::{PeerId, identity::{Keypair, PublicKey}};
use crate::cryptutil;

use super::cryptutil::{encode_pubk_to_hex, decode_hex_to_pubk, encode_bytes_to_hex, decode_hex_to_bytes};

const PUBK_U8S_LEN : usize = 36;
const SIG_U8S_LEN : usize = 64;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub sender: String,          // peer id of the sender
    pub sender_pubk: String,     // 32-byte (but stored as 36 bytes!) public key of the sender, assuming ed25519
    pub receiver: String,        // peer id of the receiver
    pub amount: String,          // amount transferred, a string for testing
    pub timestamp: i64,          // creation date

    pub hash: String,            // 32-byte hash of the above data, assuming sha256
    pub sig: String,             // 32-byte signature of the hash, assuming ed25519
}

#[derive(Debug)]
pub enum TransactionErr {
    PubKeyDecodeErr {
        e: cryptutil::HexDecodeErr
    },
    SigDecodeError {
        e: cryptutil::HexDecodeErr
    },
    HashMismatch {
        stored_hash: String,
        computed_hash: String,
    },                           // stored hash is inconsistent with its computed hash
    SigInvalid {
        pubk : String,
        hash : String,
        sig  : String
    }                            // hash and signature couldn't be verified with public key
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let datetime: DateTime<Utc>
            = DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp");
        write!(f, "Transaction {{ sender: {}, receiver: {}, amount: {}, date-time: {}, sig: {:?} }}"
        , self.sender, self.receiver, self.amount, datetime, self.sig)
    }
}

impl Transaction {
    pub fn random_transaction(amount: String, keys : Keypair) -> Self {
        let sender: String = PeerId::from(keys.public()).to_string();
        let sender_pubk: String = encode_pubk_to_hex(keys.public());

        let receiver: String = format!("0x{}", random_string(40));
        let timestamp: i64 = Utc::now().timestamp();
        let hash: String = Self::compute_hash(&sender, &sender_pubk, &receiver, &amount, timestamp);

        let sig: String =
            match keys.sign(&hash.as_bytes()){
                Ok (sig_u8s) => encode_bytes_to_hex(sig_u8s),
                Err (e) => {
                    error!("Signing failed. Couldn't decode public key from hex-string to byte vector: {}", e);
                    panic!()
                }
            };
        Transaction{ sender, sender_pubk, receiver, amount, timestamp, hash, sig }
    }

    fn compute_hash(sender: &String, sender_pk : &String, receiver: &String, amount:  &String, timestamp: i64) -> String {
        let mut hasher: Sha256 = Sha256::new();
        let message: String = format!("{}:{}:{}:{}:{}", sender, sender_pk, receiver, amount, timestamp);
        hasher.update(message);
        encode_bytes_to_hex(hasher.finalize())
    }

    pub fn validate_transaction(txn: &Transaction) -> Result<(), TransactionErr> {
        let hash: String = Transaction::compute_hash(&txn.sender, &txn.sender_pubk, &txn.receiver, &txn.amount, txn.timestamp);
        // check message integrity
        if hash != txn.hash{
            return Err(TransactionErr::HashMismatch { stored_hash: txn.hash.clone(), computed_hash: hash })
        }
        // check message signature
        let pubk: PublicKey =
            match decode_hex_to_pubk(&txn.sender_pubk, PUBK_U8S_LEN) {
                Ok (pubk) => pubk,
                Err (e) => {
                    return Err (TransactionErr::PubKeyDecodeErr {e} );
                }
            };

        let sig_u8s: Vec<u8> =
            match decode_hex_to_bytes(&txn.sig, SIG_U8S_LEN) {
                Ok (sig_u8s) => sig_u8s,
                Err (e) => {
                    return Err (TransactionErr::SigDecodeError { e })
                }
            };

        if !(pubk.verify(hash.as_bytes(), sig_u8s.as_slice())){
            return Err (TransactionErr::SigInvalid { pubk : txn.sender_pubk.clone(), sig : txn.sig.clone(), hash: txn.hash.clone()})
        }
        Ok (())
    }
}

fn random_string(len: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod transaction_tests {
    use libp2p::identity;
    use crate::cryptutil::{debug, encode_bytes_to_hex, encode_pubk_to_hex, ZERO_U32, ZERO_U64};
    use crate::transaction::{Transaction, TransactionErr};

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
        debug(Transaction::validate_transaction(&invalid_hash))
        , Err(TransactionErr::HashMismatch { .. })));

      let invalid_pubk = Transaction { sender_pubk : encode_pubk_to_hex(identity::Keypair::generate_ed25519().public()), .. valid_txn.clone()};
      assert!(matches!(
        debug(Transaction::validate_transaction(&invalid_pubk)),
        Err(TransactionErr::HashMismatch { .. })));

      let invalid_siglen = Transaction {sig: encode_bytes_to_hex(ZERO_U32), .. valid_txn.clone()};
      assert!(matches!(
        debug(Transaction::validate_transaction(&invalid_siglen)),
        Err(TransactionErr::SigDecodeError { .. })));

      let invalid_sig = Transaction {sig: encode_bytes_to_hex(ZERO_U64), .. valid_txn.clone()};
      assert!(matches!(
        debug(Transaction::validate_transaction(&invalid_sig)),
        Err(TransactionErr::SigInvalid { .. })));
    }
  }