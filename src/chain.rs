/*
    *Chain*:
    - Chain internals, a safe wrapper that manages a main chain and a hashmap of forks.
    - Methods for safely constructing, accessing, mining, extending, and validating a chain with respect to other blocks, chains, or forks.
*/

use super::{
    block::{Block, Blocks, NextBlockErr, NextBlockResult},
    fork::{ForkId, Forks, Orphans},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chain {
    main: Blocks,
    forks: Forks,
    orphans: Orphans,
}

/* Chain core operations */
impl Chain {
    pub fn genesis() -> Self {
        Self {
            main: Blocks::genesis(),
            forks: Forks::new(),
            orphans: Orphans::new(),
        }
    }

    // Swap the main chain to a local fork if valid and longer.
    pub fn choose_fork(&mut self) -> Result<ChainStatus, NextBlockErr> {
        if let Some((_, fork_id)) = self.forks.longest() {
            let (main_len, other_len) = (self.last().idx + 1, fork_id.end_idx + 1);
            if main_len < other_len {
                // remove the fork from the fork pool
                let fork: Blocks = self
                    .forks
                    .remove(&fork_id.fork_hash, &fork_id.end_hash)
                    .expect("fork definitely exists; we just stored it");
                // truncate the main chain to include the forkpoint as its last block
                let main_suffix: Option<Blocks> =
                    self.main.split_off_until(|b| b.hash == *fork_id.fork_hash);
                // if the removed suffix is non-empty, insert it as a fork
                if let Some(suffix) = main_suffix {
                    self.forks.insert(suffix);
                }
                // append the fork to the truncated main chain
                Blocks::append(&mut self.main, fork)?;
                // delete all previous forks that don't fork from the new chain
                let forkpoints: Vec<String> = self.main.iter().map(|b| b.hash.clone()).collect();
                self.forks.retain_forkpoints(&forkpoints);

                Ok(ChainStatus::ChooseOther {
                    main_len,
                    other_len,
                })
            } else {
                Ok(ChainStatus::KeepMain {
                    main_len,
                    other_len: Some(other_len),
                })
            }
        } else {
            Ok(ChainStatus::KeepMain {
                main_len: self.len(),
                other_len: None,
            })
        }
    }

    // Swap the main chain to a remote chain if valid and longer.
    pub fn choose_chain(&mut self, other: Chain) -> Result<ChainStatus, NextBlockErr> {
        other.validate()?;

        let (main_genesis, other_genesis) = (self.main.first(), other.main.first());
        if main_genesis != other_genesis {
            return Err(NextBlockErr::InvalidGenesis {
                idx: other_genesis.idx,
                hash: other_genesis.hash.clone(),
            });
        }
        let (main_len, other_len) = (self.last().idx + 1, other.last().idx + 1);
        if main_len < other_len {
            self.main = other.main.clone();
            // delete all previous forks that don't fork from the new chain
            let forkpoints: Vec<String> = self.main.iter().map(|b| b.hash.clone()).collect();
            self.forks.retain_forkpoints(&forkpoints);

            Ok(ChainStatus::ChooseOther {
                main_len,
                other_len,
            })
        } else {
            Ok(ChainStatus::KeepMain {
                main_len,
                other_len: Some(other_len),
            })
        }
    }

    // Try to store a new block in either the main chain or fork pool
    pub fn store_new_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr> {
        block.validate()?;

        let is_duplicate = |b: &Block| b.hash == block.hash;
        let is_parent = |b: &Block| block.validate_parent(b).is_ok();

        // Search for block in the main chain and forks
        if self.find(&is_duplicate).is_some() || self.forks.find(&is_duplicate).is_some() {
            Err(NextBlockErr::Duplicate {
                idx: block.idx,
                hash: block.hash,
            })
        }
        // Search for parent block in the main chain.
        else if let Some(parent) = self.find(&is_parent) {
            // See if we can append the block to the main chain
            if self.last().hash == parent.hash {
                Blocks::push_back(&mut self.main, block)?;
                Ok(NextBlockResult::ExtendedMain {
                    end_idx: self.last().idx,
                    end_hash: self.last().hash.clone(),
                })
            }
            // Otherwise attach a single-block fork to the main chain
            else {
                let fork_id = self.forks.insert(Blocks::from_vec(vec![block.clone()])?);
                Ok(fork_id.into_new_fork_result())
            }
        }
        // Search for parent block in the forks.
        else if let Some((
            ForkId {
                fork_hash,
                end_hash,
                ..
            },
            _,
            parent,
        )) = self.forks.find(&is_parent)
        {
            // If its parent was the last block in the fork, append the block and update the endpoint key
            if parent.hash == end_hash {
                let fork_id: ForkId = self.forks.extend_fork(&fork_hash, &end_hash, block)?;
                Ok(fork_id.into_extended_fork_result())
            }
            // Otherwise create a new fork from the main chain that clones the prefix of an existing fork
            else {
                let fork_id: ForkId = self.forks.nest_fork(&fork_hash, &end_hash, block)?;
                Ok(fork_id.into_new_fork_result())
            }
        }
        // Encountered a genesis block not in the main chain
        else if block.idx == 0 {
            Err(NextBlockErr::InvalidGenesis {
                idx: block.idx,
                hash: block.hash,
            })
        }
        // Otherwise, insert a new single-block orphan branch, and report a missing block that connects it to the current network
        else {
            self.orphans.insert(Blocks::from_vec(vec![block.clone()])?);
            Err(NextBlockErr::MissingParent {
                parent_idx: block.idx - 1,
                parent_hash: block.prev_hash,
            })
        }
    }

    // Try to store a fork if valid and forks from the main chain
    pub fn store_new_fork(&mut self, blocks: Blocks) -> Result<ForkId, NextBlockErr> {
        blocks.validate()?;

        let first_block = blocks.first();
        let is_parent = |b: &Block| first_block.validate_parent(b).is_ok();

        if self.find(&is_parent).is_some() {
            let fork_id = self.forks.insert(blocks);
            Ok(fork_id)
        }
        // catch when the fork has extended all the way to the genesis block (should only happen when connecting orphans)
        else if first_block.idx == 0 {
            Err(NextBlockErr::InvalidGenesis {
                idx: first_block.idx,
                hash: first_block.hash.clone(),
            })
        } else {
            Err(NextBlockErr::MissingParent {
                parent_idx: first_block.idx - 1,
                parent_hash: first_block.prev_hash.clone(),
            })
        }
    }

    // Try to store a block in an orphan branch to be attached as a new fork
    pub fn store_orphan_block(&mut self, block: Block) -> Result<NextBlockResult, NextBlockErr> {
        block.validate()?;

        let is_duplicate = |b: &Block| b.hash == block.hash;

        // Search for block in the orphans.
        if self.orphans.find(is_duplicate).is_some() {
            Err(NextBlockErr::Duplicate {
                idx: block.idx,
                hash: block.hash,
            })
        }
        // Lookup the block as the forkpoint for any orphan branches
        else if self.orphans.get_mut(&block.hash).is_some() {
            // Try to extend the orphan branch from the front
            let orphan_id = self.orphans.extend_orphan(block)?;
            let orphan = self.orphans.get(&orphan_id).unwrap();
            // Try to store the orphan branch as a valid fork from the main chain
            let fork_id = self.store_new_fork(orphan.clone())?;
            // Remove the extended orphan from the pool, and return the new fork
            self.orphans.remove(&orphan_id);
            Ok(fork_id.into_new_fork_result())
        } else {
            Err(NextBlockErr::StrayOrphan {
                idx: block.idx,
                hash: block.hash,
            })
        }
    }

    // Mine a new valid block from given data
    pub fn mine_block(&mut self, data: &str) {
        self.main.mine_block(data)
    }

    // Validate chain expecting its first block to begin at idx 0
    pub fn validate(&self) -> Result<(), NextBlockErr> {
        let first_block: &Block = self.main.first();
        if first_block.idx == 0 {
            Blocks::validate(&self.main)
        } else {
            Err(NextBlockErr::InvalidGenesis {
                idx: first_block.idx,
                hash: first_block.hash.clone(),
            })
        }
    }
}

/* Chain auxiliary functions */
impl Chain {
    // Constructor
    pub fn from_vec(blocks: Vec<Block>) -> Result<Chain, NextBlockErr> {
        let chain = Chain {
            main: Blocks::from_vec(blocks)?,
            forks: Forks::new(),
            orphans: Orphans::new(),
        };
        chain.validate()?;
        Ok(chain)
    }

    // Destructor
    pub fn to_vec(self) -> Vec<Block> {
        self.main.to_vec()
    }

    pub fn len(&self) -> usize {
        self.main.len()
    }

    pub fn find<'a, P>(&'a self, prop: &P) -> Option<&'a Block>
    where
        P: Fn(&Block) -> bool,
    {
        self.main.find(prop)
    }

    pub fn idx(&self, idx: usize) -> Option<&Block> {
        self.main.get(idx)
    }

    pub fn last(&self) -> &Block {
        self.main.last()
    }

    pub fn split_off(&mut self, len: usize) -> Option<Blocks> {
        self.main.split_off(len)
    }

    pub fn forks<'a>(&'a self) -> &'a Forks {
        &self.forks
    }

    pub fn print_forks(&self) {
        self.forks.print()
    }

    pub fn orphans<'a>(&'a self) -> &'a Orphans {
        &self.orphans
    }

    pub fn print_orphans(&self) {
        self.orphans.print()
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}", self.main)
    }
}

#[derive(Debug)]
pub enum ChainStatus {
    KeepMain {
        main_len: usize,
        other_len: Option<usize>,
    },
    ChooseOther {
        main_len: usize,
        other_len: usize,
    },
}

impl std::fmt::Display for ChainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChainStatus::KeepMain {
                main_len,
                other_len,
            } => {
                if let Some(other_len) = other_len {
                    write!(
                        f,
                        "Keeping current main chain with length {}.\n\
                    Alternative chain or fork that was compared has total length {}.",
                        main_len, other_len
                    )
                } else {
                    write!(f, "Updated main chain has length {}.", main_len)
                }
            }
            ChainStatus::ChooseOther {
                main_len,
                other_len,
            } => {
                write!(
                    f,
                    "Choosing other chain or fork with length {}. \n\
                     Old main chain had length {}.\n\
                     Storing old main as a fork if valid.",
                    other_len, main_len
                )
            }
        }
    }
}
