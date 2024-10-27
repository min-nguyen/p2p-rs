/*
    *Fork*: Auxiliary helpers for managing forks, independent of a chain.
*/

use super::block::{Block::{self}, NextBlockErr};
use std::collections::HashMap;

// <fork point, <fork end hash, forked blocks>>
pub type Forks = HashMap<String, HashMap<String, Vec<Block>>>;

#[derive(Clone, Debug)]
pub struct ForkId {
    pub fork_hash: String,
    pub fork_idx: usize,
    pub end_hash: String,
    pub end_idx: usize,
    pub length: usize
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn find_fork<'a, P>(forks: &'a Forks, prop: P) -> Option<(&'a Vec<Block>, ForkId)>
    where
    P: Fn(&Block) -> bool  {
    // iterate through fork points
    for (_, forks_from) in forks.iter() {
        // iterate through forks from the fork point
        for (_, fork) in forks_from {
            // iterate through blocks in the fork
            if let Some(_) = Block::find(&fork, &prop) {
                let fork_id = identify_fork(fork).unwrap();
                return Some((&fork, fork_id))
            }
        }
    }
    None
}

// Store a valid fork (replacing any existing one), returning its forkpoint, endpoint, and last block's index
pub fn identify_fork(fork: &Vec<Block>) -> Result<ForkId, NextBlockErr>{
    if fork.is_empty() {
        Err(NextBlockErr::NoBlocks)
    }
    else {
        let ((fork_hash, fork_idx), (end_hash, end_idx), length)
            = ( {let first_block = fork.first().unwrap();
                    (first_block.prev_hash.clone(), first_block.idx - 1)
                },
                { let end_block = fork.last().unwrap();
                    (end_block.hash.clone(), end_block.idx)
                },
                fork.len());
        Ok (ForkId {fork_hash, fork_idx, end_hash, end_idx, length})
    }
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn lookup_fork<'a>(forks: &'a Forks, forkpoint: &String, endpoint: &String) -> Option<&'a  Vec<Block>>{
    forks.get(forkpoint).and_then(|forks| forks.get(endpoint))
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn lookup_fork_mut<'a>(forks: &'a mut Forks, forkpoint: &String, endpoint: &String) -> Option<(&'a mut Vec<Block>, ForkId)>{
    forks.get_mut(forkpoint)
        .and_then(|forks| {
            forks.get_mut(endpoint)
                .map(|fork| {
                    let fork_id = identify_fork(fork);
                    (fork, fork_id.unwrap())
                }
        )})
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn remove_fork<'a>(forks: &mut Forks,  forkpoint: &String, endpoint: &String) -> Option<Vec<Block>>{
    forks.get_mut(forkpoint).and_then(|forks| forks.remove_entry(endpoint).map(|res| res.1))
}

// Store a valid fork (replacing any existing one), returning its forkpoint, endpoint, and last block's index
pub fn insert_nonempty_fork(forks: &mut Forks, fork: Vec<Block>) -> Result<ForkId, NextBlockErr>{
    let fork_id = identify_fork(&fork)?;

    forks.entry(fork_id.fork_hash.clone())
                .or_insert(HashMap::new())
                .insert(fork_id.end_hash.clone(), fork);

    Ok (fork_id)
}

pub fn extend_fork(forks: &mut Forks, fork_id: &ForkId, block : Block) -> Result<ForkId, NextBlockErr> {
    let mut fork: Vec<Block> = remove_fork(forks, &fork_id.fork_hash, &fork_id.end_hash).unwrap();
    Block::push_end(&mut fork, block);
    insert_nonempty_fork(forks, fork)
}

pub fn nest_fork(forks: &mut Forks, fork_id: &ForkId, block : Block) -> Result<ForkId, NextBlockErr> {
    let mut fork: Vec<Block> = lookup_fork(forks, &fork_id.fork_hash, &fork_id.end_hash).unwrap().clone(); //fork.clone();
    Block::split_off_until(&mut fork, |b| b.hash == block.prev_hash);
    Block::push_end(&mut fork, block);
    insert_nonempty_fork(forks, fork)
}

// <fork point, orphaned branch>
pub type Orphans = HashMap<String, Vec<Block>>;

pub type OrphanId = String; // fork point, i.e., the previous hash of the first block in an orphan

pub fn identify_orphan(orphan: &Vec<Block>) -> Result<OrphanId, NextBlockErr>{
    if orphan.is_empty() {
        Err(NextBlockErr::NoBlocks)
    }
    else {
        Ok(orphan.first().unwrap().prev_hash.clone())
    }
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn lookup_orphan<'a>(orphans: &'a HashMap<String, Vec<Block>>, forkpoint: &String) -> Option<&'a Vec<Block>>{
    orphans.get(forkpoint)
}

pub fn remove_orphan<'a>(orphans: &mut HashMap<String, Vec<Block>>, forkpoint : &String) -> Option<Vec<Block>>{
    orphans.remove_entry(forkpoint).map(|res| res.1)
}

pub fn insert_orphan(orphans: &mut HashMap<String, Vec<Block>>, orphan: Vec<Block>) -> Result<OrphanId, NextBlockErr> {
    let fork_id = identify_orphan(&orphan)?;
    orphans.insert(fork_id.clone(), orphan);
    Ok(fork_id)
}

pub fn prepend_orphan(orphans: &mut HashMap<String, Vec<Block>>, block : Block) -> Result<OrphanId, NextBlockErr>  {
    let mut orphan: Vec<Block> = orphans.remove_entry(&block.hash).map(|res| res.1).unwrap();
    Block::push_front(&mut orphan, block.clone());
    insert_orphan(orphans, orphan)
}