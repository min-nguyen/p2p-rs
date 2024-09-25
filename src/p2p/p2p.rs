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
use super::{recipes, behaviour};
// use tokio::fs

// * (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
// Below initialises these as global values that identify the current application (i.e. client) running.
//
// * A Key Pair enables us to communicate securely with the rest of the network, ensuring no one can impersonate us.
pub static LOCAL_KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
// * A PeerId is simply a unique identifier for a specific peer within the whole peer to peer network.
//   It is derived from a key pair to ensure uniqueness.
// #[derive(Clone)]
pub static LOCAL_PEER_ID: Lazy<libp2p::PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

// Events (new messages) in the network are either (1) inputs from ourselves (2) responses from peers
enum EventType {
    Input(String),
    Response(behaviour::RecipeResponse)
}

async fn set_up_peer() {
    // Set up an Asynchronous channel to communicate between different parts of our application.
    // 1. local_response_sender is an output channel that we provide directly to behaviour.rs.
    //      After our behaviour receieves network events and handles them locally (e.g. by reading a file),
    //      , it will use the channel to send the results back (to here).
    // 2. local_response_rcv is an input channel that we use here.
    //      This will receive the results sent by local_response_sender,
    //      , whereby we can broadcast the results back to the p2p network
    let (     local_response_sender // used to send messages to response_rcv
        , mut local_response_rcv) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Authentication keys for the `Noise` crypto-protocol
    // -- will be used to secure traffic within the p2p network
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&LOCAL_KEYS)
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
    // -- Defines the logic for how the peer interacts with the p2p network
    let mut behaviour
        = behaviour::set_up_recipe_behaviour(LOCAL_PEER_ID.clone(), local_response_sender)
          .await;

    // Set up a *Swarm* (a core concept in p2p):
    // -- Manages connections created with the Transport and executes our NetworkBehaviour
    // -- used to trigger and receive events from the network
    let mut swarm
        =   // Create a swarm with our Transport, NetworkBehaviour, and PeerID.
            SwarmBuilder::new(transp, behaviour, LOCAL_PEER_ID.clone())
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

    // Set up an async reader on the StdIn channel, which reads the stream line by line.
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    // Main loop,
    // -- Defines the logic for how the peer handles local events
    loop {
        // The select macro waits for several async processes, handling the first one that finishes.
        let evt = {
            tokio::select! {
                //  StdIn Event, for a local user command.
                stdin_event = stdin.next_line()
                    => Some(EventType::Input(stdin_event.expect("can get line").expect("can read line from stdin"))),
                // Local_Response_Rcv Event;
                //     this is from the channel that we originally forwarded network messages to,
                //     handles those messages by interacts with the local file system,
                //      and then responds with the appropriate data.
                local_response_event = local_response_rcv.recv()
                    => Some(EventType::Response(local_response_event.expect("response exists"))),
                // Swarm Event, which we don't need to do anything with; these are handled within our RecipeBehaviour.
                swarm_event = swarm.select_next_some()
                    => {
                        info!("Unhandled Swarm Event: {:?}", swarm_event);
                        None
                },
            }
        };

        // If there is an event, we match on it and see if it’s a Local_Response_Rcv or a StdIn event.
        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(behaviour::RECIPE_TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    cmd if cmd.starts_with("ls r") => behaviour::handle_list_recipes(cmd, &mut swarm).await,
                    cmd if cmd.starts_with("create r") => behaviour::handle_create_recipe(cmd).await,
                    _ => error!("unknown command"),
                },
            }
        }
    }
}