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
    ChainIsFork,
    InvalidSubChain(NewBlockErr),
}

// For validating whether a new block can be added to a blockchain system.
#[derive(Debug, Clone)]
pub enum NewBlockErr {
    InvalidBlock(BlockErr),
    MissingParent {
        idx: usize,
        parent_hash: String
    }, // Parent doesn't exist in any chain
    InvalidPosition {
        idx: usize,
        parent_hash: String,
        parent_idx: usize
    }, // Parent exists but in an inconsistent position in the chain
}

impl std::fmt::Display for NewBlockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NewBlockErr::InvalidBlock(e) => {
                write!(f, "{}.", e)
            }
            NewBlockErr::MissingParent { idx, parent_hash, current_idx, current_hash } => {
                write!(f, "Block index {} with parent hash {} is not a valid child of Block index {} with hash {}", block_idx, block_parent_hash, current_idx, current_hash)
            }
        }
    }
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
    InvalidSubChain(BlockErr),
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

fn find_block<'a>(blocks: &'a Vec<Block>, block_hash: &String) -> Option<&'a Block> {
    blocks.iter().find(|block| &block.hash == block_hash)
}

// Check if block is in any fork, returning the fork point, end hash, and fork
fn find_fork<'a>(forks: &'a HashMap<String, HashMap<String, Vec<Block>>>, hash: &String)
    -> Option<(String, String, &'a Vec<Block>, &'a Block)> {
    // iterate through fork points
    for (fork_point, forks_from) in forks {
        // iterate through forks from the fork point
        for (end_hash, fork) in forks_from {
            // iterate through blocks in the fork
            if let Some(block) = find_block(fork, hash) {
                return Some((fork_point.clone(), end_hash.clone(), fork, block))
            }
        }
    }
    None
}

fn unsafe_push_block(blocks: &mut Vec<Block>, new_block: &Block){
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

// Validating whether one block is a valid next block for another.
fn validate_next_block(current_block: &Block, block: &Block) -> Result<(), BlockErr> {
    // validity of block by itself
    Block::validate_block(block)?;
    // if     1) the block's idx doesn't follow the current block's
    // and/or 2) it isn't a child of the current block (i.e. belongs to a fork or different chain).
    if block.idx != current_block.idx + 1 ||  block.prev_hash != current_block.hash {
        return Err(BlockErr::InvalidBlockParent
            {   block_idx: block.idx,
                block_parent_hash: block.prev_hash.to_string(),
                current_idx: current_block.idx,
                current_hash: current_block.hash.to_string()
            })
    }
    Ok(())
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

    pub fn handle_new_block(&mut self, block: Block) -> Result<(), BlockErr>{
        let block_prev_hash = block.prev_hash.clone();
        // Search for the parent block in the main chain.
        if let Some(parent_block) = find_block(&self.main, &block_prev_hash){

            validate_next_block(parent_block, &block)?;
            println!("Found parent block in main chain.");

            // See if we can append the block to the main chain
            if self.last().hash == parent_block.hash {
               unsafe_push_block(&mut self.main, &block);
               println!("Extending the main chain");
               Ok(())
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                // need to manually validate it to create a single-block chain
                let forks_from: &mut HashMap<String, Vec<Block>>
                    = self.forks.entry(block_prev_hash).or_insert(HashMap::new());
                forks_from.insert(block.hash.to_string() // end hash
                                , vec![block]);
                println!("Adding a single-block fork to the main chain");
                Ok (())
            }
        }
        // Search for the parent block in the forks.
        else if let Some((forkpoint_hash, endpoint_hash,  fork, parent_block))
                 = find_fork(&self.forks, &block_prev_hash) {

            println!("Found parent block in a fork");
            validate_next_block(parent_block, &block)?;

            // See if we can append the block to the fork
            if endpoint_hash == parent_block.hash {
                unsafe_push_block(&mut fork, &block);
                // Update the endpoint_hash of the extended fork in the map.
                self.forks.entry(forkpoint_hash).and_modify(|forks| {
                    let fork: Vec<Block> = forks.remove(&endpoint_hash).expect("Fork definitely exists; we just pushed a block to it.");
                    forks.insert(block.hash, fork);
                });
                println!("Extending an existing fork");
                Ok(())
            }
            // Otherwise create a new direct fork from the main chain, whose prefix is a clone of an existing fork's, with
            else {
                // Truncate the fork until the block's parent,
                let mut truncated_fork: Vec<Block> = {
                    let fork_clone = fork.clone();
                    truncate_until(&mut fork_clone, |block| block.hash == block_prev_hash);
                    fork_clone
                }
                // Push the new block on
                unsafe_push_block(&mut truncated_fork, &block);
                // Insert the new fork into the map.
                self.forks.entry(forkpoint_hash).and_modify(|forks| {
                    forks.insert(block_prev_hash, truncated_fork);
                });
                println!("Adding a new fork that branches off an existing fork to the chain");
                Ok(())
            }
        }
        else {
            /* TO-DO:
            What error should happen here?
             */
            Err(BlockErr::MissingBlock { block_idx: block.idx, block_parent_hash: block.prev_hash })
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
        unsafe_push_block(&mut self.main, &b);
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
        let mut curr: &Block = subchain.get(0)
            .ok_or_else(|| NextBlockErr::UnknownError)?;
        for i in 0..subchain.len() - 1 {
            let next: &Block = subchain.get(i + 1)
                .ok_or_else(|| NextBlockErr::UnknownError)?;
            if let Err(e) = validate_next_block(curr, next) {
                return Err(e);
            }
            else {
                curr = next;
            }
        }
        Ok(())
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

    // Try to attach a fork to extend any compatible parent block in the current chain. (Can succeed even if resulting in a shorter chain.)
    //  - Not currently being used outside of testing.
    pub fn try_merge_fork(&mut self, fork: &mut Vec<Block>) -> Result<(), ForkErr>{
        let fork_head: &Block = fork.get(0).ok_or(ForkErr::ForkIsEmpty)?;
        Self::validate_fork(&fork)?;

        /* this should behave the same:
            match self.get(&fork_head.idx - 1) {
                Some(forkpoint) if (forkpoint.hash == fork_head.prev_hash) => {
        */
        match self.lookup(&fork_head.prev_hash) {
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
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (_, block) in self.main.iter().enumerate() {
            writeln!(f, "{}", block )?;
        };
        Ok(())
    }
}
