// /////////////////
//
// PoW Implementation
//
// Simple modelling of the PoW consensus mechanism
//    Every node in the network can add a block, storing data as a string, to the blockchain ledger by mining a valid block locally and then broadcasting that block. As long as it’s a valid block, each node will add the block to its chain and our piece of data become part of a decentralized network.
//
//
/////////////////

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use to_binary::BinaryString;

use crate::util;


// number of leading zeros required for the hashed block for the block to be valid.
const DIFFICULTY_PREFIX: &str = "0";

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

#[derive(Debug)]
pub enum NewBlockErr {
    InvalidBlock(BlockErr),      // error from block's self-validation
    BlockTooOld {
        block_idx: u64,
        current_idx: u64
    },                           // block is out-of-date
    DuplicateBlock {
        block_idx: u64
    },                           // competing block is a duplicate
    CompetingBlock {
        block_idx: u64,
        parent_hash: String
    },                           // competing block has same parent
    CompetingFork {
        block_idx: u64,
        block_parent_hash: String,
        current_parent_hash: String
    },                           // competing block indicates a fork
    NextBlockInvalidParent {
        block_idx: u64,
        block_parent_hash: String,
        current_hash: String
    },                           // next block's parent doesn't match the current block
    BlockTooNew {
        block_idx: u64,
        current_idx: u64
    },                           // block is ahead by more than 1 block
    UnknownError,                // non-exhaustive case (should not happen)
}

/* Chain */
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain (pub Vec<Block>);

impl Chain {
    // New chain with a single genesis block
    pub fn new() -> Self {
        Self (vec![Block::genesis()])
    }

    // Mine a new valid block from given data
    pub fn make_new_valid_block(&mut self, data: &str) {
        let current_block: &Block = self.get_current_block();
        let new_block: Block = Block::mine_block(current_block.idx + 1, data, &current_block.hash);
        self.try_push_block(&new_block).expect("returned mined block isn't valid")
    }

    // Try to append an arbitrary block
    pub fn try_push_block(&mut self, new_block: &Block) -> Result<(), String>{
        let current_block: &Block = self.get_current_block();
        match Chain::validate_new_block(current_block, &new_block) {
            Err (e) => Err (format!("Couldn't push new_block: {:?}", e)),
            Ok (()) => {
                self.0.push(new_block.clone());
                Ok (())
            }
        }
    }

    pub fn get_current_block(&self) -> &Block {
        self.0.last().expect("Chain must be non-empty")
    }

    // validate entire chain from tail to head, ignoring the genesis block
    pub fn validate_chain(chain: &Chain) -> Result<(), NewBlockErr> {
        for i in 0..chain.0.len() - 1 {
            let curr: &Block = chain.0.get(i)
                .ok_or_else(|| NewBlockErr::UnknownError)?;
            let next: &Block = chain.0.get(i + 1)
                .ok_or_else(|| NewBlockErr::UnknownError)?;
            if let Err(e) = Chain::validate_new_block(curr, next) {
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn validate_new_block(current_block: &Block, block: &Block) -> Result<(), NewBlockErr> {
        // * check validity of block by itself
        if let Err(e) = Block::validate_block(block) {
            return Err(NewBlockErr::InvalidBlock(e));
        }

        // * check validity of block with respect to our chain
        //    1. if the block is out-of-date with our chain
        if block.idx < current_block.idx {
            return Err(NewBlockErr::BlockTooOld {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }
        //    2. if the block is up-to-date (i.e. competes) with our chain
        if block.idx == current_block.idx {
            //   a. competing block is a duplicate of ours
            if block.hash == current_block.hash {
                return Err(NewBlockErr::DuplicateBlock {
                    block_idx: block.idx,
                });
            }
            //   b. competing block is different and has the same parent
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 1
            if block.prev_hash == current_block.prev_hash {
                return Err(NewBlockErr::CompetingBlock {
                    block_idx: block.idx,
                    parent_hash: block.prev_hash.clone(),
                });
            }
            //   c. competing block is different and has a different parent, indicating their chain has possibly forked
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 2
            if block.prev_hash != current_block.prev_hash {
                return Err(NewBlockErr::CompetingFork {
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
                return Err(NewBlockErr::NextBlockInvalidParent {
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
            return Err(NewBlockErr::BlockTooNew {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }

        Err(NewBlockErr::UnknownError)
    }

    // Choose the longest valid chain (defaulting to the local version). Returns true if chain was updated.
    pub fn sync_chain(&mut self, remote: &Chain) -> bool {
        match(Chain::validate_chain(&self), Chain::validate_chain(&remote))  {
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
    pub idx: u64,
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
  // Find a valid nonce and hash to construct a new block
  pub fn mine_block(idx: u64, data: &str, prev_hash: &String) -> Block {
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
        = (0, "genesis".to_string(), Utc::now().timestamp(), util::encode_bytes_to_hex(&util::ZERO_U32), 0);
    let hash: String = Self::compute_hash(idx, &data, timestamp, &prev_hash, nonce);
    Block { idx, data, timestamp, prev_hash, nonce, hash }
  }

  // Compute the hex-string of a sha256 hash (i.e. a 32-byte array) of a block
  fn compute_hash (idx: u64, data: &str, timestamp: i64, prev_hash: &String, nonce: u64)  -> String {
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

        util::encode_bytes_to_hex(&hash)
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
            });
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

#[cfg(test)] // cargo test -- --nocapture
mod chain_tests {
    use crate::{{Block, Chain}, util::{encode_bytes_to_hex, ZERO_U32}};
    // mod chain;
    // use crate::chain::Block;

    /* low-level block tests */
    #[test]
    fn test_valid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      assert!(matches!(Chain::validate_new_block(&gen, &valid_block), Ok(())));
    }

    #[test]
    fn test_invalid_first_block() {
      let gen = Block::genesis();
      let valid_block = Block::mine_block(1, "test", &gen.hash);

      let invalid_idx = Block {idx : 0, .. valid_block.clone()};
      assert!(matches!(Chain::validate_new_block(&gen, &invalid_idx), Err(_)));

      let invalid_prev_hash = Block { prev_hash : encode_bytes_to_hex(ZERO_U32), .. valid_block.clone() };
      assert!(matches!(Chain::validate_new_block(&gen, &invalid_prev_hash), Err(_)));

      let invalid_hash = Block {hash : encode_bytes_to_hex(ZERO_U32), .. valid_block.clone()};
      assert!(matches!(Chain::validate_new_block(&gen, &invalid_hash), Err(_)));

      let invalid_difficulty_prefix = Block {hash :  hex::encode([1;32]), .. valid_block.clone()};
      assert!(matches!(Chain::validate_new_block(&gen, &invalid_difficulty_prefix), Err(_)));
    }

    /* high-level chain tests */
    #[test]
    fn test_extend_chain_once() {
      let mut chain: Chain = Chain::new();
      chain.make_new_valid_block("test");
      assert!(matches!(Chain::validate_chain(&chain), Ok(())));
    }

    #[test]
    fn test_extend_chain_many() {
      let mut chain: Chain = Chain::new();
      for _ in 0 .. 10 {
        chain.make_new_valid_block("test");
      }
      assert!(matches!(Chain::validate_chain(&chain), Ok(())));
    }

}
