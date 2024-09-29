/////////////////
//
// PoW Implementation [https://blog.logrocket.com/how-to-build-a-blockchain-in-rust/]
//
//
/////////////////

use chrono::{DateTime, Utc};
//    Our design is a simple modelling of the PoW consensus mechanism for cryptocurrency transactions.
//    Every node in the network can add a block, storing transactions as a string, to the blockchain ledger by mining a valid block locally and then broadcasting that block. As long as it’s a valid block, each node will add the block to its chain and our piece of data become part of a decentralized network.
  // This does not touch on mining, consensus, etc.
use log::{error, info};
use serde::{Deserialize, Serialize};

// difficult prefix, the number of leading zeros required for the hashed block for the block to be valid.
//   Varies each new block according to the consensus alg. For simplicity, we hardcode it to two 0's.
const DIFFICULTY_PREFIX: &str = "00";

// A block records some or all of the most recent transactions not yet validated by the network.
// When a transaction occurs the details are packaged into a digital structure known as a block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
  // index, a unique identifier that represents the block's position in the chain.
  //   Ensures sequentiality in the chain + Allows quick access to blocks
  pub index: u64,
  // block body,
  //   Contains core content, e.g. a list of transactions
  pub data: String,
  // hash, a unique identifier that represesents the entire block (contents + metadata) as a cryptographic hash.
  //   Ensures the integrity of the block’s data
  pub hash: [u8; 32],
  // block header, metadata
  pub header: BlockHeader,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
  // timestamp, recording when the block was created.
  //   Used by smart contracts that depend on timestamps;
  //   and to determine how well the current average rate of block creation matches the target value
  pub timestamp: i64,
  // reference to the sha256-hash of the previous block,
  //   Implements the “chain” and ensures the chain's integrity.
  pub prev_hash: [u8; 32],
  // nonce,  random value controlled by the block creator.
  //   Used in the Proof of Work consensus algorithm to change the hash of the block header.
  //   Only a block with a header value less than a certain threshold is considered valid
  pub nonce: u64,
  // transaction root, summarizes the contents of the block’s body.
  //   Helps to ensure that the transactions that the block contains benefit from the same integrity protections as the block header.
  //   If a blockchain only allows one transaction per block, the concept of a transaction root might be less critical.
}

impl Block {
  pub fn new(index: u64, data: String, prev_hash: [u8;32]) -> Self {
      let now: DateTime<Utc> = Utc::now();
      let (nonce, hash) = Self::mine_block(index, &data, now, prev_hash);
      Self {
          index,
          data,
          hash,
          header : BlockHeader {
            timestamp: now.timestamp(),
            prev_hash,
            nonce
          }
      }
  }

  // Find a valid nonce and hash for a new block
  fn mine_block(index: u64, data: &str, now : DateTime<Utc>, prev_hash: [u8; 32]) ->  (u64, [u8; 32]) {
      info!("mining block... for:\n Block {{ index: {}, data: {}, hash: ?, timestamp: {}, prev_hash: {}, nonce: ? }}"
            , index, data, now, bytes_to_hexstr(&prev_hash));
      let mut nonce: u64 = 0;

      loop {
          let hash: [u8; 32] = Self::compute_hash(index, data, now.timestamp(), prev_hash, nonce);
          let binary_repr = bytes_to_binarystr(&hash);
          if binary_repr.starts_with(DIFFICULTY_PREFIX) {
              info!(
                  "mined! nonce: {}, hash (bytes repr): {:?},  hash (hex repr): {},  hash (binary repr): {}",
                  nonce,
                  hash,
                  hex::encode(&hash),
                  binary_repr
              );
              return (nonce, hash)
          }
          nonce += 1;
      }
  }

  // Genesis block, the very first block in a chain which never references any previous blocks. */
  pub fn genesis() -> Block{
    let mut genesis =  Block {
        header: BlockHeader {
                  prev_hash: [0u8; 32],
                  timestamp: chrono::Utc::now().timestamp(),
                  nonce: 0 },
        index: 0,
        data: String::from("genesis"),
        hash: [0u8; 32],
    };
    genesis.hash = Self::compute_block_hash(&genesis);
    genesis
  }

  // Compute the sha256 hash of a block, i.e. a 32-byte array */
  pub fn compute_block_hash (block : &Block)  -> [u8; 32] {
    Self::compute_hash(block.index, block.data.as_str(), block.header.timestamp, block.header.prev_hash, block.header.nonce)
  }

