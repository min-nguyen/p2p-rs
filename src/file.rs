/*
    *File*:
    --- Defines the types of core data
    --- IO for read and writing with it
*/

use log::info;
use tokio::fs;
use super::chain::{Chain, Block};
pub const LOCAL_STORAGE_FILE_PATH: &str = "./blocks.json";

// reads all locally stored blocks
pub async fn read_chain() -> Result<Chain, Box<dyn std::error::Error>> {
    let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
    let result: Chain = serde_json::from_slice(&content)?;
    info!("read_local_blocks()");
    Ok(result)
}

// appends to the locally stored blocks
pub async fn write_block(block: &Block) -> Result<(), Box<dyn std::error::Error>> {
    let mut local_chain: Chain = read_chain().await?;

    local_chain.0.push(block.clone());
    write_chain(&local_chain).await?;

    info!("write_local_block(\"{}\")", block);
    Ok(())
}

// (over)writes all locally stored blocks
pub async fn write_chain(blocks: &Chain) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&blocks)?;
    fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
    info!("write_local_chain()");
    Ok(())
}