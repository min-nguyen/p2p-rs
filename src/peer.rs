/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    ---
    ---
    ---
*/

use libp2p::{
    core::upgrade,
    futures::{future::Either, StreamExt},
    identity,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmEvent},
    tcp::TokioTcpConfig, PeerId, Transport,
};
use log::debug;
use once_cell::sync::Lazy;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};

use crate::network::BlockchainMessage;

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
    StdInputEvent(String),
    NetworkEvent(BlockchainMessage)
}

/* A Peer consists of:
    (1) A channel to handle commands from standard input
    (2) A channel to handle requests  forwarded from the local network behaviour (but originating from the remote network)
    (3) A swarm to publish responses and requests to the remote network */
pub struct Peer {
    from_stdin : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    from_network : UnboundedReceiver<Either<BlockRequest, BlockResponse>>,
    swarm : Swarm<network::BlockchainBehaviour>
}

impl Peer {
    /* Main loop -- Defines the logic for how the peer:
        1. Handles remote requests from the network
        2. Handles local commands from the standard input   */
    pub async fn run(&mut self){
        print_user_commands();
        loop {
            // The select macro waits for several async processes, handling the first one that finishes.
            let evt: Option<EventType> = {
                tokio::select! {
                    stdin_event = self.from_stdin.next_line()
                        => Some(EventType::StdInputEvent(stdin_event.expect("can get line").expect("can read line from stdin"))),
                    network_request = self.from_network.recv()
                        => Some(EventType::NetworkEvent(network_request.expect("response exists"))),
                    // Swarm Event; we don't need to explicitly do anything with it, and is handled by the BlockBehaviour.
                    swarm_event = self.swarm.select_next_some()
                        => { Self::handle_swarm_event(swarm_event); None }
                }
            };
            if let Some(event) = evt {
                match event {
                    // Network Request from a remote user, requiring us to publish a Response to the network.
                    EventType::NetworkEvent(req)
                        => self.handle_network_event(&req).await,
                    // StdIn Event for a local user command.
                    EventType::StdInputEvent(cmd)
                        => self.handle_stdin_event(&cmd).await
                }
            }
        }
    }

