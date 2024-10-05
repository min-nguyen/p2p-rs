use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chrono::{Utc};
use rand::{Rng, thread_rng};
use libp2p::{PeerId};

/// A struct representing a blockchain transaction.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,          // Address of the sender (could be a public key)
    pub receiver: String,        // Address of the receiver (could be a public key)
    pub amount: u64,             // Amount of currency/asset being transferred
    pub timestamp: i64,          // Unix timestamp when the transaction was created
    pub signature: String,        // Signature for verifying the authenticity of the transaction
}

impl Transaction {
    /// Create a new transaction
    pub fn new(sender: String, receiver: String, amount: u64, signature: String) -> Self {
        Transaction {
            sender,
            receiver,
            amount,
            timestamp: Utc::now().timestamp(),
            signature,
        }
    }

    /// Generate a hash of the transaction for integrity verification
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", self)); // Create a hash of the transaction's string representation
        let result = hasher.finalize();
        hex::encode(result) // Convert the hash to a hexadecimal string
    }

    // pub fn random_transaction(sender: PeerId, Pr) -> Self {
    //   let mut rng = thread_rng();

    //   // Generate random sender and receiver
    //   let receiver = format!("0x{}", random_string(40));

    //   // Generate a random amount between 1 and 1000
    //   let amount = rng.gen_range(1..1001);

    //   // Get the current timestamp
    //   let timestamp = Utc::now().timestamp() as u64; // Unix timestamp

    //   // Generate a random signature (for demonstration, this would usually be a proper cryptographic signature)
    //   let signature = random_string(64); // Random string to simulate a signature

    //   Transaction::new(sender.to_string(), receiver, amount, timestamp, signature)
    // }
}