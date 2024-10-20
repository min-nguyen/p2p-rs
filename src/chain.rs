/*
    *Chain*:
    - Chain, a safe wrapper around a vector of blocks, and error types
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use libp2p::futures::stream::Next;
use serde::{Deserialize, Serialize};
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

    pub fn handle_new_block(&mut self, block: Block) -> Result<(), NextBlockErr>{
        Block::validate_block(&block)?;

        // Search for the parent block in the main chain.
        if let Some(parent_block) = find_block(&self.main, &block.prev_hash){
            Block::validate_child(parent_block, &block)?;
            println!("Found parent block in main chain.");

            // See if we can append the block to the main chain
            if self.last().hash == parent_block.hash {
               push_block(&mut self.main, &block);
               println!("Extending the main chain");
            //    Ok(NextBlockResult::ExtendedMainChain {
            //         length: self.len(),
            //         endpoint_idx: self.last().idx,
            //         endpoint_hash: self.last().hash.clone()
            //     });
                Ok (())
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                // need to manually validate it to create a single-block chain
                let new_fork = vec![block.clone()];
                let forks_from: &mut HashMap<String, Vec<Block>>
                    = self.forks.entry(block.prev_hash.to_string()).or_insert(HashMap::new());
                forks_from.insert(block.hash.to_string() // end hash
                                , new_fork);
                println!("Adding a single-block fork to the main chain");
                // Ok(NextBlockResult::NewFork {
                //     length: 1,
                //     forkpoint_idx: block.idx - 1,
                //     forkpoint_hash: block.prev_hash,
                //     endpoint_idx: block.idx,
                //     endpoint_hash: block.hash.to_string()
                // });

                Ok (())
            }
        }
        // Search for the parent block in the forks.
        else if let Some((  forkpoint_hash,
                            endpoint_hash,
                            fork)) = find_fork_mut(&mut self.forks, &block.prev_hash) {
            let parent_block = find_block(fork, &block.prev_hash).unwrap();
            Block::validate_child(parent_block, &block)?;
            println!("Found parent block in a fork");

            // If its parent was the last block in the fork, append the block to the fork
            if endpoint_hash == parent_block.hash {
                push_block(fork, &block);
                // Update the endpoint_hash of the extended fork in the map.
                self.forks.entry(forkpoint_hash).and_modify(|forks| {
                    let fork: Vec<Block> = forks.remove(&endpoint_hash).expect("Fork definitely exists; we just pushed a block to it.");
                    forks.insert(block.hash, fork);
                });
                println!("Extending an existing fork");
                Ok (())
                // Ok(NextBlockResult::ExtendedFork {
                //     length: fork.len(),
                //     forkpoint_idx: fork.first().unwrap().idx - 1,
                //     forkpoint_hash,
                //     endpoint_idx: block.idx,
                //     endpoint_hash: block.hash
                // })
            }
            // Otherwise create a new direct fork from the main chain, whose prefix is a clone of an existing fork's, with
            else {
                // Truncate the fork until the block's parent,
                let mut new_fork: Vec<Block> = {
                    let mut fork_clone = fork.clone();
                    truncate_until(&mut fork_clone, |block| block.hash == block.prev_hash);
                    push_block(&mut fork_clone, &block);
                    fork_clone
                };
                // Push the new block on
                // Insert the new fork into the map.
                self.forks.entry(forkpoint_hash).and_modify(|forks| {
                    forks.insert(block.hash, new_fork);
                });
                println!("Adding a new fork that branches off an existing fork to the chain");
                // Ok(NextBlockResult::NewFork {
                //     length: new_fork.len(),
                //     forkpoint_idx: fork.first().unwrap().idx - 1,
                //     forkpoint_hash,
                //     endpoint_idx: block.idx,
                //     endpoint_hash: block.hash
                // });
                Ok (())
            }
        }
        else {
            // Ok(NextBlockResult::MissingParent {
            //     block_idx: block.idx,
            //     block_parent_hash: block.prev_hash
            // });
            Ok (())
        }
    }

    // Mine a new valid block from given data
    pub fn mine_new_block(&mut self, data: &str) -> Block {
        let current_block: &Block = self.last();
        Block::mine_block(current_block.idx + 1, data, &current_block.hash)
    }


    // Try to append an arbitrary block to the main chain
    pub fn mine_then_push_block(&mut self, data: &str) {
        let b: Block = self.mine_new_block(data);
        push_block(&mut self.main, &b);
    }

    pub fn show_forks(&self){
        for (forkpoint, forks_from) in self.forks.iter(){
            println!("Forks from {}", forkpoint);
            for (i, (_, fork)) in forks_from.iter().enumerate(){
                println!("Fork {} from {}",i, forkpoint);
                fork.iter().for_each(|block| println!("{}", block));
            }
        }
    }

    // Validate chain from head to tail, expecting it to begin at idx 0
    pub fn validate_chain(chain: &Chain) -> Result<(), ChainErr> {
        let first_block = chain.main.get(0).ok_or(ChainErr::ChainIsEmpty)?;
        if first_block.idx != 0 {
            return Err(ChainErr::ChainIsFork);
        }
        Self::validate_subchain(&chain.main).map_err(ChainErr::InvalidSubChain)
    }

    // (Keep private) validate subchain from head to tail, ignoring the first block
    fn validate_subchain(subchain: &Vec<Block>) -> Result<(), NextBlockErr> {
        if let Some(mut curr) = subchain.first(){
            Block::validate_block(curr)?;
            for i in 0..subchain.len() - 1 {
                let next = subchain.get(i + 1).unwrap();
                Block::validate_block(next)?;
                Block::validate_child(curr, next)?;
                curr = next;
            }
            Ok(())
        }
        else {
            Ok(())
        }
    }

    // Choose the longest valid chain (defaulting to the local version). Returns true if chain was updated.
    pub fn choose_chain(&mut self, remote: &Chain) -> bool {
        match(Self::validate_chain(&self), Self::validate_chain(&remote))  {
            (Ok(()), Ok(())) => {
            if self.main.len() >= remote.main.len() {
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

    // // Try to attach a fork to extend any compatible parent block in the current chain. (Can succeed even if resulting in a shorter chain.)
    // //  - Not currently being used outside of testing.
    // pub fn try_merge_fork(&mut self, fork: &mut Vec<Block>) -> Result<(), ForkErr>{
    //     let fork_head: &Block = fork.get(0).ok_or(ForkErr::ForkIsEmpty)?;
    //     Self::validate_fork(&fork)?;

    //     /* this should behave the same:
    //         match self.get(&fork_head.idx - 1) {
    //             Some(forkpoint) if (forkpoint.hash == fork_head.prev_hash) => {
    //     */
    //     match self.lookup(&fork_head.prev_hash) {
    //         // if fork branches off from idx n, then keep the first n + 1 blocks
    //         Some(forkpoint) => {
    //             self.main.truncate(forkpoint.idx + 1);
    //             self.main.append(fork);
    //             Ok(())
    //         }
    //         // fork's first block doesn't reference a block in the current chain.
    //         None => {
    //             Err(ForkErr::ForkIncompatible)
    //         }
    //     }
    // }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_, block) in self.main.iter().enumerate() {
            writeln!(f, "{}", block )?;
        };
        Ok(())
    }
}

//
pub enum NextBlockResult {
    MissingParent {
        block_idx: usize,
        block_parent_hash: String
    },
    ExtendedMainChain {
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
}

// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    ChainIsEmpty,
    ChainIsFork,
    InvalidSubChain(NextBlockErr),
}

impl std::fmt::Display for ChainErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChainErr::ChainIsEmpty => {
                write!(f, "Chain is empty")
            }
            ChainErr::ChainIsFork  => {
                write!(f, "Chain doesn't begin at index 0")
            }
            ChainErr::InvalidSubChain (e) => {
                write!(f, "Chain contains invalid blocks or contiguous blocks: {}", e)
            }
        }
    }
}

impl std::error::Error for ChainErr {}
