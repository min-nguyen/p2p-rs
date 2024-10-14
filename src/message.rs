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
    ChainRequest {
        transmit_type : TransmitType,
        sender_peer_id : String
    },
    ChainResponse {
        transmit_type : TransmitType,
        chain : chain::Chain
    },
    NewBlock {
        transmit_type : TransmitType, // always ToAll
        block : block::Block
    }
    // NewBlockProposal {
    //   transmit_type : TransmitType, // always ToAll
    //   data : block::Block
    // },
    // NewBlockValidation {
    //   transmit_type : TransmitType, // always ToAll
    //   data : block::Block
    // }
}

impl std::fmt::Display for PowMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
        PowMessage::ChainRequest { transmit_type, sender_peer_id }
            => write!(f, "ChainRequest {{ Transmit Type: {:?}, Sender Peer Id: {} }}", transmit_type, sender_peer_id),
        PowMessage::ChainResponse { transmit_type, chain }
            => write!(f, "ChainResponse {{\n Transmit Type: {:?},\n Data: {} \n}}", transmit_type, chain),
        PowMessage::NewBlock { transmit_type, block }
            => write!(f, "NewBlock {{\n\t Transmit Type: {:?},\n\t Data: {} \n}}", transmit_type, block),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction
    }
}
