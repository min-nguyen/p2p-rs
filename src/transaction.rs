/*
    *Transaction*:
    - Transaction type.
    - Methods for generating and validating transactions.
*/

use super::
    crypt::{HexDecodeErr, encode_pubk_to_hex, decode_hex_to_pubk, encode_bytes_to_hex, decode_hex_to_bytes, random_string};

use core::panic;
use std::fmt;
use log::error;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime};
use libp2p::{PeerId, identity::{Keypair, PublicKey}};

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


impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "\
            ================================================\n\
            Transaction:\n\
            Sender:          {}\n\
            Sender PubKey:   {}\n\
            Receiver:        {}\n\
            Amount:          {}\n\
            Timestamp:       {}\n\
            Hash:            {}\n\
            Signature:       {}\n\
            ================================================",
            self.sender,
            self.sender_pubk,
            self.receiver,
            self.amount,
            DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp"),
            self.hash,
            self.sig
        )
    }
}

#[derive(Debug)]
pub enum TransactionErr {
    PubKeyDecodeErr {
        e: HexDecodeErr
    },
    SigDecodeError {
        e: HexDecodeErr
    },
    HashMismatch {
        stored_hash: String,
        computed_hash: String,
    },
    SigInvalid {
        pubk : String,
        hash : String,
        sig  : String
    }
}

impl fmt::Display for TransactionErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionErr::PubKeyDecodeErr { e } => {
                write!(f, "Public Key Decode Error: {}", e)
            }
            TransactionErr::SigDecodeError { e } => {
                write!(f, "Signature Decode Error: {}", e)
            }
            TransactionErr::HashMismatch { stored_hash, computed_hash } => {
                write!(f, "Hash Mismatch: stored hash ({}) does not match computed hash ({})",
                    stored_hash, computed_hash
                )
            }
            TransactionErr::SigInvalid { pubk, hash, sig } => {
                write!(f, "Signature Invalid: public key ({}), hash ({}), signature ({})",
                    pubk, hash, sig
                )
            }
        }
    }
}
