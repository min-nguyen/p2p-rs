/*
    *Chain*:
    - Chain internals, a safe wrapper that manages a main chain and a hashmap of forks.
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};

use crate::fork::ForkId;

use super::block::{Block::{self}, NextBlockResult, NextBlockErr};
use super::fork::{self, Forks};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main : Vec<Block>,
    forks: Forks,
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

    pub fn forks<'a>(&'a self) -> &'a Forks {
        &self.forks
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
                        end_idx: block.idx,
                        end_hash: block.hash
                    })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let new_fork = vec![block.clone()];
                fork::insert_fork(&mut self.forks, new_fork)?;

                Ok(NextBlockResult::NewFork {
                    length: 1,
                    fork_idx: block.idx - 1,
                    fork_hash: block.prev_hash,
                    end_idx: block.idx,
                    end_hash: block.hash
                })
            }
        }
        // Search for the parent block in the forks.
        else if let Some(fork) = fork::find_fork( &self.forks, &block.prev_hash) {
            let ForkId { fork_hash, fork_idx, end_hash, .. } = fork::identify_fork(fork)?;
            let parent_block = Block::find(fork, &block.prev_hash).unwrap();

            Block::validate_child(parent_block, &block)?;

            // If its parent was the last block in the fork, append the block and update the endpoint key
            if end_hash == parent_block.hash {
                let mut extended_fork: Vec<Block> = fork::remove_fork(&mut self.forks, &fork_hash, &end_hash).unwrap();
                Block::push(&mut extended_fork, &block);
                let length = extended_fork.len();
                fork::insert_fork(&mut self.forks, extended_fork.clone())?;

                Ok(NextBlockResult::ExtendedFork {
                    length,
                    fork_idx,
                    fork_hash,
                    end_idx: block.idx,
                    end_hash: block.hash
                })
            }
            // Otherwise create a new direct fork from the main chain, cloning the prefix of an existing fork
            else {
                let mut nested_fork: Vec<Block> = fork.clone();
                Block::split_off_until(&mut nested_fork, |b| b.hash == block.prev_hash);
                Block::push(&mut nested_fork, &block);
                let length = nested_fork.len();
                fork::insert_fork(&mut self.forks, nested_fork)?;

                Ok(NextBlockResult::NewFork {
                    length,
                    fork_idx,
                    fork_hash,
                    end_idx: block.idx,
                    end_hash: block.hash
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
        Self::validate_fork(&self, &fork)?;
        let (forkpoint, endpoint) = fork::insert_fork(&mut self.forks, fork.clone())?;
        // if main chain is shorter than forked chain, swap its blocks from the forkpoint onwards
        let (main_len, other_len) = (self.last().idx + 1, fork.last().unwrap().idx + 1);
        if main_len < other_len {
            // remove the fork from the fork pool
            let mut fork = fork::remove_fork(&mut self.forks, &forkpoint, &endpoint).expect("fork definitely exists; we just stored it");
            // truncate the main chain to the forkpoint, and append the fork to it
            let main_suffix: Vec<Block> = Block::split_off_until(&mut self.main, |b| b.hash == *forkpoint);
            Block::append(&mut self.main, &mut fork);
            // insert the main chain as a new fork
            fork::insert_fork(&mut self.forks, main_suffix)?;

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