/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    ---
    ---
    ---
*/

use libp2p::{
    core::upgrade,
    futures::StreamExt,
    identity,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmEvent},
    tcp::TokioTcpConfig, PeerId, Transport,
};
use log::{debug, error, info};
use once_cell::sync::Lazy;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};

use super::file;
use super::network::{self, BlockRequest, BlockResponse, TransmitType};
use super::swarm;

/*  (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
    Below initialises these as global values that identify the current application (i.e. client) running.

    (1) Key Pair: Public & private key for secure communication with the rest of the network
    (2) PeerId: Unique hash of public key, used to identify the peer within the whole p2p network.
*/
static LOCAL_KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<libp2p::PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

/* Events for the peer to handle, either:
       (1) Local Inputs from the terminal
       (2) Remote Requests from peers in the network */
enum EventType {
    StdInput(String),
    NetworkRequest(network::BlockRequest)
}

/* A Peer consists of:
    (1) A channel to handle commands from standard input
    (2) A channel to handle requests  forwarded from the local network behaviour (but originating from the remote network)
    (3) A swarm to publish responses and requests to the remote network */
pub struct Peer {
    stdin_buff : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    local_network_receiver : UnboundedReceiver<BlockRequest>,
    swarm : Swarm<network::BlockchainBehaviour>
}

const USER_COMMANDS : [( &str, &str); 3]
= [("`req <all | [peer-id]>`", "Request data from (1) all peers, or (2) a specific known peer-id")
 , ("`create [data]: `", "Write a new piece of data to a local .json file")
 , ("`ls <peers | blocks>`", "Print a list of (1) discovered and connected peers, or (2) data in the local .json file")];

impl Peer {
    /* Main loop -- Defines the logic for how the peer:
        1. Handles remote requests from the network
        2. Handles local commands from the standard input   */
    pub async fn run(&mut self){
        loop {
            // The select macro waits for several async processes, handling the first one that finishes.
            let evt: Option<EventType> = {
                tokio::select! {
                    // StdIn Event for a local user command.
                    stdin_event = self.stdin_buff.next_line()
                        => Some(EventType::StdInput(stdin_event.expect("can get line").expect("can read line from stdin"))),
                    // NetworkRequest Event;
                    network_request = self.local_network_receiver.recv()
                        => Some(EventType::NetworkRequest(network_request.expect("response exists"))),
                    // Swarm Event; we don't need to explicitly do anything with it, and is handled by the BlockBehaviour.
                    swarm_event = self.swarm.select_next_some()
                        => { Peer::handle_swarm_event(swarm_event); None }
                }
            };
            if let Some(event) = evt {
                match event {
                    // Network Request from a remote user, requiring us to publish a Response to the network.
                    EventType::NetworkRequest(req)
                        => self.handle_network_event(&req).await,
                    // StdIn Event for a local user command.
                    EventType::StdInput(cmd)
                        => self.handle_stdin_event(&cmd).await
                }
            }
        }
    }

