/*
    *Message*: Provides the message forms communicated between peers.
    - Messages for requesting and responding with chains or new blocks.
    - Messages for broadcasting new transactions.
*/

use serde::{Deserialize, Serialize};
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
    BlockRequest {
       transmit_type : TransmitType,  // ToOne or ToAll
       block_hash : String,
       sender_peer_id : String
    },
    BlockResponse {
        transmit_type : TransmitType,   // always ToOne
        data : block::Block
    },
    NewBlock {
        transmit_type : TransmitType, // always ToAll
        block : block::Block
    },
}

impl std::fmt::Display for PowMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PowMessage::ChainRequest { transmit_type, sender_peer_id } =>
                write!(f, "ChainRequest {{\n Transmit Type: {:?}, Sender Peer Id: {} }}", transmit_type, sender_peer_id),
            PowMessage::ChainResponse { transmit_type, chain } =>
                write!(f, "ChainResponse {{\n Transmit Type: {:?},\n Chain: {} }}", transmit_type, chain),
            PowMessage::BlockRequest { transmit_type, block_hash, sender_peer_id } =>
                write!(f, "BlockRequest {{\n Transmit Type: {:?}, Block Hash: {}, Sender Peer Id: {} }}", transmit_type, block_hash, sender_peer_id),
            PowMessage::BlockResponse { transmit_type, data } =>
                write!(f, "BlockResponse {{\n Transmit Type: {:?}, Block: {} }}", transmit_type, data),
            PowMessage::NewBlock { transmit_type, block } =>
                write!(f, "NewBlock {{\n Transmit Type: {:?},\n Block: {} }}", transmit_type, block),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction
    }
}
