/******************
      TESTS
********************/
#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{
        chain::{Chain, ChainErr, ForkErr},
        block::{Block, NextBlockErr},
        cryptutil::debug};

    const CHAIN_LEN : usize = 5;
    const FORK_PREFIX_LEN : usize = 3;

    /*****************************
     * Tests for valid chains    *
    *****************************/
    #[test]
    fn test_valid_chain() {
        let mut chain: Chain = Chain::genesis();
        for i in 1 .. CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        assert!(matches!(
            debug(Chain::from_vec(chain.blocks())),
            Ok(_)));
    }
    #[test]
    fn test_chain_is_empty(){
        assert!(matches!(
            debug(Chain::from_vec(vec![])),
            Err(ChainErr::ChainIsEmpty)));
    }
    #[test]
    fn test_chain_is_fork(){
        assert!(matches!(
            debug(Chain::from_vec(vec![Block { idx : 1, .. Block::genesis() }])),
            Err(ChainErr::ChainIsFork)));
    }
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
    fn test_block_too_old() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i))
        }
        // handle an old block from the current chain that is one block older than the tip
        let out_of_date_block: Block = chain.get(chain.last().idx - 1).unwrap().clone();
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
        let duplicate_block: Block = chain.last().clone();
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
            chain.last().idx,
            "competing block at {}",
            &chain.last().prev_hash);
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i))
        }
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[*4*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.last())),
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 1 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[*5*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.last())),
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
            debug(chain.handle_new_block(dup_chain.last())),
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // chain:   [0]---[1]---[2]---[3]---[4]
        // fork:                 |----[3]---[4]---[5]---[*6*]
        assert!(matches!(
            debug(chain.handle_new_block(forked_chain.last())),
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain
        let mut fork: Vec<Block> = forked_chain.blocks()[FORK_PREFIX_LEN..].to_vec();
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // then make the current chain 2 blocks longer than the forked_chain
        for i in CHAIN_LEN .. forked_chain.len() + 2 {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // strip the common prefix between the current and forked chain
        let mut fork: Vec<Block> = forked_chain.blocks()[FORK_PREFIX_LEN..].to_vec();
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
    fn test_fork_is_empty() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        let mut empty_fork = vec![];
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:   ???
        assert!(matches!(
            debug(chain.try_merge_fork(&mut empty_fork)),
            Err(ForkErr::ForkIsEmpty { .. })
        ));
    }
    #[test]
    fn test_fork_starts_at_genesis() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_then_push_block(&format!("block {}", i));
        }
        // make a competing forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // try to merge the entire forked chain  rather than the fork.
        // chain :  [0]---[1]---[2]---[3]---[4]
        // "fork":  [0]---[1]---[2]---[3]---[4]---[5]---[6]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut forked_chain.blocks())),
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
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain, then **remove the first block** from the fork
        let mut incompatible_fork: Vec<Block> = forked_chain.blocks()[FORK_PREFIX_LEN..].to_vec().split_off(1);
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
        // make a competing forked_chain that is 2 blocks longer than the current chain
        let mut forked_chain = chain.clone();
        forked_chain.truncate(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
            forked_chain.mine_then_push_block(&format!("block {} in fork", i));
        }
        // strip the common prefix between the current and forked chain, and then **mutate the last block** in the fork
        let mut invalid_subchain_fork: Vec<Block> = {
            let mut fork: Vec<Block> = forked_chain.blocks()[FORK_PREFIX_LEN..].to_vec();
            let b: &mut Block = fork.last_mut().unwrap();
            b.data = "corrupt data".to_string();
            fork
        };
        // try to merge a fork that is corrupt subchain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[X]
        assert!(matches!(
            debug(chain.try_merge_fork(&mut invalid_subchain_fork)),
            Err(ForkErr::InvalidSubChain{ .. })
        ));
    }

}