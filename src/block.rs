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
use to_binary::{BinaryString};

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
    let last_block: &Block = self.get_last_block();
    let new_block: Block = Block::mine_block(last_block.idx + 1, data, &last_block.hash);
    self.try_push_block(&new_block).expect("returned mined block isn't valid")
  }

  // Try to append an arbitrary block
  pub fn try_push_block(&mut self, new_block: &Block) -> Result<(), &str>{
    let last_block: &Block = self.get_last_block();
    if Block::valid_block(last_block, &new_block) {
        info!("try_push_block(): added new block");
        self.0.push(new_block.clone());
        Ok (())
    } else {
        let e ="try_push_block(): could not add new_block - invalid";
        info!("{}", e);
        Err (e)
    }
  }

  pub fn get_last_block(&self) -> &Block {
    self.0.last().expect("Chain must be non-empty")
  }

  // Validate entire chain (ignoring the genesis block)
  pub fn valid_chain(chain: &Chain) -> bool {
    for i in 1..chain.0.len() {
      let err: String = format!("Block idx not found: {}", &((i-1).to_string()));
      let prev: &Block = chain.0.get(i - 1).expect(&err);
      let err: String = format!("Block idx not found: {}", &((i).to_string()));
      let curr: &Block = chain.0.get(i).expect(&err);
      if !Block::valid_block(prev, curr){
        return false
      }
    }
    true
  }

  // Choose the longest valid chain (defaulting to the local version). Returns true if chain was updated.
  pub fn sync_chain(&mut self, remote: &Chain) -> bool {
      match(Chain::valid_chain(&self), Chain::valid_chain(&remote))  {
        (true, true) => {
          if self.0.len() >= remote.0.len() {
            false
          } else {
            *self = remote.clone();
            true
          }
        },
        (false, true) => false,
        (true, false) => {*self = remote.clone(); true},
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
  // position in the chain for sequentiality and quick access
  pub idx: u64,
  // core content e.g. a list of transactions
  pub data: String,
  // cryptographic hash of the block (contents + metadata) that uniquely identifies it and ensures integrity
  pub hash: String,

  // header
  // records when block was created
  pub timestamp: i64,
  // reference to the previous block's hash to ensure chain integrity
  pub prev_hash: String,
  // random value controlled by block creator, used in PoW algorithm to find a valid block hash
  pub nonce: u64,
}


impl Block {
  // Find a valid nonce and hash to construct a new block
  pub fn mine_block(idx: u64, data: &str, prev_hash: &String) -> Block {
      let now: DateTime<Utc> = Utc::now();
      info!("mining block for:\n
              Block {{ idx: {}, data: {}, hash: ?, timestamp: {}, prev_hash: {}, nonce: ? }}"
            , idx, data, now, prev_hash);

      let mut nonce: u64 = 0;
      loop {
          let hash: String
            = Self::compute_hash(idx, data, now.timestamp(), &prev_hash, nonce);
          let BinaryString(binary_repr)
            = BinaryString::from_hex(&hash).expect("Can convert hex string to binary");

          if binary_repr.starts_with(DIFFICULTY_PREFIX) {
              info!(
                  "mine_block(): mined! \n nonce: {}, hash (bytes repr): {:?},  hash (hex repr): {},  hash (binary repr): {}"
                  , nonce, hash, hex::encode(&hash), binary_repr
              );
              return Self { idx, data : data.to_string(), hash, timestamp: now.timestamp(), prev_hash: prev_hash.clone(), nonce  }
          }
          nonce += 1;
      }
  }

  // Genesis block, the very first block in a chain which never references any previous blocks.
  pub fn genesis() -> Block {
    let mut genesis =  Block {
        idx: 0,
        data: String::from("genesis"),
        hash: bytes_to_hexstr(&ZERO_U32),
        timestamp: Utc::now().timestamp(),
        prev_hash: bytes_to_hexstr(&ZERO_U32),
        nonce: 0 ,
    };
    genesis.hash = Self::hash_block(&genesis);
    genesis
  }

  // Compute the hex-string of a sha256 hash (i.e. a 32-byte array) of a block
  pub fn hash_block (block : &Block)  -> String {
    Self::compute_hash(block.idx, block.data.as_str(), block.timestamp, &block.prev_hash, block.nonce)
  }

  fn compute_hash (idx: u64, data: &str, timestamp: i64, prev_hash: &String,  nonce: u64)  -> String {
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
      .finalize() // : Sha256 -> GenericArray<u8, U32>
      .into(); //  .into() : GenericArray<u8, U32> -> [u8; 32].

    bytes_to_hexstr(&hash)
  }

  // Validate a block */
  pub fn valid_block(last_block: &Block, block: &Block) -> bool {
    // * standard correctness checks:
    //    - check if block's header correctly stores the previous block's hash
    if block.prev_hash != last_block.hash {
      info!("valid_block(): block with idx: {} has wrong previous hash"
            , block.idx);
      return false
    }
    //    - check if block's idx is the increment of the previous block's idx
    if block.idx != last_block.idx + 1 {
      info!("valid_block(): block with idx {} is not the next block after the last one with idx {}"
            , block.idx, last_block.idx);
      return false
    }
    //    - check if block's hash is indeed the correct hash of itself.
    if block.hash != Block::hash_block(&block) {
      info!("valid_block(): block with idx {} has hash {}, which is different from its real hash binary {}"
           , last_block.idx, block.hash, Block::hash_block(&block)) ;
      return false
    }
    // * proof-of-work check:
    //    - check if block's (binary formatted) hash has a valid number of leading zeros
    let BinaryString(hash_binary)
      = BinaryString::from_hex(&block.hash).expect("Can convert hex string to binary");
    if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
      info!("valid_block(): block with idx {} has hash binary {}, which does need meet the difficulty target {}"
           , last_block.idx, hash_binary, DIFFICULTY_PREFIX) ;
      return false
    }
    info!("valid_block(): block is indeed valid!");
    true
  }
}

impl std::fmt::Display for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {

    write!(f, "Block {{\n\t idx: {}, \n\t data: {}, \n\t hash (hex): {}}}"
          , self.idx, self.data, self.hash)
  }
}

pub fn bytes_to_hexstr(hash: &[u8]) -> String {
  hex::encode(&hash)
}