  pub fn compute_hash (index: u64, data: &str, timestamp: i64, prev_hash: [u8;32],  nonce: u64)  -> [u8; 32] {
    use sha2::{Sha256, Digest};

    // create a sha256 hasher instance
    let mut hasher: Sha256 = Sha256::new();
    // represent the block as a byte array &[u8], used as input data to the haster
    let block_json: serde_json::Value = serde_json::json!({
        "header": serde_json::json!({
          "prev_hash": prev_hash,
          "timestamp": timestamp,
          "nonce":nonce
        }),
        "index": index,
        "data": data
    });
    hasher.update(block_json.to_string().as_bytes());

    // produce a sha256 hash of the block header as a [u8; 32]
    hasher
      //  .finalize() : Sha256 -> GenericArray<u8, U32> which is a wrapper around a [u8; 32] array
      .finalize()
      //  .into() : GenericArray<u8, U32> -> [u8; 32].
      .into()
  }

  // Validate a block */
  fn valid_block(last_block: &Block, block: &Block) -> bool {
    // * standard correctness checks:
    // - check if block's header correctly stores the previous block's hash
    if block.header.prev_hash != last_block.hash {
      info!("block with index: {} has wrong previous hash", block.index);
      return false
    }
    // - check if block's index is the increment of the previous block's index
    if block.index != last_block.index + 1 {
      info!("block with index {} is not the next block after the last one with index {}", block.index, last_block.index);
      return false
    }
    let hash_binary: String = bytes_to_binarystr(block.hash.as_slice());
    // check if block's hash is indeed the correct hash of itself.
    if block.hash != Self::compute_block_hash(&block) {
      info!("block with index {} has hash binary {}, which is different from its real hash binary {}"
           , last_block.index, hash_binary, bytes_to_binarystr(Self::compute_block_hash(block).as_slice())) ;
      return false
    }

    // * proof-of-work check:
    // - check if block's (binary formatted) hash has a valid number of leading zeros
    if !hash_binary.starts_with(DIFFICULTY_PREFIX) {
      info!("block with index {} has hash binary {}, which does need meet the difficulty target {}"
           , last_block.index, hash_binary, DIFFICULTY_PREFIX) ;
      return false
    }
    info!("valid_block(): block is indeed valid!");
    true
  }
}

impl std::fmt::Display for Block {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "Block {{\n data: {}, \n hash ([u8; 32]): {:?},\n hash (hex): {} \n }}"
          , self.data, self.hash, hex::encode(self.hash))
  }
}

// Application state

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain { pub blocks : Vec<Block> }

impl Chain {
  // New chain with a single genesis block
  pub fn new() -> Self {
    Self {blocks : vec![Block::genesis()]}
  }

  // Validate entire chain (ignoring the genesis block)
  fn valid_chain(chain: &Chain) -> bool {
    for i in 1..chain.blocks.len() {
      let err: String = format!("Block index not found: {}", &((i-1).to_string()));
      let prev: &Block = chain.blocks.get(i - 1).expect(&err);
      let err: String = format!("Block index not found: {}", &((i).to_string()));
      let curr: &Block = chain.blocks.get(i).expect(&err);
      if !Block::valid_block(prev, curr){
        return false
      }
    }
    true
  }

  // Retain the longest valid chain (defaulting to the local version)
  pub fn choose_chain(local: Chain, remote: Chain) -> Chain {
      match(Chain::valid_chain(&local), Chain::valid_chain(&remote))  {
        (true, true) => {
          if local.blocks.len() >= remote.blocks.len() {
            local
          } else {
            remote
          }
        },
        (false, true) => local,
        (true, false) => remote,
        _ => panic!("local and remote chains both invalid")
      }
  }

  // Append new block
  pub fn try_push_block(&mut self, new_block: Block){
    let last_block: &Block = self.blocks.last().expect("Chain must be non-empty");
    if Block::valid_block(last_block, &new_block) {
        self.blocks.push(new_block);
    } else {
        error!("could not add new_block - invalid");
    }
  }
}

impl std::fmt::Display for Chain {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "Chain {{\n")?;
    for block in &self.blocks {
      write!(f, "{}\n", block)?
    };
    write!(f, "\n}}\n")
  }
}

pub fn bytes_to_binarystr(hash: &[u8]) -> String {
  let mut binary: String = String::default();
  for c in hash {
    binary.push_str(&format!("{:b}", c));
  }
  binary
}

pub fn bytes_to_hexstr(hash: &[u8]) -> String {
  hex::encode(&hash)
}
