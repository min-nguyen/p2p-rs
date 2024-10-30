/*
    *Chain*:
    - Chain internals, a safe wrapper that manages a main chain and a hashmap of forks.
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use super::{
    block::{Block, NextBlockResult, NextBlockErr},
    fork::{Forks, ForkId, OrphanId, Orphans}
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main: Vec<Block>,
    forks: Forks,
    orphans: Orphans
}

/* Chain core operations */
impl Chain {
    pub fn genesis() -> Self {
        Self { main : vec![Block::genesis()], forks : Forks::new(), orphans : Orphans::new()}
    }

    pub fn handle_block_result(&mut self, res: NextBlockResult) -> Result<ChooseChainResult, NextBlockErr>{
        match res {
            NextBlockResult::ExtendedFork { fork_hash,end_hash, .. } => {
                self.sync_to_fork(fork_hash, end_hash)
            },
            NextBlockResult::NewFork { fork_hash, end_hash, .. } => {
                self.sync_to_fork(fork_hash, end_hash)
            }
            NextBlockResult::ExtendedMain {end_idx, .. } => {
                Ok(ChooseChainResult::KeepMain { main_len: end_idx + 1, other_len: None })
            }
        }
    }

    pub fn store_orphan_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr>{
        Block::validate_block(&block)?;

        let is_duplicate = |b: &Block| {b.hash == block.hash};

        // Search for block in the orphans.
        if let Some(..) = self.orphans.find(is_duplicate){
            Err(NextBlockErr::Duplicate { idx: block.idx, hash: block.hash })
        }
        // Lookup the block as the forkpoint for any orphan branches
        else if let Some(orphan) = self.orphans.get_mut(&block.hash) {
            Block::validate_child(&block, &orphan.first().unwrap())?;
            let orphan_id: OrphanId = self.orphans.extend_orphan(block)?;
            match self.connect_orphan_as_fork(&orphan_id) {
                Ok(fork_id) => {
                    Ok(NextBlockResult::NewFork { // ForkId as NewFork? (cast)
                        fork_idx: fork_id.fork_idx, fork_hash: fork_id.fork_hash,
                        end_idx:  fork_id.end_idx, end_hash:  fork_id.end_hash,
                    })
                },
                Err(e) => Err(e),
            }
        }
        else {
            Err(NextBlockErr::StrayParent { idx: block.idx, hash: block.hash })
        }
    }

    // Try to connect an orphan branch as a fork from the main chain
    pub fn connect_orphan_as_fork(&mut self, orphan: &OrphanId) -> Result<ForkId, NextBlockErr>{
        let orphan = self.orphans.get(orphan).unwrap();
        Self::validate_fork(&self, &orphan)?;
        let fork_id = self.forks.insert(orphan.clone())?;
        self.orphans.remove_entry(&Orphans::validate(&orphan)?);
        Ok(fork_id)
    }

    pub fn store_new_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr>{
        Block::validate_block(&block)?;

        let is_duplicate = |b: &Block| {b.hash == block.hash};
        let is_parent = |b: &Block| { Block::validate_child(b, &block).is_ok()};

        // Search for block in the main chain and forks
        if            self.find(is_duplicate).is_some()
            ||  self.forks.find(is_duplicate).is_some()  {
            Err(NextBlockErr::Duplicate { idx: block.idx, hash: block.hash })
        }
        // Search for parent block in the main chain.
        else if let Some(parent)
                = self.find(is_parent){

            // See if we can append the block to the main chain
            if self.last_block().hash == parent.hash {
                Block::push_end(&mut self.main, block);
                Ok(NextBlockResult::ExtendedMain {
                    end_idx: self.last_block().idx, end_hash: self.last_block().hash.clone() })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let ForkId { fork_idx, fork_hash, end_idx, end_hash}
                    = self.forks.insert(vec![block.clone()])?;

                Ok(NextBlockResult::NewFork {fork_idx, fork_hash, end_idx, end_hash })
            }
        }
        // Search for parent block in the forks.
        else if let Some((ForkId {fork_hash, end_hash, ..}, _, parent))
                 = self.forks.find(is_parent) {

            // If its parent was the last block in the fork, append the block and update the endpoint key
            if  parent.hash == end_hash {
                let ForkId { fork_idx, fork_hash, end_idx, end_hash}
                    = self.forks.extend_fork(&fork_hash, &end_hash, block)?;

                Ok(NextBlockResult::ExtendedFork {fork_idx, fork_hash, end_idx, end_hash })
            }
            // Otherwise create a new fork from the main chain that clones the prefix of an existing fork
            else {
                let ForkId { fork_idx, fork_hash, end_idx, end_hash}
                    = self.forks.nest_fork(&fork_hash, &end_hash, block)?;

                Ok(NextBlockResult::NewFork {fork_idx, fork_hash, end_idx, end_hash })
            }
        }
        // Otherwise, report a missing block that connects it to the current network
        else {
            if block.idx > 0 {
                // Insert as a single-block orphan
                self.orphans.insert(vec![block.clone()])?;
                Err(NextBlockErr::MissingParent {
                        parent_idx: block.idx - 1,
                        parent_hash: block.prev_hash
                })
            }
            else { // block.idx == 0 && not in main chain or forks
                Err(NextBlockErr::UnrelatedGenesis { genesis_hash: block.hash })
            }
        }
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        let new_block = Block::mine_block(self.last_block(), data);
        Block::push_end(&mut self.main, new_block)
    }

