/*
    *Block*: Provides the block and Proof-of-Work mining algorithm.
    - Block internals.
    - Methods for hashing, mining, and validating blocks.
    - Result and error types from handling new blocks.
*/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use to_binary::BinaryString;
use log::info;

use super::cryptutil::{self, pretty_hex};

// number of leading zeros required for the hashed block for the block to be valid.
const DIFFICULTY_PREFIX: &str = "00";

/* Block
  Records some or all of the most recent data not yet validated by the network.
*/
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
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
    // Construct a genesis block
    pub fn genesis() -> Block {
      let (idx, data, timestamp, prev_hash, nonce)
          = (0, "genesis".to_string(), Utc::now().timestamp(), cryptutil::encode_bytes_to_hex(&cryptutil::ZERO_U32), 0);
      let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
      Block { idx, data, timestamp, prev_hash, nonce, hash }
    }

    // Unsafe push
    pub fn push(blocks: &mut Vec<Block>, new_block: &Block){
        blocks.push(new_block.clone());
    }

    // Unsafe append
    pub fn append(blocks_pref: &mut Vec<Block>, blocks_suff: &mut Vec<Block>){
        blocks_pref.append(blocks_suff);
    }

    // Unsafe splitoff
    pub fn split_off(blocks: &mut Vec<Block>, len: usize) -> Vec<Block>{
        blocks.split_off(len)
    }

    // Unsafe splitoff until
    pub fn split_off_until<P>(blocks: &mut Vec<Block>, prop: P) -> Vec<Block>
    where
        P: Fn(&Block) -> bool,
    {
        if let Some(idx) = blocks.iter().position(|block| prop(&block)){
            blocks.split_off(idx + 1)
        }
        else {
            vec![]
        }
    }

    pub fn find<'a>(blocks: &'a Vec<Block>, block_hash: &String) -> Option<&'a Block> {
        blocks.iter().find(|block| &block.hash == block_hash)
    }

    // Find a valid nonce and hash to construct a new block
    pub fn mine_block(last_block: &Block, data: &str) -> Block {
        let idx = last_block.idx + 1;
        let prev_hash = last_block.hash.clone();

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

    // Validate a block as its own entity
    pub fn validate_block(block: &Block) -> Result<(), NextBlockErr> {
        //   check if block's hash has a valid number of leading zeros
        let BinaryString(hash_binary) = BinaryString::from_hex(&block.hash).expect("Can convert hex string to binary");
        if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
            if block.idx != 0 { // ignore the genesis block
                return Err(NextBlockErr::DifficultyCheckFailed {
                    block_idx:block.idx,
                    hash_binary,
                    difficulty_prefix: DIFFICULTY_PREFIX.to_string(),
                })
            }
        }
        //  check if block's hash is indeed the correct hash of itself.
        let computed_hash = Self::compute_hash(block.idx, &block.data, block.timestamp, &block.prev_hash,  block.nonce);
        if block.hash != computed_hash {
            return Err(NextBlockErr::InconsistentHash {
                block_idx: block.idx,
                block_hash: block.hash.clone(),
                computed_hash,
            });
        }

        Ok(())
    }

    // Validate two consecutive blocks
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

    // Validate a non-empty sequence of blocks (i.e. a subchain)
    pub fn validate_blocks(blocks: &Vec<Block>) -> Result<(), NextBlockErr> {
        let mut curr: &Block = blocks.first().expect("Subchain must be non-empty");
        Block::validate_block(curr)?;
        for i in 0..blocks.len() - 1 {
            let next = blocks.get(i + 1).unwrap();
            Block::validate_block(next)?;
            Block::validate_child(curr, next)?;
            curr = next;
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

// The result of adding a new block to a blockchain network
#[derive(Debug)]
pub enum NextBlockResult {
    MissingParent {
        block_idx: usize,
        block_parent_hash: String
    },
    ExtendedMain {
        length: usize,
        endpoint_idx: usize,
        endpoint_hash: String,
    },
    ExtendedFork {
        length: usize,
        forkpoint_idx: usize,
        forkpoint_hash: String,
        endpoint_idx: usize,
        endpoint_hash: String,
    },
    NewFork {
        length: usize,
        forkpoint_idx: usize,
        forkpoint_hash: String,
        endpoint_idx: usize,
        endpoint_hash: String,
    }
    /* To-Do:
    Duplicate Block
    */
}

impl std::fmt::Display for NextBlockResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockResult::MissingParent { block_idx, block_parent_hash } => {
                write!(f, "No update to main chain or forks. The parent of the specified block cannot be found.\n\
                           \tThe missing parent has index and hash ({}, {}).", block_idx - 1, pretty_hex(block_parent_hash))
            }
            NextBlockResult::ExtendedMain { length, endpoint_idx, endpoint_hash } => {
                write!(f, "Extended the main chain.\n\
                           \tIts new endpoint and new length is ({}, {}) and {}.", endpoint_idx, pretty_hex(endpoint_hash), length)
            }
            NextBlockResult::ExtendedFork { length, forkpoint_idx, forkpoint_hash, endpoint_idx,  endpoint_hash} => {
                write!(f,  "Extended an existing fork from from the main chain.\n\
                            \tIts forkpoint and new length from the main chain is ({}, {}) and {}.\n\
                            \tIts new endpoint is ({}, {}).",
                            forkpoint_idx, pretty_hex(forkpoint_hash), length, endpoint_idx, pretty_hex(endpoint_hash)
                )
            }
            NextBlockResult::NewFork { length, forkpoint_idx, forkpoint_hash, endpoint_idx,  endpoint_hash} => {
                match length {
                    1 => writeln!(f, "Added a single-block fork from the main chain."),
                    _ => writeln!(f, "Added a new fork that branches off an existing fork from the main chain.")
                }?;
                write!( f, "\tIts forkpoint and length from the main chain is ({}, {}) and {}. \n\
                            \tIts endpoint is ({}, {}).",
                            forkpoint_idx, pretty_hex(forkpoint_hash), length, endpoint_idx, pretty_hex(endpoint_hash)
                )
            }
            // NextBlockErr::Duplicate { block_idx, block_hash,data } => {
            //     write!(f, "Block {} with hash {} and data {} already exists.", block_idx, block_hash, data)
            // }
        }
    }
}

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
        block_hash: String,
        computed_hash: String,
    },
    InvalidIndex {
        block_idx: usize,
        expected_idx: usize
    },
    InvalidChild { // Block has an inconsistent prev_hash and/or index with a specified parent
        block_idx: usize,
        block_prev_hash: String,
        parent_block_idx: usize,
        parent_block_hash: String,
    },
    EmptyChain
}

impl std::fmt::Display for NextBlockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockErr::DifficultyCheckFailed { block_idx, hash_binary, difficulty_prefix } => {
                write!(f, "Block {}'s hash {} does not meet the difficulty target {}.", block_idx, hash_binary, difficulty_prefix)
            }
            NextBlockErr::InconsistentHash { block_idx, block_hash, computed_hash } => {
                write!(f, "Block {}'s stored hash {} does not match its computed hash {}.", block_idx, block_hash, computed_hash)
            }
            NextBlockErr::InvalidIndex { block_idx, expected_idx } => {
                write!(f, "Block {} has invalid index, whereas we expected index {}.", block_idx, expected_idx)
            }
            NextBlockErr::InvalidChild { block_idx, block_prev_hash, parent_block_idx, parent_block_hash } => {
                write!(f, "Block {} with prev_hash {} should not be a child of Block {} with hash {}.", block_idx, block_prev_hash, parent_block_idx, parent_block_hash)
            },
            NextBlockErr::EmptyChain => {
                write!(f, "Chain is empty.")
            }
        }
    }
}


impl std::error::Error for NextBlockErr {}

