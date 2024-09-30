
use log::info;
use serde::{Deserialize, Serialize};
use tokio::fs;
use super::block::{Block};
pub const LOCAL_STORAGE_FILE_PATH: &str = "./blocks.json";

// reads all locally stored blocks
pub async fn read_local_blocks() -> Result<Vec<Block>, Box<dyn std::error::Error>> {
  let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
  let result: Vec<Block> = serde_json::from_slice(&content)?;
  info!("read_local_blocks()");
  Ok(result)
}

// appends to the locally stored blocks
pub async fn write_new_local_block(block: &Block) -> Result<(), Box<dyn std::error::Error>> {
  let mut local_blocks: Vec<Block> = read_local_blocks().await?;

  local_blocks.push(block.clone());
  write_local_blocks(&local_blocks).await?;

  info!("write_new_local_block(\"{}\")", block);
  Ok(())
}

// (over)writes all locally stored blocks
async fn write_local_blocks(blocks: &Vec<Block>) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(&blocks)?;
  fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
  Ok(())
}
