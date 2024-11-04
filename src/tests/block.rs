/******************
      TESTS
********************/
#[cfg(test)] // cargo test block -- --nocapture
mod block_tests {
    use crate::{
        block::{Block, NextBlockErr},
        crypt::{encode_bytes_to_hex, ZERO_U32},
        util::trace,
    };

    #[test]
    fn test_invalid_block_difficulty_check() {
        let valid_block = Block::mine_block(&Block::genesis(), "test");

        let invalid_difficulty_prefix = Block {
            hash: hex::encode([255; 32]),
            ..valid_block.clone()
        };

        // Ensure that the block fails due to a difficulty check error
        assert!(matches!(
            invalid_difficulty_prefix.validate(),
            Err(NextBlockErr::DifficultyCheckFailed { .. })
        ));
    }
    #[test]
    fn test_invalid_block_hash_mismatch() {
        let valid_block = Block::mine_block(&Block::genesis(), "test");

        let invalid_hash = Block {
            hash: encode_bytes_to_hex(ZERO_U32),
            ..valid_block.clone()
        };

        assert!(matches!(
            trace(invalid_hash.validate()),
            Err(NextBlockErr::InconsistentHash { .. })
        ));
    }
    #[test]
    fn test_valid_block() {
        let valid_block = Block::mine_block(&Block::genesis(), "test");

        assert!(matches!(valid_block.validate(), Ok(())));
    }
}
