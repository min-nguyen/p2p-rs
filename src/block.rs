/*
    *Block*: Provides the block and Proof-of-Work mining algorithm.
    - Block internals and block error types
    - Methods for hashing, mining, and validating a block.
*/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use to_binary::BinaryString;
use log::info;

use super::cryptutil;

// number of leading zeros required for the hashed block for the block to be valid.
const DIFFICULTY_PREFIX: &str = "00";

/* Block
  Records some or all of the most recent data not yet validated by the network.
*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    // position in the chain
    pub idx: usize,
    // core content
    pub data: String,
    // block creation time
    pub timestamp: i64,
    // reference to the previous block's hash
    pub prev_hash: String,
    // arbitrary value controlled by miner to find a valid block hash
    pub nonce: u64,
    // hash of the above
    pub hash: String,
}

#[derive(Debug)]
pub enum BlockErr {
    DifficultyCheckFailed {
        hash_binary: String,
        difficulty_prefix: String,
    },                             // Block's hash does not meet the difficulty target
    HashMismatch {
        stored_hash: String,
        computed_hash: String,
    },                             // Block's stored hash is inconsistent with its computed hash
}

impl Block {
    // Genesis block, the very first block in a chain which never references any previous blocks.
    pub fn genesis() -> Block {
      let (idx, data, timestamp, prev_hash, nonce)
          = (0, "genesis".to_string(), Utc::now().timestamp(), cryptutil::encode_bytes_to_hex(&cryptutil::ZERO_U32), 0);
      let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
      Block { idx, data, timestamp, prev_hash, nonce, hash }
    }

    // Find a valid nonce and hash to construct a new block
    pub fn mine_block(idx: usize, data: &str, prev_hash: &String) -> Block {
        let now: DateTime<Utc> = Utc::now();
        info!("mining block for:\n
                Block {{ idx: {}, data: {}, timestamp: {}, prev_hash: {}, nonce: ?, hash: ? }}"
                , idx, data, now, prev_hash);

        let mut nonce: u64 = 0;
        loop {
            let hash: String
                = Self::compute_hash(idx, data, now.timestamp(), &prev_hash, nonce);
            let BinaryString(hash_bin)
                = BinaryString::from_hex(&hash).expect("can convert hex string to binary");

            if hash_bin.starts_with(DIFFICULTY_PREFIX) {
                info!(
                    "mine_block(): mined! \n nonce: {}, hash: {}, hash (bin repr): {}"
                    , nonce, hash, hash_bin
                );
                return Self { idx, data : data.to_string(), timestamp: now.timestamp(), prev_hash: prev_hash.clone(), nonce, hash  }
            }
            nonce += 1;
        }
    }

    // Compute the hex-string of a sha256 hash (i.e. a 32-byte array) of a block
    fn compute_hash (idx: usize, data: &str, timestamp: i64, prev_hash: &String, nonce: u64)  -> String {
        use sha2::{Sha256, Digest};

        // create a sha256 hasher instance
        let mut hasher: Sha256 = Sha256::new();

        // translate the block from a json value -> string -> byte array &[u8], used as input data to the hasher
        let block_json: serde_json::Value = serde_json::json!({
            "idx": idx,
            "data": data,
            "timestamp": timestamp,
            "prev_hash": prev_hash,
            "nonce": nonce
        });
        hasher.update(block_json.to_string().as_bytes());

        // retrieve hash result
        let hash : [u8; 32] = hasher
        .finalize() // Sha256 -> GenericArray<u8, U32>
        .into(); // GenericArray<u8, U32> -> [u8; 32].

        cryptutil::encode_bytes_to_hex(&hash)
    }

    // Validate a block as its own entity:
    pub fn validate_block(block: &Block) -> Result<(), BlockErr> {
        //    1. check if block's hash (in binary) has a valid number of leading zeros
        let BinaryString(hash_binary) = BinaryString::from_hex(&block.hash)
            .expect("Can convert hex string to binary");
        if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
            return Err(BlockErr::DifficultyCheckFailed {
                hash_binary,
                difficulty_prefix: DIFFICULTY_PREFIX.to_string(),
            })
        }
        //    2. check if block's hash is indeed the correct hash of itself.
        let computed_hash = Self::compute_hash(
            block.idx,
            &block.data,
            block.timestamp,
            &block.prev_hash,
            block.nonce,
        );
        if block.hash != computed_hash {
            return Err(BlockErr::HashMismatch {
                stored_hash: block.hash.clone(),
                computed_hash,
            });
        }

        Ok(())
    }
    pub fn pretty_print(&self) {
        println!("Block {}", self.idx);
        println!("Timestamp: {}", self.timestamp);
        println!("Data: {}", self.data);
        println!("Nonce: {}", self.nonce);
        println!("Previous Hash: {}", &self.prev_hash);
        println!("Hash: {}", &self.hash);
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "\
            ================================================\n\
            Block:\n\
            Index:           {}\n\
            Timestamp:       {}\n\
            Data:            {}\n\
            Nonce:           {}\n\
            Previous Hash:   {}\n\
            Hash:            {}\n\
            ================================================\n",
            self.idx,
            DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp"),
            self.data,
            self.nonce,
            self.prev_hash,
            self.hash,
        )
    }
}

/********  TESTS **********/
#[cfg(test)] // cargo test block -- --nocapture
mod block_tests {
    use crate::{
        block::{Block, BlockErr},
        cryptutil::{debug, encode_bytes_to_hex, ZERO_U32}};

    #[test]
    fn test_invalid_block_difficulty_check() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        let invalid_difficulty_prefix = Block {
            hash: hex::encode([255; 32]),
            ..valid_block.clone()
        };

        // Ensure that the block fails due to a difficulty check error
        assert!(matches!(
            Block::validate_block(&invalid_difficulty_prefix),
            Err(BlockErr::DifficultyCheckFailed { .. })
        ));
    }
    #[test]
    fn test_invalid_block_hash_mismatch() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        let invalid_hash = Block {
            hash: encode_bytes_to_hex(ZERO_U32),
            ..valid_block.clone()
        };

        assert!(matches!(
            debug(Block::validate_block(&invalid_hash)),
            Err(BlockErr::HashMismatch { .. })
        ));
    }
    #[test]
    fn test_valid_block() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        assert!(matches!(
            Block::validate_block(&valid_block),
            Ok(())
        ));
    }
}
