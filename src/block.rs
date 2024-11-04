/*
    *Block*: Provides the block and Proof-of-Work mining algorithm.
    - Block internals.
    - Methods for hashing, mining, and validating blocks.
    - Result and error types from handling new blocks.
*/

use std::ops::Deref;

use super::{crypt, util::abbrev};
use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use to_binary::BinaryString;

const DIFFICULTY_PREFIX: &str = "00";

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

/* Block core operations */
impl Block {
    // Construct a genesis block
    pub fn genesis() -> Block {
        let (idx, data, timestamp, prev_hash, nonce) = (
            0,
            "genesis".to_string(),
            1730051971,
            crypt::encode_bytes_to_hex(&crypt::ZERO_U32),
            0,
        );
        let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
        Block {
            idx,
            data,
            timestamp,
            prev_hash,
            nonce,
            hash,
        }
    }

    // Find a valid nonce and hash to construct a new block
    pub fn mine_block(last_block: &Block, data: &str) -> Block {
        let idx = last_block.idx + 1;
        let prev_hash = last_block.hash.clone();

        let now: DateTime<Utc> = Utc::now();
        info!(
            "mining block for:\n
                Block {{ idx: {}, data: {}, timestamp: {}, prev_hash: {}, nonce: ?, hash: ? }}",
            idx, data, now, prev_hash
        );

        let mut nonce: u64 = 0;
        loop {
            let hash: String = Self::compute_hash(idx, data, now.timestamp(), &prev_hash, nonce);
            let BinaryString(hash_bin) =
                BinaryString::from_hex(&hash).expect("can convert hex string to binary");

            if hash_bin.starts_with(DIFFICULTY_PREFIX) {
                info!(
                    "mine_block(): mined! \n nonce: {}, hash: {}, hash (bin repr): {}",
                    nonce, hash, hash_bin
                );
                return Self {
                    idx,
                    data: data.to_string(),
                    timestamp: now.timestamp(),
                    prev_hash: prev_hash.clone(),
                    nonce,
                    hash,
                };
            }
            nonce += 1;
        }
    }

    // Compute the hex-string of a sha256 hash (i.e. a 32-byte array) of a block
    fn compute_hash(
        idx: usize,
        data: &str,
        timestamp: i64,
        prev_hash: &String,
        nonce: u64,
    ) -> String {
        use sha2::{Digest, Sha256};

        // create a sha256 hasher instance
        let mut hasher: Sha256 = Sha256::new();

        // translate the block from a json value -> string -> byte array &[u8], used as input data to the hasher
        let json: serde_json::Value = serde_json::json!({
            "idx": idx,
            "data": data,
            "timestamp": timestamp,
            "prev_hash": prev_hash,
            "nonce": nonce
        });
        hasher.update(json.to_string().as_bytes());

        // retrieve hash result
        let hash: [u8; 32] = hasher
            .finalize() // Sha256 -> GenericArray<u8, U32>
            .into(); // GenericArray<u8, U32> -> [u8; 32].

        crypt::encode_bytes_to_hex(&hash)
    }

    // Validate a block as its own entity
    pub fn validate(&self) -> Result<(), NextBlockErr> {
        //   check if block's hash has a valid number of leading zeros
        let BinaryString(hash_binary) =
            BinaryString::from_hex(&self.hash).expect("Can convert hex string to binary");
        if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
            if self.idx != 0 {
                // ignore the genesis block
                return Err(NextBlockErr::DifficultyCheckFailed {
                    idx: self.idx,
                    hash: self.hash.clone(),
                    difficulty_prefix: DIFFICULTY_PREFIX.to_string(),
                });
            }
        }
        //  check if block's hash is indeed the correct hash of itself.
        let computed_hash = Self::compute_hash(
            self.idx,
            &self.data,
            self.timestamp,
            &self.prev_hash,
            self.nonce,
        );
        if self.hash != computed_hash {
            return Err(NextBlockErr::InconsistentHash {
                idx: self.idx,
                hash: self.hash.clone(),
                computed_hash,
            });
        }

