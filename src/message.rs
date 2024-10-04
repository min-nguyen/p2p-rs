use libp2p::futures;
use serde::{Deserialize, Serialize};

use super::block;


// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String) // receiving peer id
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BlockMessage {
  Request{
    transmit_type : TransmitType,
    sender_peer_id : String
  },
  Response {
    transmit_type : TransmitType,
    data : block::Block
  }
}

// #[derive(Debug, Serialize, Deserialize)]
// pub enum ChainMessage {
//   Request{
//     transmit_type : TransmitType,
//     sender_peer_id : String
//   },
//   Response{
//     transmit_type : TransmitType,
//     data : block::Chain
//   }
// }