use libp2p::futures;
use serde::{Deserialize, Serialize};

use super::block;


// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String) // contains intended peer id
}

// Messages are either (1) requests for data or (2) responses to some arbitrary peer's request.
#[derive(Debug, Serialize, Deserialize)]
pub enum BlockMessage {
  BlockRequest{
    // Requests for blocks can be either ToAll or ToOne
    transmit_type : TransmitType,
    // The PeerID the request came from.
    sender_peer_id : String
  },
  BlockResponse{
    // Responses for blocks are ToOne
    transmit_type : TransmitType,
    // Core message payload being transmitted in the network.
    data : block::Block
  }
}