use core::panic;

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{Utc, DateTime};
use rand::{Rng, thread_rng};
use libp2p::{PeerId, identity::{Keypair, PublicKey}};
use super::util::{encode_pubk, decode_pubk, encode_hex, decode_hex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,          // peer id of the sender
    pub sender_pbk: String,      // 32-byte public key of the sender, assuming ed25519
    pub receiver: String,        // peer id of the receiver
    pub amount: u64,             // amount transferred
    pub timestamp: i64,          // creation date

    pub hash: String,            // 32-byte hash of the above data, assuming sha256
    pub sig: String,             // 64-byte signature of the hash, assuming ed25519
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
    pub fn compute_sha256(sender: &String, sender_pk : &String, receiver: &String, amount: u64, timestamp: i64) -> String {
        let mut hasher: Sha256 = Sha256::new();
        let message: String = format!("{}:{}:{}:{}:{}", sender, sender_pk, receiver, amount, timestamp);

        hasher.update(message);
        encode_hex(hasher.finalize())
    }

    pub fn random_transaction(keys : Keypair) -> Self {
        let mut rng: rand::prelude::ThreadRng = thread_rng();

        let sender: String = PeerId::from(keys.public()).to_string();
        let sender_pbk: String = encode_pubk(keys.public());

        let receiver: String = format!("0x{}", random_string(40));
        let amount: u64 = rng.gen_range(1..1001);
        let timestamp: i64 = Utc::now().timestamp();
        let hash: String = Self::compute_sha256(&sender, &sender_pbk, &receiver, amount, timestamp);

        let sig: String =
            match keys.sign(&hash.as_bytes()){
                Ok (sig_u8s) => encode_hex(sig_u8s),
                Err (e) => {
                    eprintln!("Signing failed. Couldn't decode public key from hex-string to byte vector: {}", e);
                    panic!()
                }
            };
        println!("sig length: {}", sig.len());
        Transaction{ sender, sender_pbk, receiver, amount, timestamp, hash, sig }
    }

    pub fn verify_transaction(txn: Transaction) -> bool {
        let hash = Transaction::compute_sha256(&txn.sender, &txn.sender_pbk, &txn.receiver, txn.amount, txn.timestamp);
        // check message integrity
        if hash != txn.hash{
            eprintln!("Verify transaction failed. Invalid hash.");
            return false
        }
        // verify message signature
        let pubk: PublicKey =
            match decode_pubk(&txn.sender_pbk, 32) {
                Ok (pubk) => pubk,
                Err (e) => {
                    eprintln!("Verify transaction failed. Couldn't decode public key: {}", e);
                    return false
                }
            };
        let sig_u8s: Vec<u8> =
            match decode_hex(&txn.sig, 64) {
                Ok (sig_u8s) => sig_u8s,
                Err (e) => {
                    eprintln!("Verify transaction failed. Couldn't decode public key from hex-string to byte vector: {}", e);
                    return false
                }
            };
        if !(pubk.verify(hash.as_bytes(), sig_u8s.as_slice())){
            eprintln!("Couldn't verify transaction! invalid signature.");
            return false
        }
        eprintln!("Transaction verified!");
        true
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