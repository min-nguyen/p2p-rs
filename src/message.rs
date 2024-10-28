/*
    *Message*: Provides the message forms communicated between peers.
    - Messages for requesting and responding with chains or new blocks.
    - Messages for broadcasting new transactions.
*/

use serde::{Deserialize, Serialize};
use crate::cryptutil::pretty_hex;

use super::block;
use super::chain;
use super::transaction;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PowMessage {
    ChainRequest {
        target : Option<String>,      // ToOne or ToAll
        source : String
    },
    ChainResponse {
        target : String,
        source : String,
        chain : chain::Chain
    },
    BlockRequest {
        target: Option<String>,       // ToOne or ToAll
        source : String,
        block_idx: usize,
        block_hash : String
    },
    BlockResponse {
        target : String,
        source : String,
        block : block::Block,
    },
    NewBlock {                         // ToAll
        source : String,
        block : block::Block,
    },
}

impl PowMessage {
    pub fn source(&self) -> &String {
        match self {
            PowMessage::ChainRequest { source, .. }
            | PowMessage::ChainResponse { source, .. }
            | PowMessage::BlockRequest { source, .. }
            | PowMessage::BlockResponse { source, .. }
            | PowMessage::NewBlock { source, .. } => source,
        }
    }
}

impl std::fmt::Display for PowMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PowMessage::ChainRequest {..} =>
                write!(f, "Chain request"),
            PowMessage::ChainResponse { chain, .. } =>
                write!(f, "Chain response that has length {}", chain.len()),
            PowMessage::NewBlock {  block , ..} =>
                write!(f, "New block with idx {}", block.idx),
            PowMessage::BlockRequest {  block_idx, block_hash, .. } =>
                write!(f, "Block request for idx {} with hash {}", block_idx, pretty_hex(block_hash)),
            PowMessage::BlockResponse {  block, .. } =>
                write!(f, "Block response for idx {} with hash {}", block.idx, pretty_hex(&block.hash)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction,
        source : String
    }
}

impl TxnMessage {
    pub fn source(&self) -> &String {
        match &self {
            TxnMessage::NewTransaction { source, .. } => source
        }
    }
}
impl std::fmt::Display for TxnMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TxnMessage::NewTransaction {txn, ..} =>
                write!(f, "New transaction\n{}", txn),
        }
    }
}