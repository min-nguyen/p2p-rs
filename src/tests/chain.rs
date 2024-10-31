/******************
      TESTS
********************/
#[cfg(test)] // cargo test chain -- --nocapture
mod chain_tests {
    use crate::{
        block::{Block, Blocks, NextBlockErr, NextBlockResult},
        chain::{Chain, ChooseChainResult},
        util::trace,
    };

    const CHAIN_LEN: usize = 5;
    const FORK_PREFIX_LEN: usize = 3;

    fn init_chain(n: usize) -> Chain {
        let mut chain: Chain = Chain::genesis();
        for i in 1..n {
            chain.mine_block(&format!("block {}", i));
        }
        chain
    }

    /*****************************
     * Tests for valid chains    *
     *****************************/
    #[test]
    fn test_validate_chain() {
        let chain: Chain = init_chain(CHAIN_LEN);
        assert!(matches!(trace(Chain::from_vec(chain.to_vec())), Ok(_)));
    }
    #[test]
    fn test_validate_chain_empty() {
        let blocks: Vec<Block> = vec![];
        assert!(matches!(
            trace(Chain::from_vec(blocks)),
            Err(NextBlockErr::NoBlocks)
        ));
    }
    #[test]
    fn test_validate_chain_invalid_idx() {
        let blocks: Vec<Block> = init_chain(CHAIN_LEN)
            .split_off(FORK_PREFIX_LEN)
            .unwrap()
            .to_vec();
        assert!(matches!(
            trace(Chain::from_vec(blocks)),
            Err(NextBlockErr::InvalidIndex {
                idx: 3,
                expected_idx: 0
            })
        ));
    }
    /*****************************
     * Tests for handling new blocks *
     *****************************/
    #[test]
    fn test_store_new_block_next() {
        let mut chain: Chain = init_chain(CHAIN_LEN);
        let next_block: Block = Block::mine_block(chain.last(), "next valid block");

        // chain: [0]---[1]---[2]---[3]---[4]----[*5*]
        assert!(matches!(
            trace(chain.store_new_block(next_block)),
            Ok(NextBlockResult::ExtendedMain { end_idx: 5, .. })
        ));
    }

