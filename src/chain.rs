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

// number of leading zeros required for the hashed block for the block to be valid.
const DIFFICULTY_PREFIX: &str = "0";
// 32 byte (256-bit) array of zeros
pub const ZERO_U32 : [u8; 32] = [0; 32];

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
        match Block::validate_block(current_block, &new_block) {
            Err (e) => Err (format!("Couldn't push new_block: {}", e)),
            Ok (()) => {
                self.0.push(new_block.clone());
                Ok (())
            }
        }
    }

    pub fn get_current_block(&self) -> &Block {
        self.0.last().expect("Chain must be non-empty")
    }

    // Validate entire chain from tail to head, ignoring the genesis block
    pub fn validate_chain(chain: &Chain) -> Result<(), String> {
        for i in 1..chain.0.len() {
            let err: String = format!("Block idx not found: {}", &((i-1).to_string()));
            let prev: &Block = chain.0.get(i - 1).expect(&err);
            let err: String = format!("Block idx not found: {}", &((i).to_string()));
            let curr: &Block = chain.0.get(i).expect(&err);
            if let Err(e) = Block::validate_block(prev, curr){
                return Err (format!("Couldn't validate chain: {}", e))
            }
        }
        Ok (())
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
        = (0, "genesis".to_string(), Utc::now().timestamp(), bytes_to_hexstr(&ZERO_U32), 0);
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

        bytes_to_hexstr(&hash)
    }

  // Validate a block */
  pub fn validate_block(current_block: &Block, block: &Block) -> Result<(), String> {
    // * check validity of block as its own entity:
    //    1. check if block's hash (in binary) has a valid number of leading zeros
    let BinaryString(hash_binary)
      = BinaryString::from_hex(&block.hash).expect("Can convert hex string to binary");
    if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
        return Err(format!("Block fails difficulty check! Hash binary {} does need meet the difficulty target {}"
        , hash_binary, DIFFICULTY_PREFIX))
    }
    //    2. check if block's hash is indeed the correct hash of itself.
    let computed_hash = Self::compute_hash(block.idx, &block.data, block.timestamp, &block.prev_hash, block.nonce);
    if block.hash != computed_hash {
        return Err(format!("Block is corrupt! Stored hash {} is inconsistent with its computed hash {}"
        , block.hash, computed_hash))
    }

    // * check validity of block with respect to our chain
    //    1. if the block is out-of-date with our chain
    if block.idx < current_block.idx  {
        return Err(format!("Block too old! Block {} is less than the local chain length {}"
        , block.idx, current_block.idx))
    }
    //    2. if the block is up-to-date (i.e. competes) with our chain
    if block.idx == current_block.idx {
        //   a. competing block is a duplicate of ours
        if block.hash == current_block.hash {
            return Err(format!("Competing block {} is a duplicate of our current one"
            , block.idx))
        }
        //   b. competing block is different but has the same parent
        //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 1
        if block.prev_hash == current_block.prev_hash {
            return Err(format!("Competing block {}'s parent {} is the same as our current one block's parent"
            , block.idx, block.prev_hash))
        }
        //   c. competing block is different and also has a different parent, indicating their chain has possibly forked at a common ancestor
        //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 2
        if block.prev_hash != current_block.prev_hash {
            return Err(format!("Competing block {}'s parent {} is different to our current block's parent {}"
            , block.idx, block.prev_hash, current_block.prev_hash))
        }
    }
    //   3. if the block is ahead-of-date of our chain by exactly 1 block
    if block.idx == current_block.idx + 1 {
        //  a. next block's parent does not match our current block.
        if block.prev_hash != current_block.hash {
            // hence, we need to either:
            //    i)  request an entirely new up-to-date chain (inefficient but simple)
            //    ii) back-track and recursively request all its ancestors until getting one that we can find in our chain
            return Err(format!("Next block {}'s parent {} does not match our current block {}"
            , block.idx, block.prev_hash, current_block.hash))
        }
        //  b. next block's parent matches our current block
        else {
            // we can safely extend the chain
            return Ok(())
        }
    }
    //    4. if the block is ahead-of-date of our chain by more than 1 block
    if block.idx > current_block.idx + 1 {
        // hence, we need to either:
        //    i)  request an entirely new up-to-date chain (inefficient but simple)
        //    ii) back-track and recursively request all its ancestors until getting one that we can find in our chain
        return Err(format!("Block too new! Block Idx {} is too far ahead of local chain length {}"
        , block.idx, current_block.idx))
    };
    Err("ERROR! Non-exhaustive pattern matching. Should not happen.".to_string())
  }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Block {{\n\t idx: {}, \n\t data: {}, \n\t hash: {}, \n\t prev_hash: {}, \n\t timestamp: {}, \n\t nonce: {}}}"
                , self.idx, self.data, self.hash, self.prev_hash, DateTime::from_timestamp(self.timestamp, 0).expect("can convert timestamp"), self.nonce)
    }
}

pub fn bytes_to_hexstr(hash: &[u8]) -> String {
    hex::encode(&hash)
}