        Ok(())
    }

    // Validate two consecutive blocks
    pub fn validate_parent(&self, parent: &Block) -> Result<(), NextBlockErr> {
        parent.validate()?;
        if parent.hash != self.prev_hash || parent.idx + 1 != self.idx {
            return Err(NextBlockErr::InvalidChild {
                idx: self.idx,
                prev_hash: self.prev_hash.to_string(),
                parent_idx: parent.idx,
                parent_hash: parent.hash.to_string(),
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
            self.idx, self.data, self.prev_hash, self.hash,
        )
    }
}

/* Blocks: Ensures a valid subchain i.e. a non-empty sequence of blocks where each block correctly references the preceding one */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blocks(Vec<Block>);

impl Blocks {
    // Construct a genesis block
    pub fn genesis() -> Blocks {
        Blocks(vec![Block::genesis()])
    }

    // Safe constructor
    pub fn from_vec(vec: Vec<Block>) -> Result<Blocks, NextBlockErr> {
        let blocks = Blocks(vec);
        blocks.validate()?;
        Ok(blocks)
    }

    // Destructor
    pub fn to_vec(self) -> Vec<Block> {
        self.0
    }

    pub fn validate(&self) -> Result<(), NextBlockErr> {
        let mut curr: &Block = self.0.first().ok_or(NextBlockErr::NoBlocks)?;
        curr.validate()?;
        for i in 0..self.0.len() - 1 {
            let next = self.0.get(i + 1).unwrap();
            next.validate()?;
            next.validate_parent(curr)?;
            curr = next;
        }
        Ok(())
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        let new_block = Block::mine_block(self.last(), data);
        self.0.push(new_block)
    }

    // Safe first
    pub fn first(&self) -> &Block {
        self.0.first().expect("Blocks should always be non-empty")
    }

    // Safe last
    pub fn last(&self) -> &Block {
        self.0.last().expect("Blocks should always be non-empty")
    }

    pub fn get(&self, idx: usize) -> Option<&Block> {
        self.0.get(idx)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    // Safe push to tail
    pub fn push_back(&mut self, new_block: Block) -> Result<(), NextBlockErr> {
        new_block.validate()?;
        new_block.validate_parent(self.last())?;
        self.0.push(new_block);
        Ok(())
    }

    // Safe push to head
    pub fn push_front(&mut self, new_block: Block) -> Result<(), NextBlockErr> {
        new_block.validate()?;
        self.first().validate_parent(&new_block)?;
        self.0.insert(0, new_block);
        Ok(())
    }

    // Safe append between two valid subchains
    pub fn append(&mut self, mut suffix: Blocks) -> Result<(), NextBlockErr> {
        suffix.validate()?;
        suffix.first().validate_parent(self.last())?;
        self.0.append(&mut suffix.0);
        Ok(())
    }

    // Split off that ensures the resulting Self is always non-empty, by requiring that len > 0;
    // Does and returns nothing if len > Self.len().
    pub fn split_off(&mut self, len: usize) -> Option<Blocks> {
        if len > 0 {
            let suffix: Vec<Block> = self.0.split_off(std::cmp::min(self.len(), len));
            if suffix.len() > 0 {
                Some(Blocks(suffix))
            } else {
                None
            }
        } else {
            panic!(
                "Blocks::splitoff called with unsafe len {} for a vector of length {}",
                len,
                self.len()
            );
        }
    }

    // Splitoff_until that ensures the resulting Self is always non-empty by keeping inside it the block for the property holds;
    // Does and returns nothing if not able to find a block satisfying the property.
    pub fn split_off_until<P>(&mut self, prop: P) -> Option<Blocks>
    where
        P: Fn(&Block) -> bool,
    {
        if let Some(idx) = self.0.iter().position(|block| prop(&block)) {
            self.split_off(idx + 1)
        } else {
            None
        }
    }

    pub fn find<'a, P>(&'a self, prop: &P) -> Option<&'a Block>
    where
        P: Fn(&Block) -> bool,
    {
        self.0.iter().find(|block| prop(block))
    }

    pub fn iter(&self) -> std::slice::Iter<Block> {
        self.0.iter()
    }
}

impl std::fmt::Display for Blocks {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_, block) in self.iter().enumerate() {
            writeln!(f, "{}", block)?;
        }
        Ok(())
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
    },
}

