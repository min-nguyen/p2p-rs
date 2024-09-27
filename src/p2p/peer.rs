/*
A peer-to-peer (P2P) network in which interconnected nodes ("peers") share resources
amongst each other without the use of a centralized administrative system.
// [https://blog.logrocket.com/libp2p-tutorial-build-a-peer-to-peer-app-in-rust/]
*/

use libp2p::{
    core::upgrade,
    futures::StreamExt,
    identity,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm},
    tcp::TokioTcpConfig, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};
use super::{local_data::{self, read_local_recipes, write_new_local_recipe}, local_network::{self, RecipeRequest, RecipeResponse, TransmitType}, local_swarm};


/*
    *Peer*:



*/


/*  (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
    Below initialises these as global values that identify the current application (i.e. client) running.

    (1) Key Pair enables us to communicate securely with the rest of the network, ensuring no one can impersonate us.
    (2) PeerId is a unique identifier for a peer within the whole p2p network. Derived from a key pair to ensure uniqueness.  */
static LOCAL_KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<libp2p::PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

/* Events for the peer to handle, either:
       (1) inputs from ourselves
       (2) requests from peers in the network */
enum EventType {
    StdInput(String),
    NetworkRequest(local_network::RecipeRequest)
}

/* A Peer consists of:
    (1) A channel to handle commands from standard input
    (2) A channel to handle requests  forwarded from the local network behaviour (but originating from the remote network)
    (3) A swarm to publish responses and requests to the remote network */
pub struct Peer {
    stdin_buff : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    local_network_receiver : UnboundedReceiver<RecipeRequest>,
    swarm : Swarm<local_network::RecipeBehaviour>
}

impl Peer {
    pub async fn handle_local_events(&mut self){
        /* Main loop -- Defines the logic for how the peer:
            1. Handles remote requests from the network
            2. Handles local commands from the standard input   */
        loop {
            // The select macro waits for several async processes, handling the first one that finishes.
            let evt = {
                tokio::select! {
                    // StdIn Event for a local user command.
                    stdin_event = self.stdin_buff.next_line()
                        => Some(EventType::StdInput(stdin_event.expect("can get line").expect("can read line from stdin"))),
                    // NetworkRequest Event;
                    network_request = self.local_network_receiver.recv()
                        => Some(EventType::NetworkRequest(network_request.expect("response exists"))),
                    // Swarm Event, which we don't need to do anything with; these are handled within our RecipeBehaviour.
                    swarm_event = self.swarm.select_next_some()
                        => {
                            info!("Unhandled Swarm Event: {:?}", swarm_event);
                            None
                    },
                }
            };

            if let Some(event) = evt {
                match event {
                    // Network Request from a remote user, requiring us to publish a Response to the network.
                    EventType::NetworkRequest(req) => {
                        match local_data::read_local_recipes().await {
                            Ok(recipes) => {
                                let resp = RecipeResponse {
                                    transmit_type: TransmitType::ToAll,
                                    receiver_peer_id: req.sender_peer_id,
                                    data: recipes.into_iter().collect(),
                                };
                                local_swarm::publish_response(resp, &mut  self.swarm).await
                            }
                            Err(e) => error!("error fetching local recipes to answer request, {}", e),
                        }
                    }
                    // StdIn Event for a local user command.
                    EventType::StdInput(line) => match line.as_str() {
                        // 1. `req <all | peer_id>`, requiring us to publish a Request to the network.
                        cmd if cmd.starts_with("req")
                            => {
                                let args: Option<&str>
                                    = cmd.strip_prefix("req")
                                            .map(|rest: &str| rest.trim());
                                match args {
                                    Some("") | None => {
                                        info!("Command error: [req] missing an argument, specify \"all\" or \"<peer_id>\"")
                                    }
                                    // `req r all` send a request for all recipes from all known peers
                                    Some("all") => {
                                        let req = RecipeRequest {
                                            transmit_type: TransmitType::ToAll,
                                            sender_peer_id: LOCAL_PEER_ID.to_string()
                                        };
                                        local_swarm::publish_request(req, &mut self.swarm).await
                                    }
                                    // `req r <peerId>` sends a request for all recipes from a certain peer
                                    Some(peer_id) => {
                                        let req = RecipeRequest {
                                            transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                                            sender_peer_id: LOCAL_PEER_ID.to_string()
                                        };
                                        local_swarm::publish_request(req, &mut self.swarm).await
                                    }
                                };
                            }
                        // 2. `create {recipeName}` creates a new recipe with the given name (and an incrementing id)
                        cmd if cmd.starts_with("create")
                            => {
                                let args: Option<&str>
                                    = cmd.strip_prefix("create")
                                        .map(|rest: &str| rest.trim());
                                match args {
                                    Some("") | None => {
                                        info!("Command error: [create] missing an argument (name)")
                                    }
                                    // `req r all` send a request for all recipes from all known peers
                                    Some(name) => {
                                        if let Err(e) = write_new_local_recipe(name).await {
                                            error!("error creating recipe: {}", e);
                                        };
                                    }
                                }
                            }
                        // 3. `ls` lists recipes
                        cmd if cmd.starts_with("ls")
                            => {
                                match read_local_recipes().await {
                                    Ok(v) => {
                                        info!("Local Recipes ({})", v.len());
                                        v.iter().for_each(|r| info!("{:?}", r));
                                    }
                                    Err(e) => error!("error fetching local recipes: {}", e),
                                };
                            }
                        _ => error!("unknown command"),
                    },
                }
            }
        }
    }
}


pub async fn set_up_peer() -> Peer {
    // Peer Id, a unique hash of the local peer's public key
    let local_peer_id: PeerId
        = LOCAL_PEER_ID.clone();
    // Authentication keys, for the `Noise` crypto-protocol, used to secure traffic within the p2p network
    let local_auth_keys
        = Keypair::<X25519Spec>::new()
        .into_authentic(&LOCAL_KEYS)
        .expect("can create auth keys");
    /* Asynchronous channel, to communicate between different parts of our application.
        1. local_sender is an output channel, provided to local_network.rs.
            After network receieves a remote message, it forwards any requests here back to the peer (local_receiver)
        2. local_receiver is an input channel, used by peer.rs
            Receive requests forwarded by local_sender, and handles them. */
    let ( local_network_sender // used to send messages to response_rcv
        , local_network_receiver) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Network Behaviour,
    let behaviour
        = local_network::set_up_recipe_behaviour(local_peer_id, local_network_sender).await;

    // Transport, which we multiplex to enable multiple streams of data over one communication link.
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Swarm,
    let swarm = local_swarm::set_up_swarm(transp, behaviour, local_peer_id);

    // Async Reader for StdIn, which reads the stream line by line.
    let stdin_buff = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    info!("Peer Id: {}", local_peer_id);
    Peer { local_network_receiver, stdin_buff, swarm }
}
