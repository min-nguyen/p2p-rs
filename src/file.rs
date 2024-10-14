/*
    *File*:
    --- Defines the types of core data
    --- IO for read and writing with it
*/

use log::info;
use tokio::fs;
use super::chain::Chain;

// reads all locally stored blocks
pub async fn read_chain(file_name: &str) -> Result<Chain, Box<dyn std::error::Error>> {
    let content: Vec<u8> = fs::read(file_name).await?;
    let result: Chain = serde_json::from_slice(&content)?;
    info!("read_local_blocks()");
    Ok(result)
}

// (over)writes all locally stored blocks
pub async fn write_chain(blocks: &Chain, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&blocks)?;
    fs::write(file_name, &json).await?;
    info!("write_local_chain()");
    Ok(())
}