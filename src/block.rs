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

// For validating whether one block is a valid next block for another.
#[derive(Debug, Clone)]
pub enum NextBlockErr {
    DifficultyCheckFailed {
        block_idx: usize,
        hash_binary: String,
        difficulty_prefix: String,
    },
    InconsistentHash {
        block_idx: usize,
        stored_hash: String,
        computed_hash: String,
    },
    InvalidIndex {
        block_idx: usize
    },
    InvalidChild {
        block_idx: usize,
        block_prev_hash: String,
        parent_block_idx: usize,
        parent_block_hash: String,
    } // Block has an inconsistent prev_hash and/or index with a specified parent
}

impl std::fmt::Display for NextBlockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockErr::DifficultyCheckFailed { block_idx, hash_binary, difficulty_prefix } => {
                write!(f, "Block {}'s hash {} does not meet the difficulty target {}.", block_idx, hash_binary, difficulty_prefix)
            }
            NextBlockErr::InconsistentHash { block_idx, stored_hash, computed_hash } => {
                write!(f, "Block {}'s stored hash {} does not match its computed hash {}.", block_idx, stored_hash, computed_hash)
            }
            NextBlockErr::InvalidIndex { block_idx } => {
                write!(f, "Block {} has invalid index.", block_idx)
            }
            NextBlockErr::InvalidChild { block_idx, block_prev_hash, parent_block_idx, parent_block_hash } => {
                write!(f, "Block {} with prev_hash {} should not be a child of Block {} with hash {}.", block_idx, block_prev_hash, parent_block_idx, parent_block_hash)
            }
        }
    }
}

impl std::error::Error for NextBlockErr {}


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
    pub fn validate_block(block: &Block) -> Result<(), NextBlockErr> {
        //   1. check if block's hash (in binary) has a valid number of leading zeros, ignoring the genesis block
        let BinaryString(hash_binary) = BinaryString::from_hex(&block.hash)
            .expect("Can convert hex string to binary");
        if block.idx != 0 && !hash_binary.starts_with(DIFFICULTY_PREFIX) {
            return Err(NextBlockErr::DifficultyCheckFailed {
                block_idx:block.idx,
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
            return Err(NextBlockErr::InconsistentHash {
                block_idx: block.idx,
                stored_hash: block.hash.clone(),
                computed_hash,
            });
        }

        Ok(())
    }

    pub fn validate_child(parent: &Block, child: &Block) -> Result<(), NextBlockErr>  {
        if parent.hash != child.prev_hash || parent.idx + 1 != child.idx {
            return Err(NextBlockErr::InvalidChild  {
                block_idx: child.idx,
                block_prev_hash: child.prev_hash.to_string(),
                parent_block_idx: parent.idx,
                parent_block_hash: parent.hash.to_string()
            });
        }
        Ok(())
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "\
            ================================================\n\
            Index:           {}\n\
            Data:            {}\n\
            Previous Hash:   {}\n\
            Hash:            {}\n\
            ================================================",
            self.idx,
            self.data,
            self.prev_hash,
            self.hash,
        )
        // write!(
        //     f,
        //     "\
        //     ================================================\n\
        //     Block:\n\
        //     Index:           {}\n\
        //     Timestamp:       {}\n\
        //     Data:            {}\n\
        //     Nonce:           {}\n\
        //     Previous Hash:   {}\n\
        //     Hash:            {}\n\
        //     ================================================",
        //     self.idx,
        //     DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp"),
        //     self.data,
        //     self.nonce,
        //     self.prev_hash,
        //     self.hash,
        // )
    }
}
