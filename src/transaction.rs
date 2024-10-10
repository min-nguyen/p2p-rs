use core::panic;

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime};
use libp2p::{PeerId, identity::{Keypair, PublicKey}};
use super::util::{encode_pubk, decode_pubk, encode_hex, decode_hex};

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
        let sender_pubk: String = encode_pubk(keys.public());

        let receiver: String = format!("0x{}", random_string(40));
        let timestamp: i64 = Utc::now().timestamp();
        let hash: String = Self::compute_sha256(&sender, &sender_pubk, &receiver, &amount, timestamp);

        let sig: String =
            match keys.sign(&hash.as_bytes()){
                Ok (sig_u8s) => encode_hex(sig_u8s),
                Err (e) => {
                    eprintln!("Signing failed. Couldn't decode public key from hex-string to byte vector: {}", e);
                    panic!()
                }
            };
        Transaction{ sender, sender_pubk, receiver, amount, timestamp, hash, sig }
    }
    pub fn verify_transaction(txn: &Transaction) -> bool {
        let hash: String = Transaction::compute_sha256(&txn.sender, &txn.sender_pubk, &txn.receiver, &txn.amount, txn.timestamp);
        // check message integrity
        if hash != txn.hash{
            eprintln!("Verify transaction failed. Invalid hash.");
            return false
        }
        // verify message signature
        let pubk: PublicKey =
            match decode_pubk(&txn.sender_pubk, PUBK_U8S_LEN) {
                Ok (pubk) => pubk,
                Err (e) => {
                    eprintln!("Verify transaction failed. Couldn't decode public key: {}", e);
                    return false
                }
            };
        let sig_u8s: Vec<u8> =
            match decode_hex(&txn.sig, SIG_U8S_LEN) {
                Ok (sig_u8s) => sig_u8s,
                Err (e) => {
                    eprintln!("Verify transaction failed. Couldn't decode signature: {}", e);
                    return false
                }
            };
        if !(pubk.verify(hash.as_bytes(), sig_u8s.as_slice())){
            eprintln!("Verify transaction failed. Invalid signature.");
            return false
        }
        true
    }

    pub fn compute_sha256(sender: &String, sender_pk : &String, receiver: &String, amount:  &String, timestamp: i64) -> String {
        let mut hasher: Sha256 = Sha256::new();
        let message: String = format!("{}:{}:{}:{}:{}", sender, sender_pk, receiver, amount, timestamp);
        hasher.update(message);
        encode_hex(hasher.finalize())
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