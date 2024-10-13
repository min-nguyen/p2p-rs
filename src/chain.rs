// /////////////////
//
// PoW Implementation
//
// Simple modelling of the PoW consensus mechanism
//    Every node in the network can add a block, storing data as a string, to the blockchain ledger by mining a valid block locally and then broadcasting that block. As long as it’s a valid block, each node will add the block to its chain and our piece of data become part of a decentralized network.
//
/////////////////

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use to_binary::BinaryString;

use super::cryptutil;

// number of leading zeros required for the hashed block for the block to be valid.
const DIFFICULTY_PREFIX: &str = "00";

/* Chain */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain (pub Vec<Block>);

// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    IsSuffix,                     // chain doesn't start from idx 0
    SubChainErr(NextBlockErr)     // error between two contiguous blocks in the chain
}

// For validating whether one block is a valid next block for another.
#[derive(Debug)]
pub enum NextBlockErr {
    InvalidBlock(BlockErr),      // error from block's self-validation
    BlockTooOld {
        block_idx: usize,
        current_idx: usize
    },                           // block is out-of-date
    DuplicateBlock {
        block_idx: usize
    },                           // competing block is a duplicate
    CompetingBlock {
        block_idx: usize,
        block_parent_hash: String
    },                           // competing block has same parent
    CompetingBlockInFork {
        block_idx: usize,
        block_parent_hash: String,
        current_parent_hash: String
    },                           // competing block has different parent, belonging to a fork (or different chain)
    NextBlockInFork {
        block_idx: usize,
        block_parent_hash: String,
        current_hash: String
    },                           // next block's parent doesn't match the current block, belonging to a fork (or different chain)
    BlockTooNew {
        block_idx: usize,
        current_idx: usize
    },                           // block is ahead by more than 1 block
    UnknownError,                // non-exhaustive case (should not happen)
}

#[derive(Debug)]
pub enum ForkSuffixErr {
    ForkSuffixStartsAtGenesis (
    ),                          // fork suffix's first block has index == 0
    ForkSuffixIncomplete {
        fork_head_idx : usize,
    },                          // fork suffix's first block's parent hash doesn't match any block in the current chain
    ForkSuffixInvalidSubChain (
        NextBlockErr
    ),                          // fork suffix doesn't form a valid chain
}

impl Chain {
    // New chain with a single genesis block
    pub fn new() -> Self {
        Self (vec![Block::genesis()])
    }

    pub fn get_current_block(&self) -> &Block {
        self.0.last().expect("chain must be non-empty")
    }

    pub fn get_block_by_idx(&self, idx: usize) -> Option<&Block> {
        self.0.get(idx)
    }

