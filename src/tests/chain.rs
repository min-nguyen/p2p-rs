/******************
      TESTS
********************/
#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{
        block::{Block, NextBlockResult, NextBlockErr}, chain::{Chain}, cryptutil::trace};

    const CHAIN_LEN : usize = 5;
    const FORK_PREFIX_LEN : usize = 3;

    /*****************************
     * Tests for valid chains    *
    *****************************/
    #[test]
    fn test_valid_chain() {
        let mut chain: Chain = Chain::genesis();
        for i in 1 .. CHAIN_LEN {
            chain.mine_block(&format!("block {}", i));
        }
        assert!(matches!(
            trace(Chain::from_vec(chain.to_vec())),
            Ok(_)));
    }
    #[test]
    fn test_chain_is_empty(){
        assert!(matches!(
            trace(Chain::from_vec(vec![])),
            Err(NextBlockErr::EmptyChain)));
    }
    #[test]
    fn test_chain_is_fork(){
        assert!(matches!(
            trace(Chain::from_vec(vec![Block { idx : 7, .. Block::genesis() }])),
            Err(NextBlockErr::InvalidIndex { block_idx : 7, expected_idx: 0 })));
    }
    /*****************************
     * Tests for handling new blocks *
    *****************************/
    #[test]
    fn test_valid_next_block() {
        let mut chain: Chain = Chain::genesis();
        let next_block = Block::mine_block(chain.last(), "next valid block");

        // chain: [0]---[1]
        assert!(matches!(
            trace(chain.handle_new_block(next_block))
            , Ok(..)));
    }
    #[test]
    fn test_adding_and_extending_forks() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_block(&format!("block {}", i));
        }

        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[*3*]
        let mut forked_chain = {
            let mut f = chain.clone();
            let _ = f.split_off(FORK_PREFIX_LEN);
            f
        };
        forked_chain.mine_block(&format!("block {} in fork", 0));
        assert!(matches!(
            trace(chain.handle_new_block(forked_chain.last().clone())),
            Ok(NextBlockResult::NewFork { length : 1, .. })
        ));
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[*4*]---[*5*]
        for i in 1..3 {
            forked_chain.mine_block(&format!("block {} in fork", i));
            assert!(matches!(
                trace(chain.handle_new_block(forked_chain.last().clone())),
                Ok(NextBlockResult::ExtendedFork { .. })
            ));
        }
        println!("FORKED CHAIN {}", forked_chain);
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]
        // nested fork:                    |----[*5*]
        let mut nested_forked_chain = {
                let mut f = forked_chain.clone();
                let _ = f.split_off(f.len() - 1);
                f
        };
        println!("NESTED FORK {}", nested_forked_chain);
        nested_forked_chain.mine_block(&format!("block {} in nested fork", 0));
        assert!(matches!(
            trace(chain.handle_new_block(nested_forked_chain.last().clone())),
            Ok(NextBlockResult::NewFork {length : 3, .. })
        ));
        println!("NESTED FORK {}", nested_forked_chain);
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]
        // nested fork:                    |----[5]---[6]---[7]
        for i in 1..3 {
            nested_forked_chain.mine_block(&format!("block {} in nested fork", i));
            assert!(matches!(
                trace(chain.handle_new_block(nested_forked_chain.last().clone())),
                Ok(NextBlockResult::ExtendedFork { .. })
            ));
        }
        println!("{}", nested_forked_chain);

    }
    #[test]
    fn test_duplicate_block_in_main() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_block(&format!("block {}", i))
        }
        // handle an old block from the current chain that is one block older than the tip
        let out_of_date_block: Block = chain.get(chain.last().idx - 1).unwrap().clone();
        // chain: [0]---[1]---[2]---[3]---[4]
        //                     |---[*3*]
        assert!(matches!(
            trace(chain.handle_new_block(out_of_date_block)),
            Ok( NextBlockResult::NewFork { .. } )   // to-do: implement Duplicateblock
        ));
    }
    #[test]
    fn test_missing_parent_in_fork() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN{
            chain.mine_block(&format!("block {}", i));
        }
        // handle a competing block from a forked_chain that is the same length as the current chain
        let mut forked_chain: Chain = chain.clone();
        let _ = forked_chain.split_off(FORK_PREFIX_LEN);
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            forked_chain.mine_block(&format!("block {} in fork", i))
        }
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[?]---[*4*]
        assert!(matches!(
            trace(chain.handle_new_block(forked_chain.last().clone())),
            Ok(NextBlockResult::MissingParent{..})
        ));
    }

    #[test]
    fn test_missing_parent_in_main() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_block(&format!("block {}", i));
        }
        // handle a block from an up-to-date chain that is at a height 2 more than the current chain
        let mut dup_chain: Chain = chain.clone();
        dup_chain.mine_block("next block in dup chain");
        dup_chain.mine_block("next block in dup chain");
        // chain:      [0]---[1]---[2]---[3]---[4]---[?]---[*6*]
        assert!(matches!(
            trace(chain.handle_new_block(dup_chain.last().clone())),
            Ok(NextBlockResult::MissingParent { .. })
        ));
    }

    // /*****************************
    //  * Tests for merging forks *
    // *****************************/
    // #[test]
    // fn test_valid_fork_longer(){
    //     let mut chain: Chain = Chain::genesis();
    //     for i in 1..CHAIN_LEN {
    //         chain.mine_block(&format!("block {}", i));
    //     }
    //     // make a competing forked_chain that is 2 blocks longer than the current chain
    //     let mut forked_chain = chain.clone();
    //     forked_chain.truncate(FORK_PREFIX_LEN);
    //     for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
    //         forked_chain.mine_block(&format!("block {} in fork", i));
    //     }
    //     // strip the common prefix between the current and forked chain
    //     let mut fork: Vec<Block> = forked_chain.to_vec()[FORK_PREFIX_LEN..].to_vec();
    //     // Before:
    //     // chain: [0]---[1]---[2]---[3]---[4]
    //     // fork:               |----[3]---[4]---[5]---[6]
    //     println!("Chain : {}\n\nFork suffix : {:?}\n", chain, fork);
    //     assert!(matches!(
    //         trace(chain.try_merge_fork(&mut fork)),
    //         Ok(())
    //     ));
    //     println!("Merged chain and fork : {}", chain);
    //     // After:
    //     // chain: [0]---[1]---[2]
    //     //                     |----[3]---[4]---[5]---[6]

    // }
    // #[test]
    // fn test_valid_fork_shorter() {
    //     let mut chain: Chain = Chain::genesis();
    //     for i in 1..CHAIN_LEN {
    //         chain.mine_block(&format!("block {}", i));
    //     }
    //     // make a competing forked_chain that is 2 blocks longer than the current chain
    //     let mut forked_chain = chain.clone();
    //     forked_chain.truncate(FORK_PREFIX_LEN);
    //     for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
    //         forked_chain.mine_block(&format!("block {} in fork", i));
    //     }
    //     // then make the current chain 2 blocks longer than the forked_chain
    //     for i in CHAIN_LEN .. forked_chain.len() + 2 {
    //         chain.mine_block(&format!("block {}", i));
    //     }
    //     // strip the common prefix between the current and forked chain
    //     let mut fork: Vec<Block> = forked_chain.to_vec()[FORK_PREFIX_LEN..].to_vec();
    //     // Before:
    //     // chain: [0]---[1]---[2]---[3]---[4]---[5]---[6]---[7]---[8]
    //     // fork:               |----[3]---[4]---[5]---[6]
    //     println!("Chain : {}\n\nFork suffix : {:?}\n", chain, fork);
    //     assert!(matches!(
    //         trace(chain.try_merge_fork(&mut fork)),
    //         Ok(())
    //     ));
    //     println!("Merged chain and fork : {}", chain);
    //     // After:
    //     // chain: [0]---[1]---[2]
    //     //                     |----[3]---[4]---[5]---[6]
    // }
}