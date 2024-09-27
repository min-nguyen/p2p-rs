// A peer-to-peer (P2P) network in which interconnected nodes ("peers") share resources amongst each other without the use of a centralized administrative system.
// [https://blog.logrocket.com/libp2p-tutorial-build-a-peer-to-peer-app-in-rust/]

use libp2p::{
    core::upgrade,
    futures::StreamExt,
    identity,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};
use super::{local_data, local_network::{self, RecipeResponse}, local_swarm};


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
    StdInput(String),
    LocalResponse(local_network::RecipeResponse)
}

pub struct Peer {
    local_receiver : UnboundedReceiver<RecipeResponse>,
    stdin : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    swarm : Swarm<local_network::RecipeBehaviour>
}

impl Peer {
    pub async fn handle_local_events(
        &mut self
        ){
        // Main loop,
        // -- Defines the logic for how the peer handles local events
        loop {
            // The select macro waits for several async processes, handling the first one that finishes.
            let evt = {
                tokio::select! {
                    //  StdIn Event, for a local user command.
                    stdin_event = self.stdin.next_line()
                        => Some(EventType::StdInput(stdin_event.expect("can get line").expect("can read line from stdin"))),
                    // LocalResponse Event;
                    local_response_event = self.local_receiver.recv()
                        => Some(EventType::LocalResponse(local_response_event.expect("response exists"))),
                    // Swarm Event, which we don't need to do anything with; these are handled within our RecipeBehaviour.
                    swarm_event = self.swarm.select_next_some()
                        => {
                            info!("Unhandled Swarm Event: {:?}", swarm_event);
                            None
                    },
                }
            };

            // If there is an event, we match on it and see if it’s a LocalResponse (to publish) or a StdIn event (to handle).
            if let Some(event) = evt {
                match event {
                    EventType::LocalResponse(resp) => {
                        local_swarm::publish_response(resp, &mut  self.swarm).await
                    }
                    EventType::StdInput(line) => match line.as_str() {
                        // 1. `ls` lists recipes
                        cmd if cmd.starts_with("ls")
                            => local_data::handle_list_recipes().await,
                        // 2. `create {recipeName}` creates a new recipe with the given name (and an incrementing id)
                        cmd if cmd.starts_with("create")
                            => local_data::handle_create_recipe(cmd).await,
                        // 3. `req <all | peer_id>` broadcasts a request for recipes
                        cmd if cmd.starts_with("req")
                            => local_swarm::handle_req_recipes(cmd, &mut self.swarm).await,
                        _ => error!("unknown command"),
                    },
                }
            }
        }
    }
}


pub async fn set_up_peer() -> Peer {
    pretty_env_logger::init();
    // Set up an Asynchronous channel to communicate between different parts of our application.
    // 1. local_sender is an output channel that we provide directly to behaviour.rs.
    //      After our behaviour receieves network events and handles them locally (e.g. by reading a file),
    //      , it will use the channel to send the results back (to here).
    // 2. local_receiver is an input channel that we use here.
    //      This will receive the results sent by local_sender,
    //      , whereby we can broadcast the results back to the p2p network
    let ( local_sender // used to send messages to response_rcv
        , local_receiver) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    let local_peer_id: PeerId
        = LOCAL_PEER_ID.clone();
    // Authentication keys for the `Noise` crypto-protocol
    // -- will be used to secure traffic within the p2p network
    let local_auth_keys
        = Keypair::<X25519Spec>::new()
        .into_authentic(&LOCAL_KEYS)
        .expect("can create auth keys");

    // Set up a *Transport* (a core concept in p2p):
    // -- Configuration for a network protocol.
    // -- used to enable connection-oriented communication between peers.
    // We will specifically use  TCP as the Transport.
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(local_auth_keys).into_authenticated())
        // We multiplex the transport,
        // -- used to enable multiple streams of data over one communication link.
        // -- which here, enables us to multiplex multiple connections on the transport.
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Set up a *NetworkBehaviour* (a core concept in p2p):
    // -- Defines the logic for how the peer interacts with the p2p network
    let behaviour
        = local_network::set_up_recipe_behaviour(local_peer_id, local_sender).await;

    // Set up a *Swarm* (a core concept in p2p):
    // -- Manages connections created with the Transport and executes our NetworkBehaviour
    // -- used to trigger and receive events from the network
    let mut swarm
        =   // Create a swarm with our Transport, NetworkBehaviour, and PeerID.
            SwarmBuilder::new(transp, behaviour, local_peer_id)
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
    let stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    info!("Peer Id: {}", local_peer_id);
    Peer { local_receiver, stdin, swarm }
}
