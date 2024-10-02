/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    ---
    ---
    ---
*/

use libp2p::{
    futures::{future::Either, StreamExt},
    swarm::{Swarm, SwarmEvent},
};
use log::debug;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};

use super::file;
use super::swarm::{self, BlockchainBehaviour, BlockchainMessage, BlockRequest, BlockResponse, TransmitType};
use super::block;

/* Events for the peer to handle, either:
       (1) Local Inputs from the terminal
       (2) Remote Requests/Responses from peers in the network */
enum EventType {
    StdInputEvent(String),
    NetworkEvent(BlockchainMessage)
}

/* A Peer consists of:
    (1) A channel to handle commands from standard input
    (2) A channel to handle requests/responses forwarded from the local network behaviour (but originating from the remote network)
    (3) A swarm to publish responses and requests to the remote network */
pub struct Peer {
    from_stdin : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    from_network_behaviour : UnboundedReceiver<Either<BlockRequest, BlockResponse>>,
    swarm : Swarm<BlockchainBehaviour>
}

impl Peer {
    /* Main loop -- Defines the logic for how the peer:
        1. Handles remote requests/responses from the network
        2. Handles local commands from the standard input   */
    pub async fn run(&mut self){
        print_user_commands();
        loop {
            // The select macro waits for several async processes, handling the first one that finishes.
            let evt: Option<EventType> = {
                tokio::select! {
                    stdin_event = self.from_stdin.next_line()
                        => Some(EventType::StdInputEvent(stdin_event.expect("can get line").expect("can read line from stdin"))),
                    network_request = self.from_network_behaviour.recv()
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
            SwarmEvent::NewListenAddr { address, .. }
                => println!("Swarm listening on {}", address),
            _
                => debug!("unhandled swarm event: {:?}", swarm_event)
        }
    }
    // NetworkBehaviour Event for a remote request.
    async fn handle_network_event(&mut self, msg: &BlockchainMessage) {
        match msg {
            Either::Left(req) => {
                println!("Received request:\n {:?}", req);
                match file::read_local_chain().await {
                    Ok(chain) => {
                        // println!("{}", chain);
                        let resp = BlockResponse {
                            transmit_type: TransmitType::ToAll,
                            receiver_peer_id: req.sender_peer_id.clone(),
                            data: chain.get_last_block().clone(),
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
            // 0. `fresh`, deletes the current local chain and writes a new one with a single block.
           cmd if cmd.starts_with("fresh") => {
                self.handle_fresh_command().await
           }
            // 1. `req <all | [peer_id]>`, requiring us to publish a Request to the network.
            cmd if cmd.starts_with("req") => {
                let args = cmd.strip_prefix("req").expect("can strip `req`").trim();
                self.handle_req_command(args).await;
            }
            // 2. `mk [data]` makes and writes a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("mk") => {
                let args = cmd.strip_prefix("mk").expect("can strip `mk`").trim();
                self.handle_mk_command( args).await;
            }
              // 3. `ls <blocks | peers>` lists the local blocks or the discovered peers
            cmd if cmd.starts_with("ls") => {
                let args = cmd.strip_prefix("ls").expect("can strip `ls`").trim() ;
                self.handle_ls_command( args).await;
            }
            _ => {
                println!("Unknown command: \"{}\"", cmd);
                print_user_commands();
            }
        }
    }
    async fn handle_fresh_command(&mut self) {
        let chain = block::Chain::new();
        match file::write_local_chain(&chain).await {
            Ok(()) => println!("Wrote fresh chain: {}", chain),
            Err(e) => eprintln!("error writing new valid block: {}", e),
        }
    }
    async fn handle_req_command(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `req` missing an argument, specify \"all\" or [peer_id]");
            }
            "all" => {
                let req: BlockRequest = BlockRequest {
                    transmit_type: TransmitType::ToAll,
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Broadcasting request to all");
                swarm::publish_request(req, &mut self.swarm).await;
            }
            peer_id => {
                let req: BlockRequest = BlockRequest {
                    transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Broadcasting request for \"{}\"", peer_id);
                swarm::publish_request(req, &mut self.swarm).await;
            }
        }
    }
    async fn handle_mk_command(&self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `mk` missing an argument [data]");
            }
            data => {
                match file::read_local_chain().await {
                    Ok(mut chain) => {
                        chain.make_new_valid_block(data);
                        match file::write_local_chain(&chain).await {
                            Ok(()) => println!("Mined and wrote new block: {:?}", chain.get_last_block()),
                            Err(e) => eprintln!("error writing new valid block: {}", e),
                        }
                    }
                    Err(e) => eprintln!("error fetching local blocks to answer request, {}", e),
                }
            }
        }
    }
    async fn handle_ls_command(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `ls` missing an argument `blocks` or `peers")
            }
            "blocks"   => {
                match file::read_local_chain().await {
                    Ok(blocks) => {
                       blocks.0.iter().for_each(|r| println!("{:?}", r))
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
    /* Asynchronous channel, to communicate between different parts of our application.
        1. to_peer is an output channel, provided to network.rs.
            After network receieves a remote message, it forwards any requests here back to the peer (from_network)
        2. from_network is an input channel, used by peer.rs
            Receive requests forwarded by to_peer, and handles them. */
    let ( to_local_peer // used to send messages to response_rcv
        , from_network_behaviour) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Swarm, with our network behaviour
    let swarm
        = swarm::set_up_swarm(to_local_peer).await;

    // Async Reader for StdIn, which reads the stream line by line.
    let from_stdin
        = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    println!("Peer Id: {}", swarm.local_peer_id().to_string());
    Peer { from_stdin, from_network_behaviour, swarm }
}

fn print_user_commands(){
    let commands = r#"
─────────────────────────────────────────────
    # Available Commands #
─────────────────────────────────────────────

📤 *Request data from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request last block from all peers
│     • `[peer-id]`  - Request last block from a specific peer

🔍 *Print a list*:
└── Usage: `ls <"peers" | "blocks">`
┌── Options:
│     • `"peers"`    - Show a list of connected remote peers
│     • `"blocks"`   - Show blocks stored in the local .json file

📝 *Write new data*:
└── Usage: `mk [data]`
┌── Description:
│     • Mine and write a new block to the local .json file.

📝 *Refresh data*:
└── Usage: `fresh`
┌── Description:
│     • Delete current blocks and write a new genesis block to the local .json file.
─────────────────────────────────────────────
"#;

    println!("{}", commands);
}