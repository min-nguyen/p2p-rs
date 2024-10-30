/*
    *Forks and orphans*: Auxiliary helpers for managing forks and orphans, independent of a chain.
*/

use super::
    block::{Block, NextBlockResult, NextBlockErr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Forks are represented as a set of forkpoints (from the main chain) from which various branches arise and
// share common prefixes of blocks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forks(HashMap<String, HashMap<String, Vec<Block>>>); // <fork point, <fork end hash, forked blocks>>

impl Forks {
    pub fn new() -> Self {
        Forks(HashMap::new())
    }

    pub fn validate(fork: &Vec<Block>) -> Result<ForkId, NextBlockErr>{
        Block::validate_blocks(fork)?;
        let ((fork_hash, fork_idx), (end_hash, end_idx))
            = ( {let first_block = fork.first().unwrap();
                    (first_block.prev_hash.clone(), first_block.idx - 1)
                },
                { let end_block = fork.last().unwrap();
                    (end_block.hash.clone(), end_block.idx)
                });
        Ok (ForkId {fork_hash, fork_idx, end_hash, end_idx})

    }

    pub fn find<'a, P>(&'a self, prop: P) -> Option<(ForkId, &'a Vec<Block>, &'a Block)>
    where
    P: Fn(&Block) -> bool  {
        // iterate through fork points
        for (_, forks_from) in self.0.iter() {
            // iterate through forks from the fork point
            for (_, fork) in forks_from {
                // iterate through blocks in the fork
                if let Some(b) = Block::find(&fork, &prop) {
                    let fork_id = Self::validate(fork).unwrap();
                    return Some((fork_id, &fork, &b))
                }
            }
        }
        None
    }

    pub fn get<'a>(&'a self, forkpoint: &String, endpoint: &String) -> Option<&'a  Vec<Block>>{
        self.0.get(forkpoint).and_then(|forks| forks.get(endpoint))
    }

    pub fn get_mut<'a>(&'a mut self, forkpoint: &String, endpoint: &String) -> Option<(&'a mut Vec<Block>, ForkId)>{
        self.0.get_mut(forkpoint)
            .and_then(|forks| {
                forks.get_mut(endpoint)
                    .map(|fork| {
                        let fork_id = Self::validate(fork);
                        (fork, fork_id.unwrap())
                    }
            )})
    }

    pub fn remove_entry<'a>(&'a mut self, forkpoint: &String, endpoint: &String) -> Option<Vec<Block>>{
        self.0.get_mut(forkpoint)
                .and_then(|forks|
                    forks.remove_entry(endpoint).map(|res| res.1))
    }

    pub fn insert(&mut self, fork: Vec<Block>) -> Result<ForkId, NextBlockErr>{
        let fork_id = Forks::validate(&fork)?;

        self.0.entry(fork_id.fork_hash.clone())
                    .or_insert(HashMap::new())
                    .insert(fork_id.end_hash.clone(), fork);

        Ok (fork_id)
    }

    pub fn extend_fork(&mut self, forkpoint: &String, endpoint: &String, block : Block) -> Result<ForkId, NextBlockErr> {
        let mut fork: Vec<Block> = self.remove_entry(forkpoint, endpoint).unwrap();
        Block::push_end(&mut fork, block);
        self.insert(fork)
    }

    pub fn nest_fork(&mut self, forkpoint: &String, endpoint: &String, block: Block) -> Result<ForkId, NextBlockErr> {
        let mut fork: Vec<Block> = self.get(forkpoint, endpoint).unwrap().clone();
        Block::split_off_until(&mut fork, |b| b.hash == block.prev_hash);
        Block::push_end(&mut fork, block);
        self.insert(fork)
    }

    pub fn print(&self){
        for (forkpoint, forks_from) in self.0.iter(){
            println!("Forks from {}", forkpoint);
            for (i, (_, fork)) in forks_from.iter().enumerate(){
                println!("Fork {}:", i);
                fork.iter().for_each(|block| println!("{}", block));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ForkId {
    pub fork_hash: String,
    pub fork_idx: usize,
    pub end_hash: String,
    pub end_idx: usize,
}

impl ForkId {
    pub fn into_extended_fork_result(self) -> NextBlockResult {
        NextBlockResult::ExtendedFork {
            fork_idx: self.fork_idx,
            fork_hash: self.fork_hash,
            end_idx: self.end_idx,
            end_hash: self.end_hash,
        }
    }

    pub fn into_new_fork_result(self) -> NextBlockResult {
        NextBlockResult::NewFork {
            fork_idx: self.fork_idx,
            fork_hash: self.fork_hash,
            end_idx: self.end_idx,
            end_hash: self.end_hash,
        }
    }
}

// Orphan branches are represented as a disjoint set of chains that are constructed backwards.
// We do not track whether each orphan branch has blocks in common i.e. are forks of each other;
// they are used to connect an orphan node back to the main chain as fast as possible, at which point it forms a fork.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orphans (HashMap<String, Vec<Block>>); // <fork point, orphaned branch>
pub type OrphanId = String; // fork hash

impl Orphans {
    pub fn new() -> Self {
        Orphans(HashMap::new())
    }

    pub fn validate(orphan: &Vec<Block>) -> Result<OrphanId, NextBlockErr>{
        Block::validate_blocks(orphan)?;
        Ok(orphan.first().unwrap().prev_hash.clone())
    }

    pub fn find<'a, P>(&'a self, prop: P) -> Option<(OrphanId, &'a Vec<Block>, &'a Block)>
        where
        P: Fn(&Block) -> bool  {
            for (forkpoint, orphan) in self.0.iter() {
                if let Some(b) = Block::find(&orphan, &prop) {
                    return Some((forkpoint.clone(), &orphan, &b))
                }
            }
        None
    }

    pub fn get<'a>(&'a self, forkpoint: &String) -> Option<&'a Vec<Block>>{
        self.0.get(forkpoint)
    }

    pub fn get_mut<'a>(&'a mut self, forkpoint: &String) -> Option<&'a mut Vec<Block>>{
        self.0.get_mut(forkpoint)
    }

    pub fn insert(&mut self, orphan: Vec<Block>) -> Result<OrphanId, NextBlockErr> {
        let fork_id = Orphans::validate(&orphan)?;
        self.0.insert(fork_id.clone(), orphan);
        Ok(fork_id)
    }

    pub fn remove_entry<'a>(&mut self, forkpoint : &String) -> Option<Vec<Block>>{
        self.0.remove_entry(forkpoint).map(|res| res.1)
    }

    pub fn extend_orphan(&mut self, block : Block) -> Result<OrphanId, NextBlockErr>  {
        let mut orphan: Vec<Block> = self.remove_entry(&block.hash).unwrap();
        Block::push_front(&mut orphan, block.clone());
        self.insert(orphan)
    }

    pub fn print(&self){
        for (i, orphan) in self.0.iter().enumerate(){
            println!("Orphaned branch {}:\n\t{:?}\n", i, orphan.1);
        }
    }
}
