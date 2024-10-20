/*
    *Chain*:
    - Chain, a safe wrapper around a vector of blocks, and error types
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use libp2p::futures::stream::Next;
use serde::{Deserialize, Serialize};
use crate::cryptutil::pretty_hex;

use super::block::{Block::{self}, NextBlockErr};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main : Vec<Block>,
    // <fork point, <fork end hash, forked blocks>>
    pub forks: HashMap<String, HashMap<String, Vec<Block>>>,
}

fn find_block<'a>(blocks: &'a Vec<Block>, block_hash: &String) -> Option<&'a Block> {
    blocks.iter().find(|block| &block.hash == block_hash)
}

// Check if block is in any fork, returning the fork point, end hash, and fork
fn find_fork_mut<'a>(forks: &'a mut HashMap<String, HashMap<String, Vec<Block>>>, hash: &String)
    -> Option<(String, String, &'a mut Vec<Block>)> {
    // iterate through fork points
    for (fork_point, forks_from) in forks {
        // iterate through forks from the fork point
        for (end_hash, fork) in forks_from {
            // iterate through blocks in the fork
            if let Some(_) = find_block(fork, hash) {
                return Some((fork_point.clone(), end_hash.clone(), fork))
            }
        }
    }
    None
}

fn push_block(blocks: &mut Vec<Block>, new_block: &Block){
    blocks.push(new_block.clone());
}

fn truncate(blocks: &mut Vec<Block>, len: usize){
    blocks.truncate(std::cmp::min(blocks.len() - 1, len));
}

fn truncate_until<P>(blocks: &mut Vec<Block>, prop: P)
where
    P: Fn(&Block) -> bool,
{
    if let Some(idx) = blocks.iter().position(|block| prop(&block)){
        blocks.truncate(idx);
    }
}

impl Chain {
    // New chain with a single genesis block
    pub fn genesis() -> Self {
        Self { main : vec![Block::genesis()], forks : HashMap::new() }
    }

    // Safely construct a chain from a vector of blocks
    pub fn from_vec(blocks: Vec<Block>) -> Result<Chain, ChainErr> {
        let chain = Chain{main : blocks, forks : HashMap::new()};
        Self::validate_chain(&chain)?;
        Ok(chain)
    }

    pub fn to_vec(&self) -> Vec<Block> {
        self.main.clone()
    }

    pub fn get(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
    }

    pub fn lookup(&self, hash: &String) -> Option<&Block> {
        self.main.iter().find(|b: &&Block| b.hash == *hash)
    }

    pub fn last(&self) -> &Block {
        self.main.last().expect("Chain should always be non-empty")
    }

    pub fn len(&self) -> usize {
        self.main.len()
    }

    pub fn truncate(&mut self, len: usize){
        truncate(&mut self.main, len);
    }

    pub fn handle_new_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr>{
        Block::validate_block(&block)?;

        let endpoint_idx = block.idx;
        let endpoint_hash = block.hash.clone();
        // Search for the parent block in the main chain.
        if let Some(parent_block) = find_block(&self.main, &block.prev_hash){

            Block::validate_child(parent_block, &block)?;
            println!("Found valid parent block in main chain.");

            // See if we can append the block to the main chain
            if self.last().hash == parent_block.hash {
               push_block(&mut self.main, &block);
               Ok(NextBlockResult::ExtendedMain {
                    length: self.len(),
                    endpoint_idx,
                    endpoint_hash
                })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let new_fork = vec![block.clone()];
                let forks_from: &mut HashMap<String, Vec<Block>>
                    = self.forks.entry(block.prev_hash.to_string()).or_insert(HashMap::new());
                forks_from.insert(block.hash.to_string() // end hash
                                , new_fork);
                println!("Adding a single-block fork to the main chain");
                Ok(NextBlockResult::NewFork {
                    length: 1,
                    forkpoint_idx: block.idx - 1,
                    forkpoint_hash: block.prev_hash,
                    endpoint_idx,
                    endpoint_hash
                })
            }
        }
        // Search for the parent block in the forks.
        else if let Some((  forkpoint_hash,
                            endpoint_hash,
                            fork)) = find_fork_mut(&mut self.forks, &block.prev_hash) {
            let parent_block = find_block(fork, &block.prev_hash).unwrap();

            Block::validate_child(parent_block, &block)?;
            println!("Found valid parent block in a fork");

            // If its parent was the last block in the fork, append the block to the fork
            if endpoint_hash == parent_block.hash {
                push_block(fork, &block);
                // Update the endpoint_hash of the extended fork in the map.
                self.forks.entry(forkpoint_hash.clone()).and_modify(|forks| {
                    let fork: Vec<Block> = forks.remove(&endpoint_hash).expect("fork definitely exists.");
                    forks.insert(block.hash.clone(), fork.clone());
                });

                println!("Extending an existing fork");
                let extended_fork: &Vec<Block> = self.forks.get(&forkpoint_hash).unwrap().get(&block.hash).unwrap();
                Ok(NextBlockResult::ExtendedFork {
                    length: extended_fork.len(),
                    forkpoint_idx: extended_fork.first().expect("fork is non-empty").idx - 1,
                    forkpoint_hash,
                    endpoint_idx,
                    endpoint_hash
                })
            }
            // Otherwise create a new direct fork from the main chain, whose prefix is a clone of an existing fork's, with
            else {
                // Truncate the fork until the block's parent, then push the new block on
                let new_fork: Vec<Block> = {
                    let mut fork_clone = fork.clone();
                    truncate_until(&mut fork_clone, |b| b.hash == block.prev_hash);
                    push_block(&mut fork_clone, &block);
                    fork_clone
                };
                // Insert the new fork into the map.
                self.forks.entry(forkpoint_hash.clone()).and_modify(|forks: &mut HashMap<String, Vec<Block>>| {
                    forks.insert(block.hash.clone(), new_fork.clone());
                });
                println!("Adding a new fork that branches off an existing fork to the chain");
                Ok(NextBlockResult::NewFork {
                    length: new_fork.len(),
                    forkpoint_idx: new_fork.first().expect("fork is non-empty").idx - 1,
                    forkpoint_hash,
                    endpoint_idx,
                    endpoint_hash
                })
            }
        }
        else {
            Ok(NextBlockResult::MissingParent {
                block_idx: block.idx,
                block_parent_hash: block.prev_hash
            })
        }
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        let last_block: &Block = self.last();
        let new_block = Block::mine_block(last_block, data);
        push_block(&mut self.main, &new_block)
    }

    // Try to append an
    pub fn show_forks(&self){
        for (forkpoint, forks_from) in self.forks.iter(){
            println!("Forks from {}", forkpoint);
            for (i, (_, fork)) in forks_from.iter().enumerate(){
                println!("Fork {}:", i);
                fork.iter().for_each(|block| println!("{}", block));
            }
        }
    }

    // Validate chain from head to tail, expecting it to begin at idx 0
    pub fn validate_chain(chain: &Chain) -> Result<(), ChainErr> {
        let first_block = chain.main.get(0).ok_or(ChainErr::ChainIsEmpty)?;
        if first_block.idx != 0 {
            return Err( ChainErr::ChainIsFork { first_block_idx: first_block.idx });
        }
        let mut curr = first_block;
        for i in 0..chain.len() - 1 {
            let next = chain.get(i + 1).unwrap();
            Block::validate_block(next).map_err(|e| ChainErr::InvalidSubChain(e))?;
            Block::validate_child(curr, next).map_err(|e| ChainErr::InvalidSubChain(e))?;
            curr = next;
        }
        Ok(())
    }

    // Choose the longest valid chain (defaulting to the local version). Returns true if chain was updated.
    pub fn choose_chain(&mut self, remote: &Chain) -> bool {
        match(Self::validate_chain(&self), Self::validate_chain(&remote))  {
            (Ok(_), Ok(_)) => {
                if self.main.len() >= remote.main.len() {
                    println!("Remote chain's length {} is not longer than ours of length {}.",  remote.main.len(), self.main.len());
                    false
                } else {
                    println!("Remote chain's length {} is longer than ours of length {}.",  remote.main.len(), self.main.len());
                    *self = remote.clone();
                    true
                }
            }
            (Err(e), Ok(_)) => {
                println!("Our current chain is invalid: {}.", e);
                *self = remote.clone();
                true
            }
            (Ok(_), Err(e)) => {
                println!("The remote chain is invalid: {}.", e);
                false
            }
            _ => panic!("local and remote chains both invalid")
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_, block) in self.main.iter().enumerate() {
            writeln!(f, "{}", block )?;
        };
        Ok(())
    }
}

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
                write!(f, "Block's parent cannot be found in the current chain nor forks.\n\
                           The missing parent has index {} and hash {}.", block_idx - 1, pretty_hex(block_parent_hash))
            }
            NextBlockResult::ExtendedMain { length, endpoint_idx, endpoint_hash } => {
                write!(f, "Extended the main chain to new length {}.\n\
                           TIts last block has index {} with hash {}.", length, endpoint_idx, pretty_hex(endpoint_hash))
            }
            NextBlockResult::ExtendedFork { length, forkpoint_idx, forkpoint_hash, endpoint_idx,  endpoint_hash} => {
                write!(f,  "Extended an existing fork from ({}, {}) on the main chain, to new length {}.\n\
                            Its last block has index {} with hash {}.",
                            forkpoint_idx, pretty_hex(forkpoint_hash), length, endpoint_idx, pretty_hex(endpoint_hash)
                )
            }
            NextBlockResult::NewFork { length, forkpoint_idx, forkpoint_hash, endpoint_idx,  endpoint_hash} => {
                write!( f, "Added a completely new fork from ({}, {}) on the main chain, with length {}. \n\
                            Its last block has index {} with hash {}.",
                            forkpoint_idx, pretty_hex(forkpoint_hash), length, endpoint_idx, pretty_hex(endpoint_hash)
                )
            }
        }
    }
}


// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    ChainIsEmpty,
    ChainIsFork{first_block_idx : usize},
    InvalidSubChain(NextBlockErr),
}

impl std::fmt::Display for ChainErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChainErr::ChainIsEmpty => {
                write!(f, "Chain is empty.")
            }
            ChainErr::ChainIsFork{first_block_idx}  => {
                write!(f, "Chain begins at index {} instead of 0.", first_block_idx)
            }
            ChainErr::InvalidSubChain (e) => {
                write!(f, "Chain contains invalid blocks or contiguous blocks:\n{}.", e)
            }
        }
    }
}

impl std::error::Error for ChainErr {}