    #[test]
    fn test_store_new_block_missing_parent() {
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
            trace(chain.store_new_block(dup_chain.last().clone())),
            Err(NextBlockErr::MissingParent { parent_idx: 5, .. })
        ));
    }

    #[test]
    fn test_store_new_block_duplicate() {
        let mut chain: Chain = Chain::genesis();
        for i in 1..CHAIN_LEN {
            chain.mine_block(&format!("block {}", i))
        }
        // handle an old block from the current chain that is one block older than the tip
        let out_of_date_block: Block = chain.idx(chain.last().idx - 1).unwrap().clone();
        // chain: [0]---[1]---[2]---[3]---[4]
        //                     |---[*3*]
        assert!(matches!(
            trace(chain.store_new_block(out_of_date_block)),
            Err(NextBlockErr::Duplicate { idx: 3, .. }) // to-do: implement Duplicateblock
        ));
    }

    #[test]
    fn test_store_new_block_fork() {
        let mut main_chain: Chain = init_chain(CHAIN_LEN);
        let mut forked_chain = {
            let mut f = main_chain.clone();
            f.split_off(FORK_PREFIX_LEN);
            f
        };

        {
            // Adding new forks
            // chain: [0]---[1]---[2]---[3]---[4]
            // fork:               |----[*3*]
            forked_chain.mine_block(&format!("block {} in fork", 0));
            println!("Forked chain {}", forked_chain);
            let res = main_chain.store_new_block(forked_chain.last().clone());
            assert!(matches!(
                trace(res),
                Ok(NextBlockResult::NewFork {
                    fork_idx: 2,
                    end_idx: 3,
                    ..
                })
            ));
        }

        {
            // Extending existing forks
            // chain: [0]---[1]---[2]---[3]---[4]
            // fork:               |----[3]---[*4*]---[*5*]
            for i in 1..3 {
                forked_chain.mine_block(&format!("block {} in fork", i));
                assert!(matches!(
                    trace(main_chain.store_new_block(forked_chain.last().clone())),
                    Ok(NextBlockResult::ExtendedFork { fork_idx: 2, .. })
                ));
            }
            println!("Forked chain {}", forked_chain);
        }

        let mut nested_forked_chain = {
            let mut f = forked_chain.clone();
            let _ = f.split_off(f.len() - 1);
            f
        };

        {
            // Adding nested forks from existing forks
            // chain: [0]---[1]---[2]---[3]---[4]
            // fork:               |----[3]---[4]---[5]
            // nested fork:                    |----[*5*]
            nested_forked_chain.mine_block(&format!("block {} in nested fork", 0));
            println!("Nested forked chain {}", nested_forked_chain);
            assert!(matches!(
                trace(main_chain.store_new_block(nested_forked_chain.last().clone())),
                Ok(NextBlockResult::NewFork {
                    fork_idx: 2,
                    end_idx: 5,
                    ..
                })
            ));
        }

        {
            // Extending nested forks
            // chain: [0]---[1]---[2]---[3]---[4]
            // fork:               |----[3]---[4]---[5]
            // nested fork:                    |----[5]---[6]---[7]
            for i in 1..3 {
                nested_forked_chain.mine_block(&format!("block {} in nested fork", i));
                assert!(matches!(
                    trace(main_chain.store_new_block(nested_forked_chain.last().clone())),
                    Ok(NextBlockResult::ExtendedFork { fork_idx: 2, .. })
                ));
            }
            println!("Nested forked chain {}", nested_forked_chain);
        }
    }

    #[test]
    fn test_store_new_block_fork_missing_parent() {
        // handle a competing block from a forked_chain that is the same length as the current chain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[?]---[*4*]
        let mut main_chain: Chain = init_chain(CHAIN_LEN);
        let mut forked_chain: Chain = {
            let mut f = main_chain.clone();
            let _ = f.split_off(FORK_PREFIX_LEN);
            f
        };
        for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) {
            forked_chain.mine_block(&format!("block {} in fork", i))
        }
        assert!(matches!(
            trace(main_chain.store_new_block(forked_chain.last().clone())),
            Err(NextBlockErr::MissingParent { .. })
        ));
    }

    // /*****************************
    //  * Tests for merging forks *
    // *****************************/
    #[test]
    fn test_validate_fork() {
        // Make a competing forked_chain that is 2 blocks longer than the current chain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[6]
        let main_chain: Chain = init_chain(CHAIN_LEN);
        let fork: Blocks = {
            let mut forked_chain = main_chain.clone();
            forked_chain.split_off(FORK_PREFIX_LEN);
            for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
                forked_chain.mine_block(&format!("block {} in fork", i));
            }
            // strip the common prefix between the current and forked chain
            forked_chain.split_off(FORK_PREFIX_LEN).unwrap()
        };
        println!("Chain : {}\n\nFork : {:?}\n", main_chain, fork);
        assert!(matches!(
            trace(Chain::validate_fork(&main_chain, &fork)),
            Ok(())
        ));
    }
    #[test]
    fn test_validate_fork_missing_parent() {
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[?]---[*4*]
        let main_chain: Chain = init_chain(CHAIN_LEN);
        let fork: Blocks = {
            let mut forked_chain = main_chain.clone();
            forked_chain.split_off(FORK_PREFIX_LEN);
            for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
                forked_chain.mine_block(&format!("block {} in fork", i));
            }
            forked_chain.split_off(FORK_PREFIX_LEN + 1).unwrap()
        };
        assert!(matches!(
            trace(Chain::validate_fork(&main_chain, &fork)),
            Err(NextBlockErr::MissingParent { .. })
        ));
    }

    #[test]
    fn test_sync_to_fork_longer() {
        let mut main_chain: Chain = init_chain(CHAIN_LEN);
        let main_endpoint = main_chain.last().hash.clone();

        // Make a forked_chain that is 2 blocks longer than the current chain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]---[4]---[5]---[6]
        let fork: Blocks = {
            let mut forked_chain = main_chain.clone();
            forked_chain.split_off(FORK_PREFIX_LEN);
            for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) + 2 {
                forked_chain.mine_block(&format!("block {} in fork", i));
            }
            forked_chain.split_off(FORK_PREFIX_LEN).unwrap()
        };
        let (forkpoint, endpoint) = (fork.first().prev_hash.clone(), fork.last().hash.clone());

        // Assert initial state of chain and its stored forks
        let forks = main_chain.forks();
        assert!(matches!(trace(main_chain.len()), 5));
        assert!(matches!(trace(forks.get(&forkpoint, &endpoint)), None));
        assert!(matches!(trace(forks.get(&forkpoint, &main_endpoint)), None));
        println!("Chain: {}\n\nFork: {:?}\n", main_chain, fork);

        // Then synchronise:
        // chain: [0]---[1]---[2]
        //                     |----[3]---[4]---[5]---[6]
        let fork_id = main_chain.insert_fork(fork);
        let res = main_chain.sync_to_fork(fork_id.fork_hash, fork_id.end_hash);

        assert!(matches!(
            trace(res),
            Ok(ChooseChainResult::ChooseOther {
                main_len: 5,
                other_len: 7
            })
        ));
        println!("Merged chain and fork : {}", main_chain);

        // Assert final state of the chain and its stored forks
        let forks = main_chain.forks();
        assert!(matches!(trace(main_chain.len()), 7));
        assert!(matches!(
            trace(forks.get(&forkpoint, &main_endpoint)),
            Some(..)
        ));
        assert!(matches!(trace(forks.get(&forkpoint, &endpoint)), None));
    }

    #[test]
    fn test_sync_to_fork_shorter() {
        let mut main_chain: Chain = init_chain(CHAIN_LEN);
        let main_endpoint = main_chain.last().hash.clone();

        // Make a forked_chain that is 1 block shorter than the current chain
        // chain: [0]---[1]---[2]---[3]---[4]
        // fork:               |----[3]
        let fork: Blocks = {
            let mut forked_chain = main_chain.clone();
            forked_chain.split_off(FORK_PREFIX_LEN);
            for i in 0..(CHAIN_LEN - FORK_PREFIX_LEN) - 1 {
                forked_chain.mine_block(&format!("block {} in fork", i));
            }
            forked_chain.split_off(FORK_PREFIX_LEN).unwrap()
        };
        let (forkpoint, endpoint) = (fork.first().prev_hash.clone(), fork.last().hash.clone());

        // Assert initial state of chain and its stored forks
        let forks = main_chain.forks();
        assert!(matches!(trace(main_chain.len()), 5));
        assert!(matches!(trace(forks.get(&forkpoint, &endpoint)), None));
        assert!(matches!(trace(forks.get(&forkpoint, &main_endpoint)), None));

        println!("Chain: {}\n\nFork: {:?}\n", main_chain, fork);

        // Then synchronise:
        // chain: [0]---[1]---[2]---[3]---[4]
        let fork_id = main_chain.insert_fork(fork);
        let res = main_chain.sync_to_fork(fork_id.fork_hash, fork_id.end_hash);

        assert!(matches!(
            trace(res),
            Ok(ChooseChainResult::KeepMain {
                main_len: 5,
                other_len: Some(4)
            })
        ));
        println!("Merged chain and fork : {}", main_chain);

        // Assert final state of the chain and its stored forks
        let forks = main_chain.forks();
        assert!(matches!(trace(main_chain.len()), 5));
        assert!(matches!(trace(forks.get(&forkpoint, &main_endpoint)), None));
        assert!(matches!(trace(forks.get(&forkpoint, &endpoint)), Some(..)));
    }

    #[test]
    fn test_sync_to_fork_local() {}

    // /*****************************
    //  * Tests for automating the merging of forks *
    // *****************************/

    // fn test_sync_main(){
    //     let mut chain: Chain = Chain::genesis();
    //     for i in 1..CHAIN_LEN {
    //         chain.mine_block(&format!("block {}", i));
    //     }

    //     let mut forked_chain = {
    //         let mut f = chain.clone();
    //         let _ = f.split_off(FORK_PREFIX_LEN);
    //         f
    //     };
    //     {
    //         // chain: [0]---[1]---[2]---[3]---[4]
    //         // fork:               |----[*3*]---[*4*]
    //         forked_chain.mine_block("block 0 in fork");
    //         for i in 0..2 {
    //             forked_chain.mine_block(&format!("block {} in fork", i));
    //             let NextBlockResult::_ = chain.store_new_block(forked_chain.last().clone());
    //         }

    //         chain.sync_fork(fork_point, end_point)
    //         println!("Forked chain {}", forked_chain);
    //     }
    // }
}
