/*
    *File*: Provides auxiliary access to local storage.
    - Functions for loading and saving the blockchain state (from `blocks.json`).
*/

use super::{
    chain::Chain,
    block::Block,
};
use log::info;
use tokio::fs;


// reads all locally stored blocks
pub async fn read_chain(file_name: &str) -> Result<Chain, Box<dyn std::error::Error>> {
    let content: Vec<u8> = fs::read(file_name).await?;
    let blocks: Vec<Block> = serde_json::from_slice(&content)?;
    let chain: Chain = Chain::from_vec(blocks)?;
    info!("read_local_blocks()");
    Ok(chain)
}

// (over)writes all locally stored blocks
pub async fn write_chain(chain: &Chain, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let blocks: Vec<Block> = chain.clone().to_vec();
    let json: String = serde_json::to_string(&blocks)?;
    fs::write(file_name, &json).await?;
    info!("write_local_chain()");
    Ok(())
}