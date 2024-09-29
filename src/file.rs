/*
    *File*:
    --- Defines the types of core data
    --- IO for read and writing with it
*/

use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio::fs;

pub const LOCAL_STORAGE_FILE_PATH: &str = "./blocks.json";

pub type Blocks = Vec<Block>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    id : usize,
    body: String
}

// reads all locally stored blocks
pub async fn read_local_blocks() -> Result<Blocks, Box<dyn std::error::Error>> {
  let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
  let result: Blocks = serde_json::from_slice(&content)?;
  info!("read_local_blocks()");
  Ok(result)
}

// appends to the locally stored blocks
pub async fn write_new_local_block(block_body: &str) -> Result<Block, Box<dyn std::error::Error>> {
  let mut local_blocks = read_local_blocks().await?;
  let new_id = match local_blocks.iter().max_by_key(|r| r.id) {
      Some(v) => v.id + 1,
      None => 0,
  };
  let new_block = Block {
    id: new_id,
    body: block_body.to_owned()
  };
  local_blocks.push(new_block.clone());
  write_local_blocks(&local_blocks).await?;

  info!("write_new_local_block(\"{}\")", block_body);
  Ok(new_block)
}

// (over)writes all locally stored blocks
async fn write_local_blocks(blocks: &Blocks) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(&blocks)?;
  fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
  Ok(())
}