    // (Predefined) Swarm Event. For debugging purposes.
    fn handle_swarm_event<E : std::fmt::Debug>(swarm_event: SwarmEvent<(), E>) {
        match swarm_event {
            SwarmEvent::ConnectionEstablished { peer_id, .. }
                => debug!("connection established with peer: {:?}", peer_id),
            SwarmEvent::ConnectionClosed { peer_id, cause: Some(err), .. }
                => debug!("connection closed with peer: {:?}, cause: {:?}", peer_id, err),
            SwarmEvent::ConnectionClosed { peer_id, cause: None, .. }
                => debug!("connection closed with peer: {:?}", peer_id),
            _
                => debug!("unhandled swarm event: {:?}", swarm_event)
        }
    }
    // NetworkBehaviour Event for a remote request.
    async fn handle_network_event(&mut self, msg: &BlockchainMessage) {
        match msg {
            Either::Left(req) => {
                println!("Received request:\n {:?}", req);
                match file::read_local_blocks().await {
                    Ok(blocks) => {
                        let resp = BlockResponse {
                            transmit_type: TransmitType::ToAll,
                            receiver_peer_id: req.sender_peer_id.clone(),
                            data: blocks.into_iter().collect(),
                        };
                        swarm::publish_response(resp, &mut  self.swarm).await
                    }
                    Err(e) => eprintln!("error fetching local blocks to answer request, {}", e),
                }
            },
            Either::Right(rsp) => {
                println!("Received response:\n {:?}", rsp);
            }
        }
    }
    // StdIn Event for a local user command.
    async fn handle_stdin_event(&mut self, cmd: &str) {
        match cmd {
             // 1. `req <all | [peer_id]>`, requiring us to publish a Request to the network.
            cmd if cmd.starts_with("req") => {
                self.handle_req_command(cmd).await;
            }
            // 2. `mk [data]` makes and writes a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("mk") => {
                self.handle_mk_command(cmd).await;
            }
              // 3. `ls <blocks | peers>` lists the local blocks or the discovered peers
            cmd if cmd.starts_with("ls") => {
                self.handle_ls_command(cmd).await;
            }
            _ => {
                println!("Unknown command: \"{}\"", cmd);
                print_user_commands();
            }
        }
    }
    async fn handle_req_command(&mut self, cmd: &str) {
        let args = cmd.strip_prefix("req").expect("can strip `req`").trim() ;
        match args {
            _ if args.is_empty() => {
                println!("Command error: `req` missing an argument, specify \"all\" or [peer_id]");
            }
            "all" => {
                let req: BlockRequest = BlockRequest {
                    transmit_type: TransmitType::ToAll,
                    sender_peer_id: LOCAL_PEER_ID.to_string(),
                };
                println!("Broadcasting request to all");
                swarm::publish_request(req, &mut self.swarm).await;
            }
            peer_id => {
                let req: BlockRequest = BlockRequest {
                    transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                    sender_peer_id: LOCAL_PEER_ID.to_string(),
                };
                println!("Broadcasting request for \"{}\"", peer_id);
                swarm::publish_request(req, &mut self.swarm).await;
            }
        }
    }
    async fn handle_mk_command(&self, cmd: &str) {
        let args = cmd.strip_prefix("mk").expect("can strip `mk`").trim();
        match args {
            _ if args.is_empty() => {
                println!("Command error: `mk` missing an argument [data]");
            }
            name => {
                match file::write_new_local_block(name).await {
                    Ok(b) => println!("Made new block: {:?}", b),
                    Err(e) => eprintln!("Error creating block: {}", e),
                }
            }
        }
    }
    async fn handle_ls_command(&mut self, cmd: &str) {
        let args: &str = cmd.strip_prefix("ls").expect("can strip `ls`").trim();
        match args {
            _ if args.is_empty() => {
                println!("Command error: `ls` missing an argument `blocks` or `peers")
            }
            "blocks"   => {
                match file::read_local_blocks().await {
                    Ok(blocks) => {
                       blocks.iter().for_each(|r| println!("{:?}", r))
                    }
                    Err(e) => eprintln!("error fetching local blocks: {}", e),
                };
            }
            "peers"   => {
                let (dscv_peers, conn_peers): (Vec<String>, Vec<String>)
                    = swarm::get_peers(&mut self.swarm);
                println!("Discovered Peers ({})", dscv_peers.len());
                dscv_peers.iter().for_each(|p| println!("{}", p));
                println!("Connected Peers ({})", conn_peers.len());
                conn_peers.iter().for_each(|p| println!("{}", p));
            }
            _ => {
                println!("Command error: `ls` has unrecognised argument(s). Specify `blocks` or `peers")
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
        1. to_peer is an output channel, provided to network.rs.
            After network receieves a remote message, it forwards any requests here back to the peer (from_network)
        2. from_network is an input channel, used by peer.rs
            Receive requests forwarded by to_peer, and handles them. */
    let ( to_peer // used to send messages to response_rcv
        , from_network) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Network Behaviour,
    let behaviour
        = network::set_up_block_behaviour(local_peer_id, to_peer).await;

    // Transport, which we multiplex to enable multiple streams of data over one communication link.
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Swarm,
    let swarm = swarm::set_up_swarm(transp, behaviour, local_peer_id);

    // Async Reader for StdIn, which reads the stream line by line.
    let from_stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    println!("Peer Id: {}", local_peer_id);
    Peer { from_stdin, from_network, swarm }
}

fn print_user_commands(){
    let commands = r#"
─────────────────────────────────────────────
    # Available Commands #
─────────────────────────────────────────────

📤 *Request data from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request data from all peers
│     • `[peer-id]`  - Request data from a specific peer

🔍 *Print a list*:
└── Usage: `ls <"peers" | "blocks">`
┌── Options:
│     • `"peers"`    - Show a list of connected remote peers
│     • `"blocks"`   - Show data stored in the local .json file

📝 *Write new data*:
└── Usage: `mk [data]`
┌── Description:
│     • Write a new piece of data to the local .json file.
─────────────────────────────────────────────
"#;

    println!("{}", commands);
}
