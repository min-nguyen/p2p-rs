// A peer-to-peer (P2P) network in which interconnected nodes ("peers") share resources amongst each other without the use of a centralized administrative system.
// [https://blog.logrocket.com/libp2p-tutorial-build-a-peer-to-peer-app-in-rust/]

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
// use tokio::fs

// * (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
// Below initialises these as global values that identify the current application (i.e. client) running.
//
// * A Key Pair enables us to communicate securely with the rest of the network, ensuring no one can impersonate us.
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
// * A PeerId is simply a unique identifier for a specific peer within the whole peer to peer network.
//   It is derived from a key pair to ensure uniqueness.
pub static PEER_ID: Lazy<libp2p::PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));

// * A Topic is a named channel that we can subscribe to and send messages on, in order
//   to only listen to a subset of the traffic on a pub/sub network.
// We will subscribe to the "recipe" topic and use it to send our local receipe to other peers, and to receive theirs.
pub static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

// Core data being transmitted in the network.
type Recipes = Vec<Recipe>;
#[derive(Debug, Serialize, Deserialize)]
struct Recipe {
    id : usize,
    name: String
}

// Messages are either (1) requests for data, or (2) responses to a request.
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    mode : TransmitMode
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
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
    Response(Response)
}

async

// The core of the P2P functionality is implementing a NetworkBehaviour,
//   this defines the logic of the network and all peers, e.g. what to do with incoming events, which events to send.
// #[derive(NetworkBehaviour)]
// pub struct ... {
//     // what to do with incoming events
//     // a flood publish/subscribe protocol for communications between nodes.
//     //  this means every node must broadcast its data to all connnected nodes (not efficient)
//     pub floodsub: Floodsub,
//     // how to discover node peers
//     //    mdns is a protocol for discovering other peers on the network
//     pub mdns: Mdns,
//     #[behaviour(ignore)]
//     // a channel to send responses across to the main part of the application
//     pub response_sender: mpsc::UnboundedSender<ChainResponse>,
// }

// Defines how nodes in the network discover over nodes
// impl NetworkBehaviourEventProcess<MdnsEvent> for ... {

// }

// // Defines how nodes in the network handle incoming events
// impl NetworkBehaviourEventProcess<FloodsubEvent> for ... {

// }
