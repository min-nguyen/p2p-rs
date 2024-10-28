/*
    *Message*: Provides the message forms communicated between peers.
    - Messages for requesting and responding with chains or new blocks.
    - Messages for broadcasting new transactions.
*/

use serde::{Deserialize, Serialize};
use crate::chain::abbreviate_chain;

use super::block;
use super::chain;
use super::transaction;

// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String) // receiving peer id
}

#[derive(Debug, Serialize, Deserialize)]
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
                write!(f, "Chain response {} ", abbreviate_chain(chain)),
            PowMessage::NewBlock { transmit_type, block } =>
                write!(f, "NewBlock {{\n Transmit Type: {:?},\n Block: {} }}", transmit_type, block),
            PowMessage::BlockRequest { transmit_type, block_idx, block_hash, sender_peer_id } =>
                write!(f, "BlockRequest {{\n Transmit Type: {:?}, Block Idx {}, Block Hash: {}, Sender Peer Id: {} }}", transmit_type, block_idx, block_hash, sender_peer_id),
            PowMessage::BlockResponse { transmit_type, block } =>
                write!(f, "BlockResponse {{\n Transmit Type: {:?}, Block: {} }}", transmit_type, block),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction
    }
}
