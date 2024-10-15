/*
    *Chain*:
    - Chain and chain error types
    - Methods for accessing, mining, extending, and validating a chain's blocks with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};
use super::block::{Block::{self}, BlockErr};
use std::collections::HashMap;

// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    IsFork,                        // chain doesn't start from idx 0
    InvalidSubChain(NextBlockErr), // error between two contiguous blocks in the chain
}

// For validating forks of chains
#[derive(Debug)]
pub enum ForkErr {
    ForkEmpty,
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
// pub struct Chain {
//     pub main_chain : Vec<Block>,
//     pub forks : HashMap<String, Chain>
// }
pub struct Chain ( Vec<Block> );

impl Chain {
    // New chain with a single genesis block
    pub fn genesis() -> Self {
        Self (vec![Block::genesis()])
    }

    pub fn get_tip(&self) -> &Block {
        self.0.last().expect("chain must be non-empty")
    }

    pub fn get_block_by_idx(&self, idx: usize) -> Option<&Block> {
        self.0.get(idx)
    }

    pub fn get_block_by_hash(&self, hash: &String) -> Option<&Block> {
        self.0.iter().find(|b: &&Block| b.hash == *hash)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn truncate(&mut self, len: usize){
        self.0.truncate(std::cmp::min(self.len()-1, 0));
    }

    pub fn mine_then_push_block(&mut self, data: &str) {
        let b: Block = self.mine_new_block(data);
        self.handle_new_block(&b).expect("can push newly mined block")
    }

    // Mine a new valid block from given data
    pub fn mine_new_block(&mut self, data: &str) -> Block {
        let current_block: &Block = self.get_tip();
        Block::mine_block(current_block.idx + 1, data, &current_block.hash)
    }

    // Try to append an arbitrary block to the main chain
    pub fn handle_new_block(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
        let current_block: &Block = self.get_tip();
        Self::validate_next_block(current_block, &new_block)?;
        /*
                    TO-DO: handle forks
        */
        self.0.push(new_block.clone());
        Ok(())
    }

    // Try to attach a fork (suffix of a full chain) to extend any compatible parent block in the current chain
    // Note: Can succeed even if resulting in a shorter chain.
    pub fn try_merge_fork(&mut self, fork: &mut Vec<Block>) -> Result<(), ForkErr>{
        if let Err(e) = Self::validate_subchain(&fork) {
            return Err(ForkErr::InvalidSubChain(e))
        }

        let fork_head = fork.get(0).ok_or(ForkErr::ForkEmpty)?; //expect("fork must be non-empty");
        if fork_head.idx == 0 {
            return Err(ForkErr::ForkStartsAtGenesis)
        }
        /* this should behave the same:
            ```
            match self.get_block_by_idx(&fork_head.idx - 1) {
                Some(forkpoint) if (forkpoint.hash == fork_head.prev_hash) => {
            ```
        */
        match self.get_block_by_hash(&fork_head.prev_hash) {
            // if fork branches off from idx n, then keep the first n + 1 blocks
            Some(forkpoint) => {
                self.0.truncate(forkpoint.idx + 1);
                self.0.append(fork);
                Ok(())
            }
            // fork's first block doesn't reference a block in the current chain.
            None => {
                Err(ForkErr::ForkIncompatible)
            }
        }
    }


    /*
       TO-DO: Store a block as part of a fork
    // pub fn store_fork(&mut self, block: Block) {
    //     ...
    // }

    */

    // Validate chain from head to tail, expecting it to begin at idx 0
    pub fn validate_chain(chain: &Chain) -> Result<(), ChainErr> {
        if chain.0.get(0).expect("chain must be non-empty").idx != 0 {
            return Err(ChainErr::IsFork)
        };
        Self::validate_subchain(&chain.0)
            .map_err(|e: NextBlockErr| ChainErr::InvalidSubChain(e))?;
        Ok(())
    }

    // Validate subchain from head to tail
    pub fn validate_subchain(subchain: &Vec<Block>) -> Result<(), NextBlockErr> {
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
            if self.0.len() >= remote.0.len() {
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
        for (_, block) in self.0.iter().enumerate() {
            writeln!(f, "{}", block )?;
        }
        writeln!(f)
    }
}


