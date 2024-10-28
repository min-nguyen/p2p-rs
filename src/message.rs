/*
    *Message*: Provides the message forms communicated between peers.
    - Messages for requesting and responding with chains or new blocks.
    - Messages for broadcasting new transactions.
*/

use serde::{Deserialize, Serialize};
use crate::chain::abbreviate_chain;
use crate::cryptutil::pretty_hex;

use super::block;
use super::chain;
use super::transaction;

// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String) // receiving peer id
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PowMessage {
    ChainRequest {                    // ToOne or ToAll
        transmit_type : TransmitType,
        sender_peer_id : String
    },
    ChainResponse {                   // always ToOne
        transmit_type : TransmitType,
        chain : chain::Chain
    },
    NewBlock {
        transmit_type : TransmitType, // always ToAll
        block : block::Block
    },
    BlockRequest {
        transmit_type : TransmitType,  // ToOne or ToAll
        block_idx: usize,
        block_hash : String,
        sender_peer_id : String
    },
    BlockResponse {
        transmit_type : TransmitType,   // always ToOne
        block : block::Block
    },
}

impl std::fmt::Display for PowMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PowMessage::ChainRequest {  sender_peer_id , ..} =>
                write!(f, "Chain request from {}", sender_peer_id),
            PowMessage::ChainResponse { chain, .. } =>
                write!(f, "Chain response that has length {} ", chain.len()),
            PowMessage::NewBlock {  block , ..} =>
                write!(f, "New block with idx {} }}", block.idx),
            PowMessage::BlockRequest {  block_idx, block_hash, sender_peer_id, .. } =>
                write!(f, "Block request for idx {} with hash {} from PeerId({}) ", block_idx, pretty_hex(block_hash), pretty_hex(sender_peer_id)),
            PowMessage::BlockResponse {  block, .. } =>
                write!(f, "Block response for idx {} with hash {}", block.idx, pretty_hex(&block.hash)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction
    }
}
