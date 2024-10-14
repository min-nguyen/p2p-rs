/*
    *Chain*:
    - Chain and chain error types
    - Methods for accessing, mining, extending, and validating a chain's blocks with respect to other blocks, chains, or forks.
*/

use serde::{Deserialize, Serialize};
use super::block::{Block::{self}, BlockErr};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain (pub Vec<Block>);

// For validating full chains
#[derive(Debug)]
pub enum ChainErr {
    IsFork,                        // chain doesn't start from idx 0
    InvalidSubChain(NextBlockErr), // error between two contiguous blocks in the chain
}

// For validating forks of chains
#[derive(Debug)]
pub enum ForkErr {
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

    // Mine a new valid block from given data, and extend the chain by it
    pub fn make_new_valid_block(&mut self, data: &str) {
        let current_block: &Block = self.get_tip();
        let new_block: Block = Block::mine_block(current_block.idx + 1, data, &current_block.hash);
        self.try_push_block(&new_block).expect("returned mined block isn't valid")
    }

    // Try to append an arbitrary block
    pub fn try_push_block(&mut self, new_block: &Block) -> Result<(), NextBlockErr>{
        let current_block: &Block = self.get_tip();
        Self::validate_next_block(current_block, &new_block)?;
        self.0.push(new_block.clone());
        Ok(())
    }

    // Try to attach a fork (suffix of a full chain) to extend any compatible parent block in the current chain
    // Note: Can succeed even if resulting in a shorter chain.
    pub fn try_merge_fork(&mut self, fork: &mut Chain) -> Result<(), ForkErr>{
        if let Err(e) = Self::validate_subchain(&fork) {
            return Err(ForkErr::InvalidSubChain(e))
        }

        let fork_head = fork.get_block_by_idx(0).expect("fork must be non-empty");
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
                self.0.append(&mut fork.0);
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
        if chain.0.get(0).expect("chain must be non-empty").idx != 0 {
            return Err(ChainErr::IsFork)
        }
        Ok(())
    }

