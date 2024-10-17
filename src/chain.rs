/*
    *Chain*:
    - Chain, a safe wrapper around a vector of blocks, and error types
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};
use super::block::{Block::{self}, NextBlockErr};
use std::collections::HashMap;

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

// For validating forks of chains
#[derive(Debug)]
pub enum ForkErr {
    ForkIsEmpty,
    ForkStartsAtGenesis,
    ForkIncompatible,
    InvalidSubChain(NextBlockErr),
}

impl std::fmt::Display for ForkErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ForkErr::ForkIsEmpty => {
                write!(f, "Fork is empty")
            }
            ForkErr::ForkStartsAtGenesis  => {
                write!(f, "Fork begins at index 0")
            }
            ForkErr::ForkIncompatible => {
                write!(f, "Fork's first block has a parent hash not matching any block in the chain")
            }
            ForkErr::InvalidSubChain (e) => {
                write!(f, "Fork contains invalid blocks or contiguous blocks:  {}", e)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main : Vec<Block>,

    // <fork point, <fork end hash, forked blocks>>
    pub forks: HashMap<String, HashMap<String, Vec<Block>>>,
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

    pub fn get_block_by_hash(&self, hash: &String) -> Option<&Block> {
        self.main.iter().find(|b: &&Block| b.hash == *hash)
    }

    pub fn last(&self) -> &Block {
        self.main.last().expect("Chain should always be non-empty")
    }

    pub fn len(&self) -> usize {
        self.main.len()
    }

    pub fn truncate(&mut self, len: usize){
        self.main.truncate(std::cmp::min(self.len() - 1, len));
    }

    pub fn has_block(blocks: &Vec<Block>, block_hash: &String) -> bool {
        blocks.iter().any(|block| &block.hash == block_hash)
    }

    // Try to append an arbitrary block to the main chain
    pub fn handle_new_block(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
        if let Err(e) = Self::push_block(&mut self.main, new_block){
            match e.clone() {
                NextBlockErr::NextBlockInFork { block_idx, block_parent_hash, current_hash } => {
                    // If we can find a fork that we should be able to extend, try to extend it and terminate here.
                    if let Some((fork_point, end_hash)) = self.find_fork(&new_block.prev_hash) {
                        let mut fork = self.forks.get_mut(&fork_point).unwrap().get_mut(&end_hash).unwrap();
                        return Self::push_block(&mut fork, new_block)
                    }
                    // Otherwise, we are missing information about that fork
                    else {
                        return Err(NextBlockErr::MissingBlock { block_idx: new_block.idx, block_parent_hash: new_block.prev_hash.clone(),  })
                    }
                },
                NextBlockErr::CompetingBlockInFork { block_idx, block_parent_hash } => {
                    let mut forks
                         = self.forks.get_mut(&block_parent_hash).unwrap_or(&mut HashMap::new());
                    forks.insert(new_block.hash.clone(), vec![new_block.clone()]);

                },
                _ => {
                    return Err(e)
                }
            }
        }
        Ok(())

    }

    // Mine a new valid block from given data
    pub fn mine_new_block(&mut self, data: &str) -> Block {
        let current_block: &Block = self.last();
        Block::mine_block(current_block.idx + 1, data, &current_block.hash)
    }

    // Try to append an arbitrary block to the main chain
    fn push_block(blocks: &mut Vec<Block>, new_block: &Block) -> Result<(), NextBlockErr>{
        let current_block: &Block = blocks.last().expect("Blocks should be non-empty");
        Self::validate_next_block(current_block, &new_block)?;
        blocks.push(new_block.clone());
        Ok(())
    }

    pub fn mine_then_push_block(&mut self, data: &str) {
        let b: Block = self.mine_new_block(data);
        Self::push_block(&mut self.main, &b).expect("can push newly mined block")
    }

    // Check if block is in any fork, returning the fork point and end hash
    fn find_fork(&self, parent_hash: &String) -> Option<(String, String)> {
        for (fork_point, forks) in &self.forks {
            if let Some((end_hash, _))
                    = forks.iter().find(|(_, fork)|
                        fork.last().expect("Fork must be non-empty").hash == *parent_hash) {
                return Some((fork_point.clone(), end_hash.clone()));
            }
        }
        None
    }

    // // Try to append an arbitrary block to the main chain
    // pub fn push_block_to_fork(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
    //     let current_block: &Block = self.last();
    //     Self::validate_next_block(current_block, &new_block)?;
    //     self.main.push(new_block.clone());
    //     Ok(())
    // }

    // Try to attach a fork (suffix of a full chain) to extend any compatible parent block in the current chain
    // Note: Can succeed even if resulting in a shorter chain.
    pub fn try_merge_fork(&mut self, fork: &mut Vec<Block>) -> Result<(), ForkErr>{
        let fork_head: &Block = fork.get(0).ok_or(ForkErr::ForkIsEmpty)?;
        Self::validate_fork(&fork)?;

        /* this should behave the same:
            ```
            match self.get(&fork_head.idx - 1) {
                Some(forkpoint) if (forkpoint.hash == fork_head.prev_hash) => {
            ```
        */
        match self.get_block_by_hash(&fork_head.prev_hash) {
            // if fork branches off from idx n, then keep the first n + 1 blocks
            Some(forkpoint) => {
                self.main.truncate(forkpoint.idx + 1);
                self.main.append(fork);
                Ok(())
            }
            // fork's first block doesn't reference a block in the current chain.
            None => {
                Err(ForkErr::ForkIncompatible)
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

    // Validate fork from head to tail, expecting it to begin at any idx
    pub fn validate_fork(fork: &Vec<Block>) -> Result<(), ForkErr> {
        let first_block = fork.get(0).ok_or(ForkErr::ForkIsEmpty)?;
        if first_block.idx == 0 {
            return Err(ForkErr::ForkStartsAtGenesis);
        }
        Self::validate_subchain(&fork).map_err(ForkErr::InvalidSubChain)
    }

    // (Keep private) validate subchain from head to tail, ignoring the first block
    fn validate_subchain(subchain: &Vec<Block>) -> Result<(), NextBlockErr> {
        for i in 0..subchain.len() - 1 {
            let curr: &Block = subchain.get(i)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            let next: &Block = subchain.get(i + 1)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            if let Err(e) = Self::validate_next_block(curr, next) {
                return Err(e);
            }
        }
        Ok(())
    }

    // Validating whether one block is a valid next block for another.
    pub fn validate_next_block(current_block: &Block, block: &Block) -> Result<(), NextBlockErr> {
        // * check validity of block by itself
        if let Err(e) = Block::validate_block(block) {
            return Err(NextBlockErr::InvalidBlock(e));
        }

        // * check validity of block with respect to our chain
        //    1. if the block is out-of-date with our chain
        if block.idx < current_block.idx {
            return Err(NextBlockErr::BlockTooOld {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }
        //    2. if the block is up-to-date (i.e. competes) with our chain
        if block.idx == current_block.idx {
            //   a. competing block is a duplicate of ours
            if block.hash == current_block.hash {
                return Err(NextBlockErr::DuplicateBlock {
                    block_idx: block.idx,
                });
            }
            //   b. competing block is different and has the same parent
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 1
            if block.prev_hash == current_block.prev_hash {
                return Err(NextBlockErr::CompetingBlock {
                    block_idx: block.idx,
                    block_parent_hash: block.prev_hash.clone(),
                });
            }
            //   c. competing block is different and has a different parent, indicating their chain has possibly forked
            //      - either ignore it, or store it temporarily and see if it can be used when receiving a block with idx + 2
            if block.prev_hash != current_block.prev_hash {
                return Err(NextBlockErr::CompetingBlockInFork {
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
                return Err(NextBlockErr::NextBlockInFork {
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
            return Err(NextBlockErr::BlockTooNew {
                block_idx: block.idx,
                current_idx: current_block.idx,
            });
        }

        Err(NextBlockErr::UnknownError)
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
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_, block) in self.main.iter().enumerate() {
            writeln!(f, "{}", block )?;
        };
        Ok(())
    }
}

type Forks = HashMap<String, Vec<Block>>;

pub fn prepend_in_forks(forks : &mut Forks, missing_block: Block){
    let fork = forks.entry(missing_block.hash.clone()).or_insert(vec![]);
    fork.insert(0, missing_block)
}