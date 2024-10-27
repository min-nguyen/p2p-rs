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
    forks: Forks
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

    pub fn lookup_block_idx(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
    }

    pub fn lookup_block_hash(&self, hash: &String) -> Option<&Block> {
        Block::find(&self.main, |block| block.hash == *hash)
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

    pub fn handle_block_result(&mut self, res : NextBlockResult) -> Result<ChooseChainResult, NextBlockErr>{
        match res {
            NextBlockResult::ExtendedFork { fork_hash,end_hash, .. } => {
                self.sync_to_fork(fork_hash, end_hash)
            },
            NextBlockResult::NewFork { fork_hash, end_hash, .. } => {
                self.sync_to_fork(fork_hash, end_hash)
            }
            NextBlockResult::ExtendedMain { length, .. } => {
                Ok(ChooseChainResult::KeepMain { main_len: length, other_len: None })
            }
        }
    }

    pub fn store_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr>{
        Block::validate_block(&block)?;

        // Search for block in the main chain.
        if let Some(..) = Block::find(&self.main, |b: &Block| b.hash == block.hash) {
            Err(NextBlockErr::Duplicate { block_idx: block.idx, block_hash: block.hash })
        }
        // Search for block in the forks.
        else if let Some(..) = fork::find_fork( &self.forks, |b| b.hash == block.hash) {
            Err(NextBlockErr::Duplicate { block_idx: block.idx, block_hash: block.hash })
        }
        // Search for parent block in the forks.
        else if let Some(parent_block) = Block::find(&self.main, |parent: &Block| parent.hash == block.prev_hash){

            Block::validate_child(parent_block, &block)?;

            // See if we can append the block to the main chain
            if self.last().hash == parent_block.hash {
                Block::push_end(&mut self.main, block);
                Ok(NextBlockResult::ExtendedMain { length: self.len(), end_idx: self.last().idx, end_hash: self.last().hash.clone() })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let ForkId { length, fork_idx, fork_hash, end_idx, end_hash}
                    = fork::insert_fork(&mut self.forks, vec![block.clone()])?;

                Ok(NextBlockResult::NewFork {length, fork_idx, fork_hash, end_idx, end_hash })
            }
        }
        // Search for parent block in the forks.
        else if let Some((fork, fork_id)) = fork::find_fork( &self.forks, |parent| parent.hash == block.prev_hash) {
            let parent = Block::find(fork, |parent| parent.hash == block.prev_hash).unwrap();
            Block::validate_child(parent, &block)?;

            // If its parent was the last block in the fork, append the block and update the endpoint key
            if *fork_id.end_hash == parent.hash {
                let ForkId { length, fork_idx, fork_hash, end_idx, end_hash}
                    = fork::extend_fork(&mut self.forks, &fork_id, block)?;

                Ok(NextBlockResult::ExtendedFork {length, fork_idx, fork_hash, end_idx, end_hash })
            }
            // Otherwise create a new fork from the main chain that clones the prefix of an existing fork
            else {
                let ForkId { length, fork_idx, fork_hash, end_idx, end_hash}
                    = fork::nest_fork(&mut self.forks, &fork_id, block)?;

                Ok(NextBlockResult::NewFork {length, fork_idx, fork_hash, end_idx, end_hash })
            }
        }
        // Otherwise, report a missing block that connects it to the current network
        else {
            if block.idx > 0 {
                Err(NextBlockErr::MissingParent {
                        block_parent_idx: block.idx - 1,
                        block_parent_hash: block.prev_hash
                })
            }
            else { // block.idx == 0 && not in main chain or forks
                Err(NextBlockErr::UnrelatedGenesis { genesis_hash: block.hash })
            }
        }
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        let last_block: &Block = self.last();
        let new_block = Block::mine_block(last_block, data);
        Block::push_end(&mut self.main, new_block)
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
    pub fn sync_to_chain(&mut self, other: Chain) -> Result<ChooseChainResult, NextBlockErr> {
        Self::validate_chain(&other)?;
        let (main_genesis, other_genesis) = (self.main.first().unwrap().hash.clone(), other.main.first().unwrap().hash.clone());
        if main_genesis != other_genesis {
            return Err (NextBlockErr::UnrelatedGenesis { genesis_hash: other_genesis  })
        }
        let (main_len, other_len) = (self.last().idx + 1, other.last().idx + 1);
        if main_len < other_len {
            *self = other.clone();
            Ok(ChooseChainResult::ChooseOther { main_len, other_len })
        } else {
            Ok(ChooseChainResult::KeepMain { main_len, other_len: Some(other_len) })
        }
    }

    // Validate fork, expecting its prev block to be in the main chain
    pub fn validate_fork(&self, fork: &Vec<Block>) -> Result<(), NextBlockErr> {
        Block::validate_blocks(fork)?;
        let first_block = fork.first().unwrap();
        if let Some(forkpoint) = self.lookup_block_hash( &first_block.prev_hash) {
            Block::validate_child(forkpoint, first_block)?;
            Ok (())
        }
        else {
            Err(NextBlockErr::MissingParent { block_parent_idx: first_block.idx - 1, block_parent_hash: first_block.prev_hash.clone()})
        }
    }

    // Swap the main chain to a fork in the pool if longer
    pub fn sync_to_fork(&mut self, fork_hash: String, end_hash: String) -> Result<ChooseChainResult, NextBlockErr>{
        if let Some((_, fork_id)) = fork::lookup_fork_mut(&mut self.forks, &fork_hash, &end_hash) {
            let (main_len, other_len) = (self.last().idx + 1, fork_id.end_idx + 1);
            if main_len < other_len {
                // remove the fork from the fork pool
                let mut fork
                    = fork::remove_fork(&mut self.forks, &fork_id.fork_hash, &fork_id.end_hash)
                            .expect("fork definitely exists; we just stored it");
                // truncate the main chain to the forkpoint, and append the fork to it
                let main_suffix: Vec<Block> = Block::split_off_until(&mut self.main, |b| b.hash == *fork_id.fork_hash);
                Block::append(&mut self.main, &mut fork);
                // insert the main chain as a new fork
                fork::insert_fork(&mut self.forks, main_suffix)?;

                return Ok(ChooseChainResult::ChooseOther { main_len, other_len })
            }
            else {
                return Ok(ChooseChainResult::KeepMain { main_len, other_len: Some(other_len) })
            }
        }
        Ok(ChooseChainResult::KeepMain { main_len: self.len(), other_len: None })
    }

    // Store a valid fork
    pub fn connect_fork(&mut self, fork: Vec<Block>) -> Result<ForkId, NextBlockErr>{
        Self::validate_fork(&self, &fork)?;
        fork::insert_fork(&mut self.forks, fork.clone())
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
        other_len: Option<usize>
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
                write!(f, "Keeping current main chain with length {}", main_len)?;
                if let Some(other_len) = other_len {
                    write!(f, ", other chain/fork has total length {}.", other_len)?
                }
                write!(f, ".")
            }
            ChooseChainResult::ChooseOther {  main_len,  other_len } => {
                write!(f, "Choosing other chain/fork with length {}, previous main chain has length {}.", other_len, main_len)
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


// Handle it as an orphan for a child block in the orphans
// else {
//     if let Some((orphan_fork, fork_id)) =
//             fork::find_fork( &self.orphans, |child| child.prev_hash == block.hash)  {
//        let child = Block::find(orphan_fork, |child| child.prev_hash == block.hash).unwrap();
//        Block::validate_child(&block, &child)?;

//        // If its child was the first block in the orphan fork, prepend the block and update the forkpoint key
//        if fork_id.fork_hash == child.prev_hash {
//         println!("Found tip of orphan");
//             let ForkId {  fork_idx, fork_hash, ..}
//                 = fork::prepend_fork(&mut self.orphans, &fork_id, block)?;
//             Err(NextBlockErr::MissingParent {
//                 block_parent_idx: fork_idx,
//                 block_parent_hash: fork_hash
//             })
//        }
//        else {
//         /* This should not happen */
//            Err(NextBlockErr::MissingParent {
//                 block_parent_idx: block.idx - 1,
//                block_parent_hash: block.prev_hash
//            })
//        }
//    }
// else {
        // let ForkId {  fork_idx, fork_hash, ..}
        //     = fork::insert_fork(&mut self.orphans, vec![block.clone()])?;
        // Err(NextBlockErr::MissingParent {
        //         block_parent_idx: fork_idx,
        //         block_parent_hash: fork_hash
        // })
// }