    // Validate chain, expecting its first block to begin at idx 0
    pub fn validate(&self) -> Result<(), NextBlockErr> {
        Block::validate_blocks(&self.main)?;
        let first_block = self.main.first().unwrap();
        if first_block.idx == 0 {
            Ok(())
        }
        else {
            Err( NextBlockErr::InvalidIndex { idx: first_block.idx, expected_idx: 0 })
        }
    }

    // Swap the main chain to a remote chain if valid and longer.
    pub fn sync_to_chain(&mut self, other: Chain) -> Result<ChooseChainResult, NextBlockErr> {
        Chain::validate(&other)?;
        let (main_genesis, other_genesis)
            = (self.main.first().unwrap().hash.clone(), other.main.first().unwrap().hash.clone());
        if main_genesis != other_genesis {
            return Err (NextBlockErr::UnrelatedGenesis { genesis_hash: other_genesis  })
        }
        let (main_len, other_len) = (self.last_block().idx + 1, other.last_block().idx + 1);
        if main_len < other_len {
            *self = other.clone();
            Ok(ChooseChainResult::ChooseOther { main_len, other_len })
        } else {
            Ok(ChooseChainResult::KeepMain { main_len, other_len: Some(other_len) })
        }
    }

    // Validate fork as a branch off the main chain
    pub fn validate_fork(&self, fork: &Vec<Block>) -> Result<(), NextBlockErr> {
        Forks::validate(fork)?;

        let first_block = fork.first().unwrap();
        let is_parent = |b: &Block| { Block::validate_child(b, &first_block).is_ok()};

        if let Some(..) = self.find(|b| is_parent(b)) {
            Ok (())
        }
        // catch when the fork has extended all the way to the genesis block (should generally not happen)
        else if first_block.idx == 0 {
            if first_block.hash == self.main.first().unwrap().hash {
                Ok (())
            }
            else {
                Err(NextBlockErr::UnrelatedGenesis { genesis_hash: first_block.hash.clone() })
            }
        }
        else {
            Err(NextBlockErr::MissingParent {
                parent_idx: first_block.idx - 1,
                parent_hash: first_block.prev_hash.clone()})
        }
    }

    // Swap the main chain to a fork in the pool if longer
    pub fn sync_to_fork(&mut self, fork_hash: String, end_hash: String) -> Result<ChooseChainResult, NextBlockErr>{
        if let Some((_, fork_id)) = self.forks.get_mut(&fork_hash, &end_hash) {
            let (main_len, other_len) = (self.last_block().idx + 1, fork_id.end_idx + 1);
            if main_len < other_len {
                // remove the fork from the fork pool
                let mut fork
                    = self.forks.remove_entry(&fork_id.fork_hash, &fork_id.end_hash)
                                .expect("fork definitely exists; we just stored it");
                // truncate the main chain to the forkpoint, and append the fork to it
                let main_suffix: Vec<Block>
                    = Block::split_off_until(&mut self.main, |b| b.hash == *fork_id.fork_hash);
                Block::append(&mut self.main, &mut fork);

                // insert the removed suffix of the main chain as a new fork if non-empty (i.e., if the fork doesn't directly extend from it)
                // the only case it is non-empty
                if !main_suffix.is_empty() {
                    self.forks.insert(main_suffix)?;
                }

                return Ok(ChooseChainResult::ChooseOther { main_len, other_len })
            }
            else {
                return Ok(ChooseChainResult::KeepMain { main_len, other_len: Some(other_len) })
            }
        }
        Ok(ChooseChainResult::KeepMain { main_len: self.len(), other_len: None })
    }
}

/* Chain auxiliary functions */
impl Chain {
    pub fn from_vec(blocks: Vec<Block>) -> Result<Chain, NextBlockErr> {
        let chain = Chain{main : blocks, forks: Forks::new(), orphans: Orphans::new()};
        Self::validate(&chain)?;
        Ok(chain)
    }

    pub fn to_vec(&self) -> Vec<Block> {
        self.main.clone()
    }

    pub fn len(&self) -> usize {
        self.main.len()
    }

    pub fn find<'a, P> (&'a self, prop: P) -> Option<&'a Block>
    where P: Fn(&Block) -> bool{
        Block::find(&self.main, prop)
    }

    pub fn idx(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
    }

    pub fn last_block(&self) -> &Block {
        self.main.last().expect("Chain should always be non-empty")
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

    pub fn forks<'a>(&'a self) -> &'a Forks {
        &self.forks
    }

    pub fn insert_fork(&mut self, fork: Vec<Block>) -> Result<ForkId, NextBlockErr>{
        self.forks.insert(fork)
    }

    pub fn print_forks(&self){
        self.forks.print()
    }

    pub fn orphans<'a>(&'a self) -> &'a Orphans {
        &self.orphans
    }

    pub fn print_orphans(&self){
        self.orphans.print()
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