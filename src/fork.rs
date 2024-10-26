/*
    *Chain*:
    - Chain internals, a safe wrapper that manages a main chain and a hashmap of forks.
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use super::block::{Block::{self}, NextBlockErr};
use std::collections::HashMap;

pub type Forks = HashMap<String, HashMap<String, Vec<Block>>>;

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn find_fork<'a>(forks: &'a Forks, hash: &String) -> Option<&'a Vec<Block>> {
    // iterate through fork points
    for (_, forks_from) in forks.iter() {
        // iterate through forks from the fork point
        for (_, fork) in forks_from {
            // iterate through blocks in the fork
            if let Some(_) = Block::find(&fork, hash) {
                return Some(&fork)
            }
        }
    }
    None
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn find_fork_mut<'a>(forks: &'a mut Forks, hash: &String) -> Option<(String, String, &'a mut Vec<Block>)> {
    // iterate through fork points
    for (fork_point, forks_from) in forks.iter_mut() {
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
pub fn lookup_fork<'a>(forks: &'a Forks, forkpoint: &String, endpoint: &String) -> Option<&'a  Vec<Block>>{
    forks.get(forkpoint).and_then(|forks| forks.get(endpoint))
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn lookup_fork_mut<'a>(forks: &'a mut Forks, forkpoint: &String, endpoint: &String) -> Option<&'a mut Vec<Block>>{
    forks.get_mut(forkpoint).and_then(|forks| forks.get_mut(endpoint))
}

// Check if block is in any fork, returning the fork point, end hash, and fork
pub fn remove_fork<'a>(forks: &mut Forks,  forkpoint: &String, endpoint: &String) -> Option<Vec<Block>>{
    forks.get_mut(forkpoint).and_then(|forks| forks.remove_entry(endpoint).map(|res| res.1))
}

// Store a valid fork (replacing any existing one), returning its forkpoint, endpoint, and last block's index
pub fn insert_fork(forks: &mut Forks, fork: Vec<Block>) -> Result<(String, String), NextBlockErr>{
    let ((forkpoint, _), (endpoint, _), _) = identify_fork(&fork)?;

    forks.entry(forkpoint.clone())
                .or_insert(HashMap::new())
                .insert(endpoint.clone(), fork);

    Ok ((forkpoint, endpoint))
}

// Store a valid fork (replacing any existing one), returning its forkpoint, endpoint, and last block's index
pub fn identify_fork(fork: &Vec<Block>) -> Result<((String, usize), (String, usize), usize), NextBlockErr>{
    if fork.is_empty() {
        Err(NextBlockErr::NoBlocks)
    }
    else {
        let (forkpoint, endpoint, fork_len)
            = ( {let first_block = fork.first().unwrap();
                    (first_block.prev_hash.clone(), first_block.idx - 1)
                },
                { let end_block = fork.last().unwrap();
                    (end_block.hash.clone(), end_block.idx)
                },
                fork.len());
        Ok ((forkpoint, endpoint, fork_len))
    }
}
