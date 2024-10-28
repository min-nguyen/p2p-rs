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
          = (0, "genesis".to_string(), 1730051971, cryptutil::encode_bytes_to_hex(&cryptutil::ZERO_U32), 0);
      let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
      Block { idx, data, timestamp, prev_hash, nonce, hash }
    }

    // Unsafe push
    pub fn push_end(blocks: &mut Vec<Block>, new_block: Block){
        blocks.push(new_block.clone());
    }

    // Unsafe push
    pub fn push_front(blocks: &mut Vec<Block>, new_block: Block){
        blocks.insert(0, new_block.clone());
    }

    // Unsafe append
    pub fn append(blocks_pref: &mut Vec<Block>,  blocks_suff: &mut Vec<Block>){
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

    pub fn find<'a, P>(blocks: &'a Vec<Block>, prop: P) -> Option<&'a Block>
    where
        P: Fn(&Block) -> bool,
    {
        blocks.iter().find(|block|  prop(&block))
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
        let mut curr: &Block = blocks.first().ok_or(NextBlockErr::NoBlocks)?;
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
    ExtendedMain {
        end_idx: usize,
        end_hash: String,
    },
    ExtendedFork {
        fork_idx: usize,
        fork_hash: String,
        end_idx: usize,
        end_hash: String,
    },
    NewFork {
        fork_idx: usize,
        fork_hash: String,
        end_idx: usize,
        end_hash: String,
    }
    /* To-Do:
    Duplicate Block
    */
}

impl std::fmt::Display for NextBlockResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockResult::ExtendedMain { end_idx, end_hash } => {
                write!(f, "Extended the main chain.\n\
                           \tIts endpoint is ({}, {})", end_idx, pretty_hex(end_hash))
            }
            NextBlockResult::ExtendedFork { fork_idx, fork_hash, end_idx,  end_hash} => {
                write!(f,  "Extended an existing fork from the main chain.\n\
                            \tIts forkpoint is ({}, {}) and endpoint is ({}, {}).",
                            fork_idx, pretty_hex(fork_hash), end_idx, pretty_hex(end_hash)
                )
            }
            NextBlockResult::NewFork { fork_idx, fork_hash, end_idx,  end_hash} => {
                match end_idx - fork_idx {
                    1 => writeln!(f, "Added a single-block fork from the main chain."),
                    _ => writeln!(f, "Added a new fork that branches off an existing fork from the main chain.")
                }?;
                write!( f, "\tIts forkpoint is ({}, {}) and endpoint is ({}, {}).",
                            fork_idx, pretty_hex(fork_hash), end_idx, pretty_hex(end_hash)
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
    InvalidChild {
        block_idx: usize,
        block_prev_hash: String,
        parent_block_idx: usize,
        parent_block_hash: String,
    }, // Block has an inconsistent prev_hash and/or index with a specified parent
    UnrelatedGenesis {
        genesis_hash: String
    },  // Block belongs to a chain with a different genesis root
    MissingParent {
        block_parent_idx: usize,
        block_parent_hash: String
    },  // Block is missing a parent that connects it to the main chain or forks
    StrayParent {
        block_idx: usize,
        block_hash: String,
    }, // Block represents a missing parent that but connecting to any orphaned branches,
    Duplicate {
        block_idx: usize,
        block_hash: String,
    }, // Block exists in the main chain, forks, or orphans
    NoBlocks
       // Block used in a context with an empty chain or fork
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
            }
            NextBlockErr::MissingParent { block_parent_idx, block_parent_hash } => {
                write!(f, "Block {} is missing its parent {} with hash {} in the chain or forks.", block_parent_idx + 1, block_parent_idx, pretty_hex(block_parent_hash))
            }
            NextBlockErr::Duplicate {block_idx, block_hash} => {
                write!(f, "Block {} with hash {} is a duplicate, already stored in the main chain or forks.", block_idx, block_hash)
            }
            NextBlockErr::UnrelatedGenesis {genesis_hash} => {
                write!(f, "Block(s) belong to a chain with a different genesis, {}.", pretty_hex(genesis_hash))
            }
            NextBlockErr::NoBlocks => {
                write!(f, "Chain or fork is empty.")
            }
            NextBlockErr::StrayParent { block_idx, block_hash } => {
                write!(f, "Block {} with hash {} represents a missing parent but has no children in the orphans.", block_idx, block_hash)
            }
        }
    }
}


impl std::error::Error for NextBlockErr {}