impl std::fmt::Display for NextBlockResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockResult::ExtendedMain { end_idx, end_hash } => {
                write!(
                    f,
                    "Extended the main chain.\n\
                           \tIts endpoint is ({}, {})",
                    end_idx,
                    abbrev(end_hash)
                )
            }
            NextBlockResult::ExtendedFork {
                fork_idx,
                fork_hash,
                end_idx,
                end_hash,
            } => {
                write!(
                    f,
                    "Extended an existing fork from the main chain.\n\
                            \tIts forkpoint is ({}, {}) and endpoint is ({}, {}).",
                    fork_idx,
                    abbrev(fork_hash),
                    end_idx,
                    abbrev(end_hash)
                )
            }
            NextBlockResult::NewFork {
                fork_idx,
                fork_hash,
                end_idx,
                end_hash,
            } => {
                match end_idx - fork_idx {
                    1 => writeln!(f, "Added a single-block fork from the main chain."),
                    _ => writeln!(
                        f,
                        "Added a new fork that branches off an existing fork from the main chain."
                    ),
                }?;
                write!(
                    f,
                    "\tIts forkpoint is ({}, {}) and endpoint is ({}, {}).",
                    fork_idx,
                    abbrev(fork_hash),
                    end_idx,
                    abbrev(end_hash)
                )
            }
        }
    }
}

// For validating whether one block is a valid next block for another.
#[derive(Debug, Clone)]
pub enum NextBlockErr {
    DifficultyCheckFailed {
        idx: usize,
        hash: String,
        difficulty_prefix: String,
    },
    InconsistentHash {
        idx: usize,
        hash: String,
        computed_hash: String,
    },
    InvalidIndex {
        idx: usize,
        expected_idx: usize,
    },
    InvalidChild {
        idx: usize,
        prev_hash: String,
        parent_idx: usize,
        parent_hash: String,
    }, // Block has an inconsistent prev_hash and/or index with a specified parent
    UnrelatedGenesis {
        genesis_hash: String,
    }, // Block belongs to a chain with a different genesis root
    MissingParent {
        parent_idx: usize,
        parent_hash: String,
    }, // Block is missing a parent that connects it to the main chain or forks
    StrayParent {
        idx: usize,
        hash: String,
    }, // Block represents a missing parent that doesn't prepend to any orphaned branches,
    Duplicate {
        idx: usize,
        hash: String,
    }, // Block exists in the main chain, forks, or orphans
    NoBlocks, // Block used in a context with an empty chain or fork
}

impl std::fmt::Display for NextBlockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockErr::DifficultyCheckFailed {
                idx,
                hash,
                difficulty_prefix,
            } => {
                write!(
                    f,
                    "Block {}'s hash binary {} does not meet the difficulty target {}.",
                    idx,
                    BinaryString::from_hex(&hash).expect("can convert hex string to binary"),
                    difficulty_prefix
                )
            }
            NextBlockErr::InconsistentHash {
                idx,
                hash,
                computed_hash,
            } => {
                write!(
                    f,
                    "Block {}'s stored hash {} does not match its computed hash {}.",
                    idx,
                    abbrev(hash),
                    abbrev(computed_hash)
                )
            }
            NextBlockErr::InvalidIndex { idx, expected_idx } => {
                write!(
                    f,
                    "Block {} has invalid index, whereas we expected index {}.",
                    idx, expected_idx
                )
            }
            NextBlockErr::InvalidChild {
                idx,
                prev_hash,
                parent_idx,
                parent_hash,
            } => {
                write!(
                    f,
                    "Block {} with prev_hash {} should not be a child of Block {} with hash {}.",
                    idx,
                    abbrev(prev_hash),
                    parent_idx,
                    abbrev(parent_hash)
                )
            }
            NextBlockErr::MissingParent {
                parent_idx,
                parent_hash,
            } => {
                write!(
                    f,
                    "Block {} is missing its parent {} with hash {} in the main chain or forks.",
                    parent_idx + 1,
                    parent_idx,
                    abbrev(parent_hash)
                )
            }
            NextBlockErr::Duplicate { idx, hash } => {
                write!(f, "Block {} with hash {} is a duplicate already stored in the main chain, forks, or orphans."
                , idx, abbrev(hash))
            }
            NextBlockErr::UnrelatedGenesis { genesis_hash } => {
                write!(
                    f,
                    "Block belongs to a chain with a different genesis, {}.",
                    abbrev(genesis_hash)
                )
            }
            NextBlockErr::NoBlocks => {
                write!(f, "Encountered an empty chain or fork.")
            }
            NextBlockErr::StrayParent { idx, hash } => {
                write!(f, "Block {} with hash {} represents an out-of-sync missing parent, already handled or that we have no use for."
                , idx, abbrev(hash))
            }
        }
    }
}

impl std::error::Error for NextBlockErr {}