/******************
      TESTS
********************/
#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{
        chain::{Chain, ChainErr, ForkErr, NextBlockErr},
        block::Block,
        cryptutil::debug};

    const CHAIN_LEN : usize = 5;
    const FORK_PREFIX_LEN : usize = 3;

    /*****************************
     * Tests for handling new blocks *
    *****************************/
    #[test]
    fn test_valid_next_block() {
        let mut chain: Chain = Chain::genesis();
        let next_block = chain.mine_new_block(&format!("next valid block"));

        assert!(matches!(
            debug(chain.handle_new_block(&next_block))
            , Ok(())));
    }
    #[test]
    fn test_valid_chain() {
        let mut chain: Chain = Chain::genesis();
        for i in 1 .. CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        assert!(matches!(
            debug(Chain::validate_chain(&chain)),
            Ok(())));
    }
    #[test]
    fn test_block_too_old() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i))
        }
        // handle an old block from the current chain that is one block older than the tip
        let out_of_date_block: Block = chain.get_block_by_idx(chain.get_tip().idx - 1).unwrap().clone();
        // chain: [0]---[1]---[2]---[3]---[4]
        // old:                     [*3*]
        assert!(matches!(
            debug(chain.handle_new_block(&out_of_date_block)),
            Err(NextBlockErr::BlockTooOld { .. })
        ));
    }
    #[test]
    fn test_duplicate_block() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.mine_then_push_block(&format!("block {}", i))
        }
        // handle a duplicate block from the current chain that is the same as the tip
        let duplicate_block: Block = chain.get_tip().clone();
        // chain: [0]---[1]---[2]---[3]---[4]
        // dup:                           [*4*]
        assert!(matches!(
            debug(chain.handle_new_block(&duplicate_block)),
            Err(NextBlockErr::DuplicateBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // handle an alternative mined block from the current chain that has the same parent as the tip
        let competing_block: Block = Block::mine_block(
            chain.get_tip().idx,
            "competing block at {}",
            &chain.get_tip().prev_hash);
        // chain: [0]---[1]---[2]---[3]---[4]
        // competing block:          |----[*4*]
        assert!(matches!(
            debug(chain.handle_new_block(&competing_block)),
            Err(NextBlockErr::CompetingBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // handle a competing block from a forked_chain that is the same length as the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i))
        }
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[*4*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.get_tip())),
            Err(NextBlockErr::CompetingBlockInFork { .. })
        ));
    }
    #[test]
    fn test_next_block_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // handle the next expected block from a forked_chain that is one block longer than the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 1 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[*5*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.get_tip())),
            Err(NextBlockErr::NextBlockInFork { .. })
        ));
    }
    #[test]
    fn test_block_too_new() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // handle the latest block from a duplicate chain that is 2 blocks longer than the current chain
        let mut dup_chain: Chain = chain.clone();
        dup_chain.mine_then_push_block("next block in dup chain");
        dup_chain.mine_then_push_block("next block in dup chain");
        // chain:      [0]---[1]---[2]---[3]---[4]
        // duplicate:                           |---[5]---[*6*]
        assert!(matches!(
            debug(chain.handle_new_block(dup_chain.get_tip())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }
    #[test]
    fn test_block_too_new_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // handle the latest block from a forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // chain:   [0]---[1]---[2]---[3]---[4]
        // fork:                 |----[3]---[4]---[5]---[*6*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.get_tip())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }


    /*****************************
     * Tests for merging forks *
    *****************************/
    #[test]
    fn test_valid_fork_longer(){
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // make a competing forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain: Chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain
        let mut fork = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN);
            forked_chain.0
        };
        // Before:
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[6]
        println!("Chain : {}\n\nFork suffix : {:?}\n", chain, fork);
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork)),
            Ok(())
        ));
        println!("Merged chain and fork : {}", chain);
        // After:
        // chain: [0]---[1]---[2]
        //                     |----[3]---[4]---[5]---[6]

    }
    #[test]
    fn test_valid_fork_shorter() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // make a competing forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain: Chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // then make the current chain 2 blocks longer than the forked_chain
        for i in CHAIN_LEN .. forked_chain.len() + 2 {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // strip the common prefix between the current and forked chain
        let mut fork = forked_chain.0.split_off(FORK_PREFIX_LEN);
        // Before:
        // chain: [0]---[1]---[2]---[3]---[4]---[5]---[6]---[7]---[8]
        // fork:               |----[3]---[4]---[5]---[6]
        println!("Chain : {}\n\nFork suffix : {:?}\n", chain, fork);
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork)),
            Ok(())
        ));
        println!("Merged chain and fork : {}", chain);
        // After:
        // chain: [0]---[1]---[2]
        //                     |----[3]---[4]---[5]---[6]
    }
    #[test]
    fn test_fork_starts_at_genesis() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // try to merge the entire forked chain  rather than the fork.
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:  [0]---[1]---[2]---[3]---[4]---[5]---[6]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut forked_chain.0)),
            Err(ForkErr::ForkStartsAtGenesis{ .. })
        ));
    }
    #[test]
    fn test_fork_incompatible() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // make a competing forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain, then **remove the first block** from the fork
        let mut incompatible_fork: Vec<Block> = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN + 1);
            forked_chain.0
        };
        // try to merge a fork that is missing a reference to the current chain:
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[?]---[4]---[5]---[6]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut incompatible_fork)),
            Err(ForkErr::ForkIncompatible{ .. })
        ));
    }
    #[test]
    fn test_fork_invalid_subchain() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain, and then **mutate a block** in the fork
        let mut fork_invalid_subchain: Vec<Block> = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN);
            let b: &mut Block = forked_chain.0.last_mut().unwrap();
            b.data = "corrupt data".to_string();
            forked_chain.0
        };
        // try to merge a fork that is corrupt subchain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[X]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork_invalid_subchain)),
            Err(ForkErr::InvalidSubChain{ .. })
        ));
    }

}