    // Validate subchain from head to tail, ignoring the first block
    pub fn validate_subchain(chain: &Chain) -> Result<(), NextBlockErr> {
        for i in 0..chain.0.len() - 1 {
            let curr: &Block = chain.0.get(i)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            let next: &Block = chain.0.get(i + 1)
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
    pub fn sync_chain(&mut self, remote: &Chain) -> bool {
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
            write!(f, "{}", block )?;
        }
        writeln!(f)
    }
}

/********  TESTS **********/
#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{
        chain::{Chain, ChainErr, ForkErr, NextBlockErr},
        block::Block,
        cryptutil::debug};

    const CHAIN_LEN : usize = 5;
    const FORK_PREFIX_LEN : usize = 3;

    #[test]
    fn test_block_too_old() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }
        let out_of_date_block: Block = chain.get_block_by_idx(chain.get_tip().idx - 1).unwrap().clone();

        assert!(matches!(
            debug(chain.try_push_block(&out_of_date_block)),
            Err(NextBlockErr::BlockTooOld { .. })
        ));
    }
    #[test]
    fn test_duplicate_block() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }
        let duplicate_block: Block = chain.get_tip().clone();

        assert!(matches!(
            debug(chain.try_push_block(&duplicate_block)),
            Err(NextBlockErr::DuplicateBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }
        // mine an alternative block at the same position as the current one
        let competing_block: Block = Block::mine_block(
            chain.get_tip().idx,
            "competing block at {}",
            &chain.get_tip().prev_hash);

        assert!(matches!(
            debug(chain.try_push_block(&competing_block)),
            Err(NextBlockErr::CompetingBlock { .. })
        ));
    }
    #[test]
    fn test_competing_block_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // make competing forked_chain the same length as the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(forked_chain.get_tip())),
            Err(NextBlockErr::CompetingBlockInFork { .. })
        ));
    }
    #[test]
    fn test_next_block_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // make competing forked_chain one block longer than the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 1 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(forked_chain.get_tip())),
            Err(NextBlockErr::NextBlockInFork { .. })
        ));
    }
    #[test]
    fn test_block_too_new() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // create a matching chain that is 2 blocks longer than the current chain
        let mut dup_chain: Chain = chain.clone();
        dup_chain.make_new_valid_block("next block in dup chain");
        dup_chain.make_new_valid_block("too-far-ahead block in dup chain");

        assert!(matches!(
            debug(chain.try_push_block(dup_chain.get_tip())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }
    #[test]
    fn test_block_too_new_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // make competing forked_chain 2 blocks longer than the current chain
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        assert!(matches!(
            debug(chain.try_push_block(forked_chain.get_tip())),
            Err(NextBlockErr::BlockTooNew { .. })
        ));
    }
    #[test]
    fn test_valid_next_block() {
        let mut chain: Chain = Chain::genesis();
        let current_block: Block = chain.get_tip().clone();

        chain.make_new_valid_block(&format!("test next valid block"));
        let next_valid_block = chain.get_tip();

        assert!(matches!(
            debug(Chain::validate_next_block(&current_block, &next_valid_block))
            , Ok(())));
    }
    #[test]
    fn test_valid_chain() {
        let mut chain: Chain = Chain::genesis();
        for i in 1 .. CHAIN_LEN {
          chain.make_new_valid_block(&format!("next valid block {}", i));
        }
        assert!(matches!(
            debug(Chain::validate_chain(&chain)),
            Ok(())));
    }
    #[test]
    fn test_valid_fork_longer(){
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut forked_chain: Chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        // strip the common prefix between the current and forked chain
        let mut fork = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN);
            forked_chain
        };

        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[6]
        println!("Chain : {}\n\nFork suffix : {}\n", chain, fork);
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork)),
            Ok(())
        ));
        println!("Merged chain and fork : {}", chain);
        // chain: [0]---[1]---[2]
        //                     |----[3]---[4]---[5]---[6]

    }
    #[test]
    fn test_valid_fork_shorter() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }
        // make competing forked_chain 2 blocks longer than the current chain
        let mut forked_chain: Chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }
        // make current chain 2 blocks longer than the forked_chain chain
        for i in CHAIN_LEN .. forked_chain.len() + 2 {
            chain.make_new_valid_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain
        let mut fork = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN);
            forked_chain
        };

        // chain: [0]---[1]---[2]---[3]---[4]---[5]---[6]---[7]---[8]
        // fork:               |----[3]---[4]---[5]---[6]
        println!("Chain : {}\n\nFork suffix : {}\n", chain, fork);
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork)),
            Ok(())
        ));
        println!("Merged chain and fork : {}", chain);
        // chain: [0]---[1]---[2]
        //                     |----[3]---[4]---[5]---[6]
    }
    #[test]
    fn test_fork_starts_at_genesis() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        // try to merge the entire forked chain, rather than the fork.
        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:  [0]---[1]---[2]---[3]---[4]---[5]---[6]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut forked_chain)),
            Err(ForkErr::ForkStartsAtGenesis{ .. })
        ));
    }
    #[test]
    fn test_fork_incompatible() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        // strip the common prefix between the current and forked chain, plus one more block
        let mut incompatible_fork: Chain = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN + 1);
            forked_chain
        };

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
            chain.make_new_valid_block(&format!("block {}", i));
        }

        let mut forked_chain = Chain(chain.0.clone()[..FORK_PREFIX_LEN].to_vec());
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.make_new_valid_block(&format!("block {} in fork", i));
        }

        // strip the common prefix between the current and forked chain, and mutate a block
        let mut fork_invalid_subchain: Chain = {
            forked_chain.0.drain(0 ..FORK_PREFIX_LEN);
            let b: &mut Block = forked_chain.0.last_mut().unwrap();
            b.data = "corrupt data".to_string();
            forked_chain
        };

        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[X]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut fork_invalid_subchain)),
            Err(ForkErr::InvalidSubChain{ .. })
        ));
    }
}
