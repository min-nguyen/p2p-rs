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

// # *NetworkBehavior* (key concept in p2p): Defines the logic of the p2p network and all its peers.
//
// We need to specify at least 2 Protocols:
//   1. Communication Protocol between peers
//   2. Discovery Protocol for peers to find each other
// We need to define 2 concrete network sub-behaviours:
//   1. Handling network events regarding the Communication Protocol
//   2. Handling network events regarding the Discovery Protocol
//
// We will use the FloodSub Communication Protocol.
// This is a publish-subscribe protocol:
// - Publishers send messages to all peers they are directly connected to, without any filtering.
// - Subscribers receive messages by subscribing to specific topics.
// - When a message is published, it is flooded to all peers in the network, and
//   each peer forwards the message to their connected peers until the message reaches all interested nodes.
//
// We will use the mDNS Discovery Protocol.
//

#[derive(NetworkBehaviour)]
pub struct RecipeBehaviour {
    // ** Relevant to the global P2P Network Behaviour that all peers must share:
    // 1. A Communication Protocol between peers on the network.
    //    We will use the FloodSub protocol to deal with events in the network.
    floodsub: Floodsub,
    // 2. A Discovery Protocol for discovering new peers
    //    We will use the mDNS protocol for discovering other peers on the local network.
    mdns: Mdns,

    // ** Relevant only to a specific peer that we are setting up, and Irrelevant to the NetworkBehaviour:
    // 1. A channel to receive responses *from* the network, and forward these *to* the main part of our application elsewhere.
    //    We will use `response_sender` to send responses from the network to some paired `response_rcv` elsewhere in our program.
    #[behaviour(ignore)]
    local_response_sender: mpsc::UnboundedSender<RecipeResponse>,
}
// Concrete Sub-Behaviour for the Discovery Protocol.

// Concrete Sub-Behaviour for the Communication Protocol.


// FloodSub Topic for subscribing and sending recipes
pub static RECIPE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

// Core message payload being transmitted in the network.
type Recipes = Vec<Recipe>;
#[derive(Debug, Serialize, Deserialize)]
struct Recipe {
    id : usize,
    name: String
}

// Messages are either (1) requests for data, or (2) responses to a request.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeRequest {
    mode : TransmitMode
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeResponse {
    mode : TransmitMode,
    data : Recipes,
    receiver : String
}

// Messages can be intended for all peers or a specific peer.
#[derive(Debug, Serialize, Deserialize)]
enum TransmitMode {
    ToAll,
    ToOne(String)
}

// Events (new messages) in the network are either (1) inputs from ourselves (2) responses from peers
enum EventType {
    Input(String),
    Response(RecipeResponse)
}

// Helper for setting up a Recipe NetworkBehaviour that subscribes to the Recipe topic.
pub async fn set_up_recipe_behaviour
        (   local_peer_id : Lazy<libp2p::PeerId>
          , local_response_sender : mpsc::UnboundedSender<RecipeResponse>)
{
  let mut behaviour = RecipeBehaviour {
      floodsub: Floodsub::new(local_peer_id.clone()),
      mdns: Mdns::new(Default::default())
          .await
          .expect("can create mdns"),
      local_response_sender,
  };

  // Subscribe our specific network behaviour to be subscribed to the "recipes" topic.
  behaviour.floodsub.subscribe(RECIPE_TOPIC.clone());
}
