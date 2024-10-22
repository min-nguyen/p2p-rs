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
    pub fn find_fork_mut<'a>(&'a mut self, hash: &String)
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

    // pub fn switch_main(&mut self, (fork_point, end_point): (String, String)) -> Option<()>{
    //     let fork = self.forks.get_mut(&fork_point)?.get_mut(&end_point)?;
    //     if fork.last().expect("fork must be non-empty").idx > self.last().idx {
    //         let suffix: Vec<Block> = Block::split_off_until(&mut self.main, |b| b.hash == fork_point);
    //     }
    //     Some (())
    // }

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
                println!("Extending an existing fork");
                Ok(NextBlockResult::ExtendedFork {
                    length: extended_fork.len(),
                    forkpoint_idx: extended_fork.first().unwrap().idx - 1,
                    forkpoint_hash: forkpoint,
                    endpoint_idx: block.idx,
                    endpoint_hash: block.hash
                })
            }
            // Otherwise create a new direct fork from the main chain, whose prefix is a clone of an existing fork's, with
            else {
                // Truncate the fork until the block's parent, then push the new block on
                let new_fork: Vec<Block> = {
                    let mut fork_clone = fork.clone();
                    let _ = Block::split_off_until(&mut fork_clone, |b| b.hash == block.prev_hash);
                    Block::push(&mut fork_clone, &block);
                    fork_clone
                };

                // Insert the new fork into the map.
                self.forks.entry(forkpoint.clone()).and_modify(|forks: &mut HashMap<String, Vec<Block>>| {
                    forks.insert(block.hash.clone(), new_fork.clone());
                });

                Ok(NextBlockResult::NewFork {
                    length: new_fork.len(),
                    forkpoint_idx: new_fork.first().unwrap().idx - 1,
                    forkpoint_hash: forkpoint,
                    endpoint_idx: block.idx,
                    endpoint_hash: block.hash
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
        Block::push(&mut self.main, &new_block)
    }

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
    pub fn validate_chain(chain: &Chain) -> Result<(), NextBlockErr> {
        let first_block = chain.main.get(0).ok_or(NextBlockErr::EmptyChain)?;
        if first_block.idx != 0 {
            return Err( NextBlockErr::InvalidIndex { block_idx: first_block.idx, expected_idx: 0 });
        }
        Block::validate_blocks(&chain.main)
    }

    // Choose the longest valid chain (defaulting to the local version).
    pub fn choose_chain(&mut self, remote: &Chain) -> Result<ChooseChainResult, NextBlockErr> {
        match Self::validate_chain(&remote)  {
            Ok(_) => {
                if self.main.len() >= remote.main.len() {
                    println!("Remote chain's length {} is not longer than ours of length {}.",  remote.main.len(), self.main.len());
                    Ok(ChooseChainResult::ChooseMain { main_len: self.main.len(), other_len: remote.main.len() })
                } else {
                    *self = remote.clone();
                    Ok(ChooseChainResult::ChooseOther { main_len: self.main.len(), other_len: remote.main.len() })
                }
            }
            Err(e) => {
                Err(e)
            }
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
    ChooseMain {
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
            ChooseChainResult::ChooseMain { main_len, other_len   } => {
                write!(f, "Other chain length {} is not longer than main chain of length {}.", other_len, main_len)
            }
            ChooseChainResult::ChooseOther {  main_len,  other_len } => {
                write!(f, "Other chain length {} is longer than main chain of length {}.", other_len, main_len)
            }
        }
    }
}
