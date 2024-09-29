/*
    *NetworkBehavior*: High-level abstraction that defines how a peer *should* behave on the network.
    -- Configures the protocols and messages of the p2p network used by all peers.
    -- Each peer owns a local NetworkBehaviour that receives events/messages from other peers.

    We need to specify at least 2 Protocol Types:
        1. Communication Protocol between peers
        2. Discovery Protocol for peers to find each other
    We need to define a Concrete Network Sub-Behaviour for each Protocol Type:
        1. Handling network events regarding the Communication Protocol
        2. Handling network events regarding the Discovery Protocol

    We will use the FloodSub Communication Protocol, a publish-subscribe protocol:
        - Publishers send messages to *all* peers they are directly connected to, without any filtering.
        - Subscribers receive messages by subscribing to specific topics.
        - When a message is published, it is flooded to all peers in the network, and
            each peer forwards the message to their connected peers until the message reaches all interested nodes.

    We will use the mDNS Discovery Protocol.
*/

use libp2p::{
  floodsub::{Floodsub, FloodsubEvent, Topic}, futures::future::Either, mdns::{Mdns, MdnsEvent}, swarm::NetworkBehaviourEventProcess, NetworkBehaviour
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::file;

// FloodSub Topic for subscribing and sending blocks
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

// Messages are either (1) requests for data or (2) responses to some arbitrary peer's request.
pub type BlockchainMessage = Either<BlockRequest, BlockResponse>;
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockRequest {
    // Requests for blocks can be either ToAll or ToOne
    pub transmit_type : TransmitType,
    // The PeerID the request came from.
    pub sender_peer_id : String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockResponse {
    // Responding with our blocks will always to be ToAll
    pub transmit_type : TransmitType,
    // Core message payload being transmitted in the network.
    pub data : file::Blocks,
    // The PeerID to recieve the response.
    pub receiver_peer_id : String
}
// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String)   // contains intended peer id
}

// Base NetworkBehaviour, simply specifying the 2 Protocol types
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    // ** Relevant to the global P2P Network Behaviour that all peers must share:
    pub floodsub: Floodsub,
    pub mdns: Mdns,

    // ** Relevant only to a specific local peer that we are setting up
    #[behaviour(ignore)]
    to_peer: mpsc::UnboundedSender<BlockchainMessage>,
    #[behaviour(ignore)]
    peer_id: libp2p::PeerId
}

/*
Defining the Sub-Behaviours for handling events, `inject_event()`, from each Protocol Type.
    1. Sub-Behaviour for the mDNS Discovery Protocol.
    2. Concrete Sub-Behaviour for the FloodSub Communication Protocol.
*/
impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    info!("discovered peer: {}", peer);
                    // Add to our list of peers to propagate messages to
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("removed peer: {}", peer);
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
            //
            // MdnsEvent
        }
    }
}
impl NetworkBehaviourEventProcess<FloodsubEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            // Event for a new message from a peer
            FloodsubEvent::Message(msg) => {
                // Match on the deserialized payload as a BlockResponse, which we print to console.
                if let Ok(resp) = serde_json::from_slice::<BlockResponse>(&msg.data) {
                    if resp.receiver_peer_id == self.peer_id.to_string() {
                        info!("response from {}:", msg.source);
                        self.to_peer.send(Either::Right(resp));
                    }
                }
                // Match on the deserialized payload as a BlockRequest, which we may forward to our local peer
                else if let Ok(req) = serde_json::from_slice::<BlockRequest>(&msg.data) {
                    match req.transmit_type {
                        // Forward any ToAll requests to local peer
                        TransmitType::ToAll => {
                            info!("received req {:?} from {:?}", req, msg.source);
                            info!("forwarding req from {:?}", msg.source);
                            if let Err(e) = self.to_peer.send(Either::Left(req)) {
                                error!("error sending response via channel, {}", e);
                            };
                        }
                        // Filter any ToOne requests if not intended for us, otherwise forwarding to local peer
                        TransmitType::ToOne(ref peer_id) => {
                            info!("received req {:?} from {:?}", req, msg.source);
                            if peer_id == &self.peer_id.to_string() {
                                info!("forwarding req from {:?}", msg.source);
                                if let Err(e) = self.to_peer.send(Either::Left(req)) {
                                    error!("error sending response via channel, {}", e);
                                };
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

// Sets up a NetworkBehaviour that subscribes to the Blocks topic.
pub async fn set_up_block_behaviour
        (   peer_id : libp2p::PeerId
          , to_peer : mpsc::UnboundedSender<BlockchainMessage>) -> BlockchainBehaviour
{
  let mut behaviour = BlockchainBehaviour {
      floodsub: Floodsub::new(peer_id.clone()),
      mdns: Mdns::new(Default::default())
          .await
          .expect("can create mdns"),
      to_peer,
      peer_id
  };

  behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
  behaviour
}
