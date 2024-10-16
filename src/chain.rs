/*
    *Chain*:
    - Chain, a safe wrapper around a vector of blocks, and error types
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};
use super::block::{Block::{self}, BlockErr};
use std::collections::HashMap;

// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    ChainIsEmpty,
    ChainIsFork,                   // chain doesn't start from idx 0
    InvalidSubChain(NextBlockErr), // error between two contiguous blocks in the chain
}

// For validating forks of chains
#[derive(Debug)]
pub enum ForkErr {
    ForkIsEmpty,
    ForkStartsAtGenesis,            // fork's first block has index == 0
    ForkIncompatible,               // fork's first block's parent hash doesn't match any block in the current chain
    InvalidSubChain(NextBlockErr),  // error between two contiguous blocks in the chain
}

// For validating whether one block is a valid next block for another.
#[derive(Debug)]
pub enum NextBlockErr {
    InvalidBlock(BlockErr),      // error from block's self-validation
    BlockTooOld {
        block_idx: usize,
        current_idx: usize
    },                           // block is out-of-date
    DuplicateBlock {
        block_idx: usize
    },                           // competing block is a duplicate
    CompetingBlock {
        block_idx: usize,
        block_parent_hash: String
    },                           // competing block has same parent
    CompetingBlockInFork {
        block_idx: usize,
        block_parent_hash: String,
        current_parent_hash: String
    },                           // competing block has different parent, belonging to a fork (or different chain)
    NextBlockInFork {
        block_idx: usize,
        block_parent_hash: String,
        current_hash: String
    },                           // next block's parent doesn't match the current block, belonging to a fork (or different chain)
    BlockTooNew {
        block_idx: usize,
        current_idx: usize
    },                           // block is ahead by more than 1 block
    UnknownError,                // non-exhaustive case (should not happen)
}

impl std::fmt::Display for NextBlockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NextBlockErr::InvalidBlock(err) => {
                write!(f, "Invalid block encountered: {:?}", err)
            }
            NextBlockErr::BlockTooOld { block_idx, current_idx } => {
                write!(f, "Block {} is too old compared to current block {}.", block_idx, current_idx)
            }
            NextBlockErr::DuplicateBlock { block_idx } => {
                write!(f, "Duplicate block encountered: Block {} is already in the chain.", block_idx)
            }
            NextBlockErr::CompetingBlock { block_idx, block_parent_hash } => {
                write!(f, "Competing block detected: Block {} with parent hash {} is competing.", block_idx, block_parent_hash)
            }
            NextBlockErr::CompetingBlockInFork { block_idx, block_parent_hash, current_parent_hash } => {
                write!(f, "Competing block in fork detected: Block {} with parent hash {} competing against current parent hash {}.",
                    block_idx, block_parent_hash, current_parent_hash)
            }
            NextBlockErr::NextBlockInFork { block_idx, block_parent_hash, current_hash } => {
                write!(f, "Next block in fork detected: Block {} with parent hash {} does not match current block hash {}.",
                    block_idx, block_parent_hash, current_hash)
            }
            NextBlockErr::BlockTooNew { block_idx, current_idx } => {
                write!(f, "Block {} is too new compared to current block {}.", block_idx, current_idx)
            }
            NextBlockErr::UnknownError => {
                write!(f, "An unknown error occurred while trying to push the block.")
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main : Vec<Block>
}

impl Chain {
    // New chain with a single genesis block
    pub fn genesis() -> Self {
        Self { main : vec![Block::genesis()] }
    }

    // Safely construct a chain from a vector of blocks
    pub fn from_vec(blocks: Vec<Block>) -> Result<Chain, ChainErr> {
        let chain = Chain{main : blocks};
        Self::validate_chain(&chain)?;
        Ok(chain)
    }

    pub fn get(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
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

    pub fn get_block_by_hash(&self, hash: &String) -> Option<&Block> {
        self.main.iter().find(|b: &&Block| b.hash == *hash)
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.main.clone()
    }

    // Mine a new valid block from given data
    pub fn mine_new_block(&mut self, data: &str) -> Block {
        let current_block: &Block = self.last();
        Block::mine_block(current_block.idx + 1, data, &current_block.hash)
    }

    // Try to append an arbitrary block to the main chain
    pub fn handle_new_block(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
        let current_block: &Block = self.last();
        Self::validate_next_block(current_block, &new_block)?;
        /*
            TO-DO: possibly handle forks inside here
        */
        self.main.push(new_block.clone());
        Ok(())
    }

    pub fn mine_then_push_block(&mut self, data: &str) {
        let b: Block = self.mine_new_block(data);
        self.handle_new_block(&b).expect("can push newly mined block")
    }

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