    pub fn get_block_by_hash(&self, hash: &String) -> Option<&Block> {
        self.0.iter().find(|b: &&Block| b.hash == *hash)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    // Mine a new valid block from given data, and extend the chain by it
    pub fn make_new_valid_block(&mut self, data: &str) {
        let current_block: &Block = self.get_current_block();
        let new_block: Block = Block::mine_block(current_block.idx + 1, data, &current_block.hash);
        self.try_push_block(&new_block).expect("returned mined block isn't valid")
    }

    // Try to append an arbitrary block
    pub fn try_push_block(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
        let current_block: &Block = self.get_current_block();
        Self::validate_next_block(current_block, &new_block)?;
        self.0.push(new_block.clone());
        Ok(())
    }

    // Try to attach a fork (suffix of a full chain) to extend any compatible parent block in the current chain
    // Note: Can succeed even if resulting in a shorter chian.
    pub fn try_merge_fork(&mut self, fork_suffix: &mut Chain) -> Result<(), ForkSuffixErr>{
        if let Err(e) = Self::validate_subchain(&fork_suffix) {
            return Err(ForkSuffixErr::ForkSuffixInvalidSubChain(e))
        }

        let fork_suffix_head = fork_suffix.get_block_by_idx(0).expect("forked chain must be non-empty");
        if fork_suffix_head.idx == 0 {
            return Err(ForkSuffixErr::ForkSuffixStartsAtGenesis())
        }
        match self.get_block_by_hash(&fork_suffix_head.prev_hash) {
            Some(forkpoint) => {
                // if forkpoint starts at idx n, then keep the first n + 1 blocks
                self.0.truncate(forkpoint.idx + 1);
                self.0.append(&mut fork_suffix.0);
                Ok(())
            }
            // fork suffix's first block doesn't reference a block in the current chain.
            None => {
                Err(ForkSuffixErr::ForkSuffixIncomplete {fork_head_idx: fork_suffix_head.idx})
            }
        }
    }

    // validate entire chain from tail to head, ignoring the first block
    pub fn validate_chain(chain: &Chain) -> Result<(), ChainErr> {
        if chain.0.get(0).expect("chain must be non-empty").idx != 0 {
            return Err(ChainErr::IsSuffix)
        }
        Ok(())
    }

    // validate entire chain from tail to head, ignoring the first block
    pub fn validate_subchain(chain: &Chain) -> Result<(), NextBlockErr> {
        for i in 0..chain.0.len() - 1 {
            let curr: &Block = chain.0.get(i)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            let next: &Block = chain.0.get(i + 1)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            if let Err(e) = Self::validate_next_block(curr, next) {
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn validate_next_block(current_block: &Block, block: &Block) -> Result<(), NextBlockErr> {
        // * check validity of block by itself
        if let Err(e) = Block::validate_block(block) {
            return Err(NextBlockErr::InvalidBlock(e));
        }

        // * check validity of block with respect to our chain
        //    1. if the block is out-of-date with our chain
        if block.idx < current_block.idx {
            return Err(NextBlockErr::BlockTooOld {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }
        //    2. if the block is up-to-date (i.e. competes) with our chain
        if block.idx == current_block.idx {
            //   a. competing block is a duplicate of ours
            if block.hash == current_block.hash {
                return Err(NextBlockErr::DuplicateBlock {
                    block_idx: block.idx,
                });
            }
            //   b. competing block is different and has the same parent
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 1
            if block.prev_hash == current_block.prev_hash {
                return Err(NextBlockErr::CompetingBlock {
                    block_idx: block.idx,
                    block_parent_hash: block.prev_hash.clone(),
                });
            }
            //   c. competing block is different and has a different parent, indicating their chain has possibly forked
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 2
            if block.prev_hash != current_block.prev_hash {
                return Err(NextBlockErr::CompetingBlockInFork {
                    block_idx: block.idx,
                    block_parent_hash: block.prev_hash.clone(),
                    current_parent_hash: current_block.prev_hash.clone(),
                });
            }
        }
        //   3. if the block is ahead-of-date of our chain by exactly 1 block
        if block.idx == current_block.idx + 1 {
            //  a. next block's parent does not match our current block.
            if block.prev_hash != current_block.hash {
                // hence, we need to either:
                //    i)  request an entirely new up-to-date chain (inefficient but simple)
                //    ii) back-track and recursively request all its ancestors until getting one that we can find in our chain -- if at all
                return Err(NextBlockErr::NextBlockInFork {
                    block_idx: block.idx,
                    block_parent_hash: block.prev_hash.clone(),
                    current_hash: current_block.hash.clone(),
                });
            } else {
                // we can safely extend the chain
                return Ok(());
            }
        }
        //    4. if the block is ahead-of-date of our chain by more than 1 block
        if block.idx > current_block.idx + 1 {
            // hence, we need to either:
            //    i)  request an entirely new up-to-date chain (inefficient but simple)
            //    ii) back-track and recursively request all its ancestors until getting one that we can find in our chain -- if at all
            return Err(NextBlockErr::BlockTooNew {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }

        Err(NextBlockErr::UnknownError)
    }

    // Choose the longest valid chain (defaulting to the local version). Returns true if chain was updated.
    pub fn sync_chain(&mut self, remote: &Chain) -> bool {
        match(Self::validate_chain(&self), Self::validate_chain(&remote))  {
            (Ok(()), Ok(())) => {
            if self.0.len() >= remote.0.len() {
                false
            } else {
                *self = remote.clone();
                true
            }
            },
            (Err(_), Ok(())) => false,
            (Ok(()), Err(_)) => {*self = remote.clone(); true},
            _ => panic!("local and remote chains both invalid")
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Chain {{\n")?;
        for block in &self.0 {
            write!(f, "\t{}\n", block)?
        };
        write!(f, "}}")
    }
}

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

  // Genesis block, the very first block in a chain which never references any previous blocks.
  pub fn genesis() -> Block {
    let (idx, data, timestamp, prev_hash, nonce)
        = (0, "genesis".to_string(), Utc::now().timestamp(), cryptutil::encode_bytes_to_hex(&cryptutil::ZERO_U32), 0);
    let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
    Block { idx, data, timestamp, prev_hash, nonce, hash }
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
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Block {{\n\t idx: {}, \n\t data: {}, \n\t hash: {}, \n\t prev_hash: {}, \n\t timestamp: {}, \n\t nonce: {}}}"
                , self.idx, self.data, self.hash, self.prev_hash, DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp"), self.nonce)
    }
}


/********  TESTS **********/

#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{chain::{BlockErr, ChainErr, ForkSuffixErr, NextBlockErr}, cryptutil::{debug, encode_bytes_to_hex, ZERO_U32}, Block, Chain};

    /* tests for a block by itself  */
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
    /* tests for extending a chain by a new proposed block */
    const CHAIN_LEN : usize = 5;
    const FORK_PREFIX_LEN : usize = 3;

    #[test]
    fn test_out_of_date_block() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }
        let out_of_date_block: Block = chain.get_block_by_idx(chain.get_current_block().idx - 1).unwrap().clone();

        assert!(matches!(
            debug(chain.try_push_block(&out_of_date_block)),
            Err(NextBlockErr::BlockTooOld { .. })
        ));
    }
    #[test]
    fn test_duplicate_block() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }
        let duplicate_block: Block = chain.get_current_block().clone();

        assert!(matches!(
            debug(chain.try_push_block(&duplicate_block)),
            Err(NextBlockErr::DuplicateBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }
        // mine an alternative block at the same position as the current one
        let competing_block: Block = Block::mine_block(
            chain.get_current_block().idx,
            "competing block at {}",
            &chain.get_current_block().prev_hash);

        assert!(matches!(
            debug(chain.try_push_block(&competing_block)),
            Err(NextBlockErr::CompetingBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block_in_fork() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut fork = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        // make competing fork the same length as the current chain
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            fork.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(fork.get_current_block())),
            Err(NextBlockErr::CompetingBlockInFork { .. })
        ));
    }
    #[test]
    fn test_next_block_in_fork() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut fork = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        // make competing fork one block longer than the current chain
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 1 {
            fork.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(fork.get_current_block())),
            Err(NextBlockErr::NextBlockInFork { .. })
        ));
    }
    #[test]
    fn test_too_ahead_of_date_block() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // create a matching chain that is 2 blocks longer than the current chain
        let mut dup_chain: Chain = chain.clone();
        dup_chain.make_new_valid_block("next block in dup chain");
        dup_chain.make_new_valid_block("too-far-ahead block in dup chain");

        assert!(matches!(
            debug(chain.try_push_block(dup_chain.get_current_block())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }
    #[test]
    fn test_too_ahead_of_date_block_in_fork() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut fork = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        // make competing fork 2 blocks longer than the current chain
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            fork.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(fork.get_current_block())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }
    #[test]
    fn test_valid_next_block() {
        let mut chain: Chain = Chain::new();
        let current_block: Block = chain.get_current_block().clone();

        chain.make_new_valid_block(&format!("test next valid block"));
        let next_valid_block = chain.get_current_block();

        assert!(matches!(
            debug(Chain::validate_next_block(&current_block, &next_valid_block))
            , Ok(())));
    }
    #[test]
    fn test_valid_chain() {
        let mut chain: Chain = Chain::new();
        for i in 1 .. CHAIN_LEN {
          chain.make_new_valid_block(&format!("next valid block {}", i));
        }
        assert!(matches!(
            debug(Chain::validate_chain(&chain)),
            Ok(())));
    }
    #[test]
    fn test_valid_fork() {
        let mut chain: Chain = Chain::new();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut fork = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        // make competing fork 2 blocks longer than the current chain
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            fork.make_new_valid_block(&format!("block {} in fork", i));
        }

        // strip the common prefix between the current chain and the forked chain
        let mut fork_suffix = {
            fork.0.drain(0 ..FORK_PREFIX_LEN);
            fork
        };

        println!("Chain : {}\n", chain);
        println!("Fork suffix : {}\n", fork_suffix);
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork_suffix)),
            Ok(())
        ));
        println!("Merged chain and fork suffix : {}", chain);
    }
}
