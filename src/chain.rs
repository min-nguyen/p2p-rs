/*
    *Chain*:
    - Chain internals, a safe wrapper that manages a main chain and a hashmap of forks.
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};

use super::block::{Block::{self}, NextBlockResult, NextBlockErr};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main : Vec<Block>,
    // <fork point, <fork end hash, forked blocks>>
    forks: HashMap<String, HashMap<String, Vec<Block>>>,
}

impl Chain {
    // New chain with a single genesis block
    pub fn genesis() -> Self {
        Self { main : vec![Block::genesis()], forks : HashMap::new() }
    }

    // Safely construct a chain from a vector of blocks
    pub fn from_vec(blocks: Vec<Block>) -> Result<Chain, NextBlockErr> {
        let chain = Chain{main : blocks, forks : HashMap::new()};
        Self::validate_chain(&chain)?;
        Ok(chain)
    }

    pub fn to_vec(&self) -> Vec<Block> {
        self.main.clone()
    }

    pub fn last(&self) -> &Block {
        self.main.last().expect("Chain should always be non-empty")
    }

    pub fn len(&self) -> usize {
        self.main.len()
    }

    pub fn get(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
    }

    pub fn find(&self, hash: &String) -> Option<&Block> {
        Block::find(&self.main, hash)
    }

    // Safe split off that ensures the main chain is always non-empty
    pub fn split_off(&mut self, len: usize) -> Option<Vec<Block>> {
        if len == 0 {
            None
        }
        else {
            let main_chain_len = self.len();
            Some(Block::split_off(&mut self.main, std::cmp::min(main_chain_len, len)))
        }
    }

    // Check if block is in any fork, returning the fork point, end hash, and fork
    fn find_fork_mut<'a>(&'a mut self, hash: &String)
        -> Option<(String, String, &'a mut Vec<Block>)> {
        // iterate through fork points
        for (fork_point, forks_from) in &mut self.forks {
            // iterate through forks from the fork point
            for (end_hash, fork) in forks_from {
                // iterate through blocks in the fork
                if let Some(_) = Block::find(fork, hash) {
                    return Some((fork_point.clone(), end_hash.clone(), fork))
                }
            }
        }
        None
    }

    // Check if block is in any fork, returning the fork point, end hash, and fork
    pub fn lookup_fork_mut<'a>(&'a mut self, forkpoint: &String, endpoint: &String) -> Option<&'a mut Vec<Block>>{
        self.forks.get_mut(forkpoint).and_then(|forks| forks.get_mut(endpoint))
    }

    // Store a valid fork (replacing any existing one), returning its forkpoint, endpoint, and last block's index
    pub fn insert_fork(&mut self, fork: Vec<Block>) -> Result<(String, String, usize), NextBlockErr>{
        // check if fork is valid and hence non-empty
        Self::validate_fork(self, &fork)?;

        let (forkpoint, (endpoint, endidx))
            = ( fork.first().unwrap().prev_hash.clone(),
                { let end_block = fork.last().unwrap();
                  (end_block.hash.clone(), end_block.idx)
                });

        self.forks.entry(forkpoint.clone())
                                    .or_insert(HashMap::new())
                                    .insert(endpoint.clone(), fork);

        Ok ((forkpoint, endpoint, endidx))
    }

    pub fn handle_new_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr>{
        Block::validate_block(&block)?;

        // Search for the parent block in the main chain.
        if let Some(parent_block) = Block::find(&self.main, &block.prev_hash){

            Block::validate_child(parent_block, &block)?;

            // See if we can append the block to the main chain
            if self.last().hash == parent_block.hash {
                Block::push(&mut self.main, &block);
                Ok(NextBlockResult::ExtendedMain {
                        length: self.len(),
                        endpoint_idx: block.idx,
                        endpoint_hash: block.hash
                    })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let new_fork = vec![block.clone()];
                let forks_from_parent = self.forks.entry(parent_block.hash.to_string()).or_insert(HashMap::new());
                forks_from_parent.insert(block.hash.to_string(), new_fork);

                Ok(NextBlockResult::NewFork {
                    length: 1,
                    forkpoint_idx: block.idx - 1,
                    forkpoint_hash: block.prev_hash,
                    endpoint_idx: block.idx,
                    endpoint_hash: block.hash
                })
            }
        }
        // Search for the parent block in the forks.
        else if let Some((  forkpoint,
                            endpoint,
                            fork)) = self.find_fork_mut( &block.prev_hash) {
            let parent_block = Block::find(fork, &block.prev_hash).unwrap();

            Block::validate_child(parent_block, &block)?;

            // If its parent was the last block in the fork, append the block to the fork
            if endpoint == parent_block.hash {
                // Update the endpoint_hash of the extended fork in the map.
                let extended_fork: &Vec<Block> = {
                    Block::push(fork, &block);
                    self.forks.entry(forkpoint.clone()).and_modify(|forks| {
                        let fork: Vec<Block> = forks.remove(&endpoint).expect("fork definitely exists.");
                        forks.insert(block.hash.clone(), fork.clone());
                    });
                    self.forks.get(&forkpoint).unwrap().get(&block.hash).unwrap()
                };
                // println!("Extending an existing fork");

                Ok(NextBlockResult::ExtendedFork {
                    length: extended_fork.len(),
                    forkpoint_idx: extended_fork.first().unwrap().idx - 1,
                    forkpoint_hash: forkpoint,
                    endpoint_idx: block.idx,
                    endpoint_hash: block.hash
                })
            }
            // Otherwise create a new direct fork from the main chain, cloning the prefix of an existing fork
            else {
                // Truncate the fork until the block's parent, then push the new block on
                let nested_fork: Vec<Block> = {
                    let mut fork_clone = fork.clone();
                    let _ = Block::split_off_until(&mut fork_clone, |b| b.hash == block.prev_hash);
                    Block::push(&mut fork_clone, &block);
                    fork_clone
                };
                // Insert the new fork into the map.
                self.forks.entry(forkpoint.clone()).and_modify(|forks: &mut HashMap<String, Vec<Block>>| {
                    forks.insert(block.hash.clone(), nested_fork.clone());
                });

                Ok(NextBlockResult::NewFork {
                    length: nested_fork.len(),
                    forkpoint_idx: nested_fork.first().unwrap().idx - 1,
                    forkpoint_hash: forkpoint,
                    endpoint_idx: block.idx,
                    endpoint_hash: block.hash
                })
            }
        }
        else {
            Err(NextBlockErr::MissingParent {
                block_idx: block.idx,
                block_parent_hash: block.prev_hash
            })
        }
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        let last_block: &Block = self.last();
        let new_block = Block::mine_block(last_block, data);
        Block::push(&mut self.main, &new_block)
    }

    // Validate chain, expecting its first block to begin at idx 0
    pub fn validate_chain(&self) -> Result<(), NextBlockErr> {
        Block::validate_blocks(&self.main)?;
        let first_block = self.main.first().unwrap();
        if first_block.idx == 0 {
            Ok(())
        }
        else {
            Err( NextBlockErr::InvalidIndex { block_idx: first_block.idx, expected_idx: 0 })
        }
    }

    // Swap the main chain to a remote chain if valid and longer.
    pub fn sync_to_chain(&mut self, remote: Chain) -> Result<ChooseChainResult, NextBlockErr> {
        Self::validate_chain(&remote)?;
        let (main_len, other_len) = (self.last().idx + 1, remote.last().idx + 1);
        if main_len >= other_len {
            Ok(ChooseChainResult::KeepMain { main_len, other_len })
        } else {
            *self = remote.clone();
            Ok(ChooseChainResult::ChooseOther { main_len, other_len })
        }
    }

    // Validate fork, expecting its prev block to be in the main chain
    pub fn validate_fork(&self, fork: &Vec<Block>) -> Result<(), NextBlockErr> {
        Block::validate_blocks(fork)?;
        let first_block = fork.first().unwrap();
        if let Some(forkpoint) = self.find(&first_block.prev_hash) {
            Block::validate_child(forkpoint, first_block)?;
            Ok (())
        }
        else {
            Err(NextBlockErr::MissingParent { block_idx: first_block.idx, block_parent_hash: first_block.prev_hash.clone()})
        }
    }

    // Store a valid fork, and then swap the main chain to it if longer
    pub fn sync_to_fork(&mut self, fork: Vec<Block>) -> Result<ChooseChainResult, NextBlockErr>{
        // clone and store fork
        let (forkpoint, endpoint, fork_last_idx) = self.insert_fork(fork)?;
        // if main chain is shorter than forked chain, swap its blocks from the forkpoint onwards
        let (main_len, other_len) = (self.last().idx + 1, fork_last_idx + 1);
        if main_len < other_len {
            let forks: &mut HashMap<String, Vec<Block>> = self.forks.get_mut(&forkpoint).unwrap();
            // remove the fork from the fork pool
            let mut fork = forks.remove_entry(&endpoint).expect("fork definitely exists; we just stored it").1;
            // truncate the main chain to the forkpoint, and append the fork to it
            let main_suffix: Vec<Block> = Block::split_off_until(&mut self.main, |b| b.hash == *forkpoint);
            Block::append(&mut self.main, &mut fork);
            // insert the main chain as a new fork
            forks.insert(main_suffix.last().unwrap().hash.clone(), main_suffix);

            Ok(ChooseChainResult::SwitchToFork { main_len, other_len })
        }
        else {
            Ok(ChooseChainResult::KeepMain { main_len, other_len })
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
pub enum ChooseChainResult {
    KeepMain {
        main_len: usize,
        other_len: usize
    },
    SwitchToFork {
        main_len: usize,
        other_len: usize
    },
    ChooseOther {
        main_len: usize,
        other_len: usize
    }
}

impl std::fmt::Display for ChooseChainResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChooseChainResult::KeepMain { main_len, other_len   } => {
                write!(f, "Main chain length {} is longer than other chain length {}.", main_len, other_len)
            }
            ChooseChainResult::SwitchToFork { main_len, other_len   } => {
                write!(f, "Local fork length {} is longer than main chain length {}.", other_len, main_len)
            }
            ChooseChainResult::ChooseOther {  main_len,  other_len } => {
                write!(f, "Other chain length {} is longer than main chain length {}.", other_len, main_len)
            }
        }
    }
}

pub fn show_forks(chain : &Chain){
    for (forkpoint, forks_from) in chain.forks.iter(){
        println!("Forks from {}", forkpoint);
        for (i, (_, fork)) in forks_from.iter().enumerate(){
            println!("Fork {}:", i);
            fork.iter().for_each(|block| println!("{}", block));
        }
    }
}

// // Return a reference to the longest stored fork
// pub fn longest_fork<'a>(&'a self) -> Option<&'a Vec<Block>>{
//     let longest_fork: Option<&'a Vec<Block>> = None;

//     self.forks
//             .values()
//             .flat_map(|forks| forks.values())
//             .fold(longest_fork,
//                 |longest, current|
//                 match longest {
//                     Some(fork) if fork.len() >= current.len() => Some(fork),
//                     _ => Some(current),
//                 })
// }