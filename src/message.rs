use serde::{Deserialize, Serialize};

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
        data : chain::Chain
    },
    NewBlock {
        transmit_type : TransmitType, // always ToAll
        data : chain::Block
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
        PowMessage::ChainResponse { transmit_type, data }
            => write!(f, "ChainResponse {{\n Transmit Type: {:?},\n Data: {} \n}}", transmit_type, data),
        PowMessage::NewBlock { transmit_type, data }
            => write!(f, "NewBlock {{\n\t Transmit Type: {:?},\n\t Data: {} \n}}", transmit_type, data),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxnMessage {
    NewTransaction {
        txn : transaction::Transaction
    }
}
