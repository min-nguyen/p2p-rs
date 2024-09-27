use libp2p::{
  core::upgrade,
  floodsub::{Floodsub, FloodsubEvent, Topic},
  futures::StreamExt,
  identity,
  mdns::{Mdns, MdnsEvent},
  mplex,
  noise::{Keypair, NoiseConfig, X25519Spec},
  swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
  tcp::TokioTcpConfig,
  NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use super::local_data;

/*
    *NetworkBehavior*: Defines the logic of the p2p network and all its peers.

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

// FloodSub Topic for subscribing and sending recipes
pub static RECIPE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

// Messages are either (1) requests for data or (2) responses to some arbitrary peer's request.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeRequest {
    // Requests for recipes can be either ToAll or ToOne
    pub transmit_type : TransmitType,
    // The PeerID the request came from.
    pub sender_peer_id : String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeResponse {
    // Responding with our recipes will always to be ToAll
    pub transmit_type : TransmitType,
    // Core message payload being transmitted in the network.
    pub data : local_data::Recipes,
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
pub struct RecipeBehaviour {
    // ** Relevant to the global P2P Network Behaviour that all peers must share:
    pub floodsub: Floodsub,
    mdns: Mdns,

    // ** Relevant only to a specific peer that we are setting up
    #[behaviour(ignore)]
    local_network_sender: mpsc::UnboundedSender<RecipeRequest>,
    #[behaviour(ignore)]
    local_peer_id: libp2p::PeerId
}

/*
Defining the Sub-Behaviours for handling events, `inject_event()`, from each Protocol Type.
    1. Sub-Behaviour for the mDNS Discovery Protocol.
    2. Concrete Sub-Behaviour for the FloodSub Communication Protocol.
*/
impl NetworkBehaviourEventProcess<MdnsEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    // Add to our list of peers to propagate messages to
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("local_network: Removed Peer");
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
impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            // Event for a new message from a peer
            FloodsubEvent::Message(msg) => {
                // Match on the deserialized payload as a RecipeResponse, which we print to console.
                if let Ok(resp) = serde_json::from_slice::<RecipeResponse>(&msg.data) {
                    if resp.receiver_peer_id == self.local_peer_id.to_string() {
                        info!("local_network: Response from {}:", msg.source);
                        resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                }
                // Match on the deserialized payload as a RecipeRequest, which we may forward to our local peer
                else if let Ok(req) = serde_json::from_slice::<RecipeRequest>(&msg.data) {
                    match req.transmit_type {
                        // Forward any ToAll requests to local peer
                        TransmitType::ToAll => {
                            info!("local_network: Received ToAll req {:?} from {:?}", req, msg.source);
                            info!("local_network: Forwarding ToAll req from {:?}", msg.source);
                            if let Err(e) = self.local_network_sender.send(req) {
                                error!("error sending response via channel, {}", e);
                            };
                        }
                        // Filter any ToOne requests if not intended for us, otherwise forwarding to local peer
                        TransmitType::ToOne(ref peer_id) => {
                            info!("Received ToOne req {:?} from {:?}", req, msg.source);
                            if peer_id == &self.local_peer_id.to_string() {
                                info!("local_network: Forwarding ToOne req from {:?}", msg.source);
                                if let Err(e) = self.local_network_sender.send(req) {
                                    error!("local_network: error sending response via channel, {}", e);
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

// Sets up a NetworkBehaviour that subscribes to the Recipe topic.
pub async fn set_up_recipe_behaviour
        (   local_peer_id : libp2p::PeerId
          , local_network_sender : mpsc::UnboundedSender<RecipeRequest>) -> RecipeBehaviour
{
  let mut behaviour = RecipeBehaviour {
      floodsub: Floodsub::new(local_peer_id.clone()),
      mdns: Mdns::new(Default::default())
          .await
          .expect("can create mdns"),
      local_network_sender,
      local_peer_id
  };

  behaviour.floodsub.subscribe(RECIPE_TOPIC.clone());
  behaviour
}
