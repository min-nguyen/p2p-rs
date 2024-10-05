use serde::{Deserialize, Serialize};

use super::block;


// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String) // receiving peer id
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
  ChainRequest {
    transmit_type : TransmitType,
    sender_peer_id : String
  },
  ChainResponse {
    transmit_type : TransmitType,
    data : block::Chain
  },
  NewBlock {
    transmit_type : TransmitType, // always ToAll
    data : block::Block
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

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Message::ChainRequest { transmit_type, sender_peer_id }
        => write!(f, "ChainRequest {{ Transmit Type: {:?}, Sender Peer Id: {} }}", transmit_type, sender_peer_id),
      Message::ChainResponse { transmit_type, data }
        => write!(f, "ChainResponse {{\n Transmit Type: {:?},\n Data: {} \n}}", transmit_type, data),
      Message::NewBlock { transmit_type, data }
        => write!(f, "NewBlock {{\n\t Transmit Type: {:?},\n\t Data: {} \n}}", transmit_type, data),
    }
  }
}