    // (Predefined) Swarm Event. For debugging purposes.
    fn handle_swarm_event<E : std::fmt::Debug>(swarm_event: SwarmEvent<(), E>) {
        match swarm_event {
            SwarmEvent::ConnectionEstablished { peer_id, .. }
                => info!("Connection established with peer: {:?}", peer_id),
            SwarmEvent::ConnectionClosed { peer_id, cause: Some(err), .. }
                => info!("Connection closed with peer: {:?}, cause: {:?}", peer_id, err),
            SwarmEvent::ConnectionClosed { peer_id, cause: None, .. }
                => info!("Connection closed with peer: {:?}", peer_id),
            _
                => info!("Unhandled swarm event: {:?}", swarm_event)
        }
    }
    // NetworkBehaviour Event for a local user command.
    async fn handle_stdin_event(&mut self, cmd: &str) {
        match cmd {
             // 1. `req <all | peer_id>`, requiring us to publish a Request to the network.
            cmd if cmd.starts_with("req") => {
                self.handle_request_command(cmd).await;
            }
            // 2. `create {string data}` creates a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("create") => {
                self.handle_create_command(cmd).await;
            }
              // 3. `ls <blocks | peers>` lists the local blocks or the discovered peers
            cmd if cmd.starts_with("ls") => {
                self.handle_ls_command(cmd).await;
            }
            _ => {
                error!("Unknown command: {}", cmd);
                println!("\nCommands:");
                USER_COMMANDS
                    .into_iter()
                    .map(|(command, description)|
                            println!("\n{}\n{}", description, command))
                    .collect()
            }
        }
    }
    // StdIn Event for a local user command.
    async fn handle_network_event(&mut self, req: &BlockRequest) {
        {
            match file::read_local_blocks().await {
                Ok(blocks) => {
                    let resp = BlockResponse {
                        transmit_type: TransmitType::ToAll,
                        receiver_peer_id: req.sender_peer_id.clone(),
                        data: blocks.into_iter().collect(),
                    };
                    swarm::publish_response(resp, &mut  self.swarm).await
                }
                Err(e) => error!("error fetching local blocks to answer request, {}", e),
            }
        }
    }
    async fn handle_request_command(&mut self, cmd: &str) {
        let args = cmd.strip_prefix("req").expect("can strip `req`").trim() ;
        match args {
            _ if args.is_empty() => {
                info!("Command error: [req] missing an argument, specify \"all\" or \"<peer_id>\"");
            }
            "all" => {
                let req = BlockRequest {
                    transmit_type: TransmitType::ToAll,
                    sender_peer_id: LOCAL_PEER_ID.to_string(),
                };
                swarm::publish_request(req, &mut self.swarm).await;
            }
            peer_id => {
                let req = BlockRequest {
                    transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                    sender_peer_id: LOCAL_PEER_ID.to_string(),
                };
                swarm::publish_request(req, &mut self.swarm).await;
            }
        }
    }
    async fn handle_create_command(&self, cmd: &str) {
        let args = cmd.strip_prefix("create").expect("can strip `create`").trim();
        match args {
            _ if args.is_empty() => {
                info!("Command error: [create] missing an argument (name)");
            }
            name => {
                if let Err(e) = file::write_new_local_block(name).await {
                    error!("error creating block: {}", e);
                }
            }
        }
    }
    async fn handle_ls_command(&mut self, cmd: &str) {
        let args: &str = cmd.strip_prefix("ls").expect("can strip `create`").trim();
        match args {
            _ if args.is_empty() => {
                info!("Command error: [ls] missing an argument `blocks` or `peers")
            }
            "blocks"   => {
                match file::read_local_blocks().await {
                    Ok(blocks) => {
                        info!("Local Blocks ({})", blocks.len());
                        blocks.iter().for_each(|r| info!("{:?}", r));
                    }
                    Err(e) => error!("error fetching local blocks: {}", e),
                };
            }
            "peers"   => {
                let (dscv_peers, conn_peers): (Vec<String>, Vec<String>)
                    = swarm::get_peers(&mut self.swarm);
                info!("Discovered Peers ({})", dscv_peers.len());
                dscv_peers.iter().for_each(|p| info!("{}", p));
                info!("Connected Peers ({})", conn_peers.len());
                conn_peers.iter().for_each(|p| info!("{}", p));
            }
            _ => {
                info!("Command error: [ls] missing an argument `blocks` or `peers")
            }
        }
    }
}


pub async fn set_up_peer() -> Peer {
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
        = network::set_up_block_behaviour(local_peer_id, local_network_sender).await;

    // Transport, which we multiplex to enable multiple streams of data over one communication link.
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Swarm,
    let swarm = swarm::set_up_swarm(transp, behaviour, local_peer_id);

    // Async Reader for StdIn, which reads the stream line by line.
    let stdin_buff = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    info!("Peer Id: {}", local_peer_id);
    Peer { local_network_receiver, stdin_buff, swarm }
}
