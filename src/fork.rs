/*
 *Forks and orphans*: Auxiliary helpers for managing forks and orphans, independent of a chain.
*/

use super::{
    block::{Block, Blocks, NextBlockErr, NextBlockResult},
    util::abbrev,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

// Forks are represented as a set of forkpoints (from the main chain) from which various branches arise and
// share common prefixes of blocks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forks(HashMap<String, HashMap<String, Blocks>>); // <fork point, <fork end hash, forked blocks>>

impl Forks {
    pub fn new() -> Self {
        Forks(HashMap::new())
    }

    pub fn identify(fork: &Blocks) -> ForkId {
        let ((fork_hash, fork_idx), (end_hash, end_idx)) = (
            {
                let first_block = fork.first();
                (first_block.prev_hash.clone(), first_block.idx - 1)
            },
            {
                let end_block = fork.last();
                (end_block.hash.clone(), end_block.idx)
            },
        );
        ForkId {
            fork_hash,
            fork_idx,
            end_hash,
            end_idx,
        }
    }

    pub fn find<'a, P>(&'a self, prop: &P) -> Option<(ForkId, &'a Blocks, &'a Block)>
    where
        P: Fn(&Block) -> bool,
    {
        // iterate through fork points
        for forks_from in self.0.values() {
            // iterate through forks from the fork point
            for fork in forks_from.values() {
                // iterate through blocks in the fork
                if let Some(b) = fork.find(prop) {
                    let fork_id = Self::identify(fork);
                    return Some((fork_id, fork, b));
                }
            }
        }
        None
    }

    pub fn get<'a>(&'a self, forkpoint: &String, endpoint: &String) -> Option<&'a Blocks> {
        self.0.get(forkpoint).and_then(|forks| forks.get(endpoint))
    }

    pub fn get_mut<'a>(
        &'a mut self,
        forkpoint: &String,
        endpoint: &String,
    ) -> Option<(&'a mut Blocks, ForkId)> {
        self.0.get_mut(forkpoint).and_then(|forks| {
            forks.get_mut(endpoint).map(|fork| {
                let fork_id = Self::identify(fork);
                (fork, fork_id)
            })
        })
    }

    // Return a reference to the longest stored fork
    pub fn longest<'a>(&'a self) -> Option<(&'a Blocks, ForkId)> {
        let longest_fork: Option<(&'a Blocks, ForkId)> = None;

        self.0
            .values()
            .flat_map(|forks| forks.values())
            .fold(longest_fork, |longest, current| match longest {
                Some(fork) if fork.0.len() >= current.len() => Some(fork),
                _ => Some((current, Self::identify(current))),
            })
    }

    pub fn retain_forkpoints(&mut self, forkpoints: &[String]) {
        // let hashes : Vec<String> = chain.iter().map(|b| b.hash.clone()).collect();
        self.0.retain(|forkpoint, _| forkpoints.contains(forkpoint));
    }

    pub fn remove(&mut self, forkpoint: &String, endpoint: &String) -> Option<Blocks> {
        // Remove the fork matching the (forkpoint, endpoint)
        let fork = self
            .0
            .get_mut(forkpoint)
            .and_then(|forks| forks.remove(endpoint));
        // If there are no remaining forks from the forkpoint, delete that hashmap
        if let Some(true) = self.0.get(forkpoint).map(|forks| forks.is_empty()) {
            self.0.remove(forkpoint);
        }
        fork
    }

    pub fn insert(&mut self, fork: Blocks) -> ForkId {
        let fork_id = Self::identify(&fork);

        self.0
            .entry(fork_id.fork_hash.clone())
            .or_default()
            .insert(fork_id.end_hash.clone(), fork);

        fork_id
    }

    pub fn extend_fork(
        &mut self,
        forkpoint: &String,
        endpoint: &String,
        block: Block,
    ) -> Result<ForkId, NextBlockErr> {
        let mut fork: Blocks = self.remove(forkpoint, endpoint).unwrap();
        Blocks::push_back(&mut fork, block)?;
        let fork_id = self.insert(fork);
        Ok(fork_id)
    }

    pub fn nest_fork(
        &mut self,
        forkpoint: &String,
        endpoint: &String,
        block: Block,
    ) -> Result<ForkId, NextBlockErr> {
        let mut fork_clone: Blocks = self.get(forkpoint, endpoint).unwrap().clone();
        let _ = Blocks::split_off_until(&mut fork_clone, |b| b.hash == block.prev_hash);
        Blocks::push_back(&mut fork_clone, block)?;
        let fork_id = self.insert(fork_clone);
        Ok(fork_id)
    }

    pub fn print(&self) {
        for (_, forks_from) in self.0.iter() {
            for (i, (_, fork)) in forks_from.iter().enumerate() {
                let id = Self::identify(fork);
                println!(
                    "Fork from (idx: {}, hash: {}) #{}:",
                    id.fork_idx,
                    abbrev(&id.fork_hash),
                    i
                );
                fork.iter().for_each(|block| println!("{}", block));
            }
        }
    }
}

impl Default for Forks {
    fn default() -> Self {
        Self::new()
    }
}

// Orphan branches are represented as a disjoint set of chains that are constructed backwards.
// We do not track whether each orphan branch has blocks in common i.e. are forks of each other;
// they are used to connect an orphan node back to the main chain as fast as possible, at which point it forms a fork.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orphans(HashMap<String, Blocks>); // <fork point, orphaned branch>
pub type OrphanId = String; // fork hash

impl Orphans {
    pub fn new() -> Self {
        Orphans(HashMap::new())
    }

    pub fn find<'a, P>(&'a self, prop: P) -> Option<(OrphanId, &'a Blocks, &'a Block)>
    where
        P: Fn(&Block) -> bool,
    {
        for (forkpoint, orphan) in self.0.iter() {
            if let Some(b) = Blocks::find(orphan, &prop) {
                return Some((forkpoint.clone(), orphan, b));
            }
        }
        None
    }

    pub fn get<'a>(&'a self, forkpoint: &String) -> Option<&'a Blocks> {
        self.0.get(forkpoint)
    }

    pub fn get_mut<'a>(&'a mut self, forkpoint: &String) -> Option<&'a mut Blocks> {
        self.0.get_mut(forkpoint)
    }

    pub fn insert(&mut self, orphan: Blocks) -> OrphanId {
        let orphan_id: String = orphan.first().prev_hash.clone();
        self.0.insert(orphan_id.clone(), orphan);
        orphan_id
    }

    pub fn remove(&mut self, forkpoint: &String) -> Option<Blocks> {
        self.0.remove(forkpoint)
    }

    pub fn extend_orphan(&mut self, block: Block) -> Result<OrphanId, NextBlockErr> {
        let mut orphan: Blocks = self.remove(&block.hash).unwrap();
        Blocks::push_front(&mut orphan, block.clone())?;
        Ok(self.insert(orphan))
    }

    pub fn print(&self) {
        for (i, orphan) in self.0.iter().enumerate() {
            println!("Orphaned branch {}:\n\t{:?}\n", i, orphan.1);
        }
    }
}

impl Default for Orphans {
    fn default() -> Self {
        Self::new()
    }
}
