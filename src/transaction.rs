use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::Utc;
use rand::{Rng, thread_rng};
use libp2p::{PeerId, identity::{Keypair, PublicKey}};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,          // peer id of the sender
    pub sender_pbk: Vec<u8>,     // public key of the sender
    pub receiver: String,        // peer id of the receiver
    pub amount: u64,             // amount transferred
    pub timestamp: i64,          // when the transaction was created

    pub hash: String,            // hash of the above
    pub sig: Vec<u8>,            // signature of the hash
}

impl Transaction {
    pub fn hash(sender: &String, sender_pk : &Vec<u8>, receiver: &String, amount: u64, timestamp: i64) -> String {
        let mut hasher: Sha256 = Sha256::new();
        let message: String = format!("{}:{}:{}:{}:{}", sender, hex::encode(sender_pk), receiver, amount, timestamp);

        hasher.update(message);
        hex::encode(hasher.finalize())
    }

    pub fn random_transaction(keys : Keypair) -> Self {
        let mut rng = thread_rng();

        let sender = PeerId::from(keys.public()).to_string();
        let sender_pbk: Vec<u8> = keys.public().into_protobuf_encoding();

        let receiver = format!("0x{}", random_string(40));
        let amount = rng.gen_range(1..1001);
        let timestamp = Utc::now().timestamp();
        let hash = Self::hash(&sender, &sender_pbk, &receiver, amount, timestamp);

        let sig = keys.sign(&hash.as_bytes()).expect("Signing failed");

        Transaction{ sender, sender_pbk, receiver, amount, timestamp, hash, sig }
    }

    pub fn verify_transaction(txn: Transaction) -> bool {
        let hash = Transaction::hash(&txn.sender, &txn.sender_pbk, &txn.receiver, txn.amount, txn.timestamp);
        // check message integrity
        if hash != txn.hash{
            eprintln!("Couldn't verify transaction! invalid hash.");
            return false
        }
        // verify message signature
        let pk = PublicKey::from_protobuf_encoding(&txn.sender_pbk).expect("can decode sender public key");
        if !(pk.verify(hash.as_bytes(), &txn.sig)){
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