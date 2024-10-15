
/******************
      TESTS
********************/
#[cfg(test)] // cargo test block -- --nocapture
mod block_tests {
    use crate::{
        block::{Block, BlockErr},
        cryptutil::{debug, encode_bytes_to_hex, ZERO_U32}};

    #[test]
    fn test_invalid_block_difficulty_check() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        let invalid_difficulty_prefix = Block {
            hash: hex::encode([255; 32]),
            ..valid_block.clone()
        };

        // Ensure that the block fails due to a difficulty check error
        assert!(matches!(
            Block::validate_block(&invalid_difficulty_prefix),
            Err(BlockErr::DifficultyCheckFailed { .. })
        ));
    }
    #[test]
    fn test_invalid_block_hash_mismatch() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        let invalid_hash = Block {
            hash: encode_bytes_to_hex(ZERO_U32),
            ..
            valid_block.clone()
        };

        assert!(matches!(
            debug(Block::validate_block(&invalid_hash)),
            Err(BlockErr::HashMismatch { .. })
        ));
    }
    #[test]
    fn test_valid_block() {
        let valid_block = Block::mine_block(1, "test", &Block::genesis().hash);

        assert!(matches!(
            Block::validate_block(&valid_block),
            Ok(())
        ));
    }
}
