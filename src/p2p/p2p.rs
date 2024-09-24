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
use super::behaviour;
// use tokio::fs

// * (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
// Below initialises these as global values that identify the current application (i.e. client) running.
//
// * A Key Pair enables us to communicate securely with the rest of the network, ensuring no one can impersonate us.
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
// * A PeerId is simply a unique identifier for a specific peer within the whole peer to peer network.
//   It is derived from a key pair to ensure uniqueness.
pub static PEER_ID: Lazy<libp2p::PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));

async fn set_up_peer() {
    // Set up an Asynchronous channel to communicate between different parts of our application.
    // -- will be used to send responses *from* the p2p network (via response_sender),
    //                                   *to* our application to handle them (via response_rcv).
    let (     local_response_sender // used to send messages to response_rcv
        , mut local_response_rcv) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Authentication keys for the `Noise` crypto-protocol
    // -- will be used to secure traffic within the p2p network
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");

    // Set up a *Transport* (a core concept in p2p):
    // -- Configuration for a network protocol.
    // -- used to enable connection-oriented communication between peers.
    // We will specifically use  TCP as the Transport.
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        // We multiplex the transport,
        // -- used to enable multiple streams of data over one communication link.
        // -- which here, enables us to multiplex multiple connections on the transport.
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Set up a *NetworkBehaviour* (a core concept in p2p):
    // -- Defines the logic of the p2p network and all its peers
    let mut behaviour = behaviour::set_up_recipe_behaviour(PEER_ID, local_response_sender);

    // Subscribe our specific network behaviour to be subscribed to the "recipes" topic.
    behaviour.floodsub.subscribe(TOPIC.clone());

    // Set up a *Swarm* (a core concept in p2p):
    // -- Manages connections created with the Transport and executes our NetworkBehaviour
    // -- used to trigger and receive events from the network
    let mut swarm
        =   // Create a swarm with our Transport, NetworkBehaviour, and PeerID.
            SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

    // Start our Swarm to listen to a local IP (port decided by the OS) using our set up.
    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");
}

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
