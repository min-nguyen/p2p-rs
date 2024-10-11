/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    ---
    ---
    ---
*/


use libp2p::{
    PeerId,
    futures::StreamExt,
    swarm::{Swarm, SwarmEvent},
};
use log::info;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};
use std::collections::{HashMap, HashSet};

use super::file;
use super::chain::{self, Chain, Block};
use super::transaction::Transaction;
use super::message::{PowMessage, TxnMessage, TransmitType};
use super::swarm::{self as swarm, BlockchainBehaviour};
// use super::swarm_flood::{self as swarm, BlockchainBehaviour};

/* Events for the peer to handle, either:
    (1) Local inputs from the terminal
    (2) Remote chain messages from miners in the network
    (3) Remote transaction messages from peers in the network
*/
enum EventType {
    StdEvent(String),
    PowEvent(PowMessage),
    TxnEvent(TxnMessage)
}

/* A Peer consists of:
    (1) A channel to handle commands from standard input
    (2) A channel to receive blockchain requests/responses forwarded from the network behaviour
    (3) A channel to receive transaction messages forwarded from the network behaviour
    (4) A local blockchain
    (3) A local transaction pool, identified by their hashes */
pub struct Peer {
    from_stdin : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    pow_receiver : UnboundedReceiver<PowMessage>,
    txn_receiver : UnboundedReceiver<TxnMessage>,
    swarm : Swarm<BlockchainBehaviour>,
    chain : Chain,
    txn_pool  : HashSet<Transaction>
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
                    pow_event = self.pow_receiver.recv()
                        => Some(EventType::PowEvent(pow_event.expect("pow event exists"))),
                    txn_event = self.txn_receiver.recv()
                        => Some(EventType::TxnEvent(txn_event.expect("txn event exists"))),
                    std_event = self.from_stdin.next_line()
                        => Some(EventType::StdEvent(std_event.expect("can get line").expect("can read line from stdin"))),
                    // Swarm Event; we don't need to explicitly do anything with it, and is handled by the BlockBehaviour.
                    swarm_event = self.swarm.select_next_some()
                        => { Self::handle_swarm_event(swarm_event); None }
                }
            };
            if let Some(event) = evt {
                match event {
                    EventType::PowEvent(msg)
                        => self.handle_pow_event(msg),
                    EventType::TxnEvent(msg)
                        => self.handle_txn_event(msg),
                    EventType::StdEvent(cmd)
                        => self.handle_std_event(&cmd).await,
                }
            }
        }
    }
    // Blockchain event.
    fn handle_pow_event(&mut self, msg: PowMessage) {
        println!("Received message:\n {}", msg);
        match msg {
            PowMessage::ChainRequest { sender_peer_id, .. } => {
                let resp = PowMessage::ChainResponse {
                    transmit_type: TransmitType::ToOne(sender_peer_id.clone()),
                    chain: self.chain.clone(),
                };
                println!("Sent response to {}", sender_peer_id);
                swarm::publish_pow_msg(resp, &mut  self.swarm)
            },
            PowMessage::ChainResponse{ chain , ..} => {
                if self.chain.sync_chain(&chain){
                    println!("Updated current chain to a remote peer's longer chain")
                }
                else {
                    println!("Retained current chain over a remote peer's chain")
                }
            },
            PowMessage::NewBlock { block, .. } => {
                // Validate transaction inside the block, *if any*, and return early if invalid
                if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data){
                    match Transaction::validate_transaction(&txn) {
                        Ok (()) => {
                            println!("Successfully validated transaction inside the received block.")
                        }
                        Err (e) => {
                            println!("Couldn't validate transaction inside the received block:\n\t{}\nIgnoring new block.", e);
                            return
                        }
                    }
                }
                // Validate block itself
                match self.chain.try_push_block(&block){
                    Ok(()) =>{
                        println!("Successfully validated block itself.\n\
                                 Extended current chain by a remote peer's new block");
                        if remove_from_pool(&mut self.txn_pool, &block){
                            println!("Deleted mined transaction from the local pool.");
                        }
                    }
                    Err(e) =>
                        println!("Retained current chain and ignored remote peer's new block:\n\t{}", e)
                }
            }
        }
    }
    /* TODO */
    fn handle_txn_event(&mut self, msg: TxnMessage) {
        match msg {
            TxnMessage::NewTransaction { txn } => {
                println!("Received new transaction:\n{}", txn);
                match Transaction::validate_transaction(&txn) {
                    Ok (()) => {
                        println!("Transaction valid! Adding to pool");
                        self.txn_pool.insert(txn);
                    }
                    Err (e) => {
                        println!("Transaction not valid:\n\t{}\nIgnoring new transaction.", e);
                        return
                    }
                }
            }
        }
    }
    // Stdin event for a local user command.
    async fn handle_std_event(&mut self, cmd: &str) {
        match cmd {
            // `reset`, deletes the current local chain and writes a new one with a single block.
            cmd if cmd.starts_with("reset") => {
                self.handle_cmd_reset()
            }
            // `load`, loads a chain from a local file.
            cmd if cmd.starts_with("load") => {
                self.handle_cmd_load().await
            }
            // `save`, saves a chain from a local file.
            cmd if cmd.starts_with("save") => {
                self.handle_cmd_save().await
            }
            // `redial`, dial all discovered peers
            cmd if cmd.starts_with("redial") => {
                self.handle_cmd_redial()
            },
            cmd if cmd.starts_with("help") => {
                 print_user_commands();
            },
            //`req <all | [peer_id]>`, requiring us to publish a ChainRequest to the network.
            cmd if cmd.starts_with("req") => {
                let args = cmd.strip_prefix("req").expect("can strip `req`").trim();
                self.handle_cmd_req(args)
            }
            // `mine [data]` makes and writes a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("mine") => {
                let args = cmd.strip_prefix("mine").expect("can strip `mine`").trim();
                self.handle_cmd_mine(args)
            }
            // `show <chain | peers>` lists the local chain or the discovered & connected peers
            cmd if cmd.starts_with("show") => {
                let args = cmd.strip_prefix("show").expect("can strip `show`").trim() ;
                self.handle_cmd_show(args);
            }
            // `txn [data]`, broadcasts a random transaction with the "amount" set to [data]
            cmd if cmd.starts_with("txn") => {
                let arg = cmd.strip_prefix("txn").expect("can strip `txn`").trim();
                self.handle_cmd_txn(arg);
            }
            _ => {
                println!("Unknown command: \"{}\" \nWrite `help` to show available commands.", cmd);
            }
        }
    }
    fn handle_cmd_txn(&mut self, arg: &str) {
        let txn: Transaction = Transaction::random_transaction(arg.to_string(), swarm::LOCAL_KEYS.clone());

        self.txn_pool.insert(txn.clone());
        println!("Added new transaction to pool. \n{}\t", txn);
        let txn_msg: TxnMessage = TxnMessage::NewTransaction { txn };
        swarm::publish_txn_msg(txn_msg, &mut self.swarm);
        println!("Broadcasted new transaction to to all.");
    }
    async fn handle_cmd_load(&mut self){
        match file::read_chain().await {
            Ok(chain) => {
                self.chain = chain;
                println!("Loaded chain from local file")
            }
            Err(e) => eprintln!("Error loading chain from local file:\n\t{}", e),
        }
    }
    async fn handle_cmd_save(&mut self ){
        match file::write_chain(&self.chain).await {
            Ok(()) => println!("Saved chain to local file"),
            Err(e) => eprintln!("Error saving chain to local file:\n\t{}", e),
        }
    }
    fn handle_cmd_reset(&mut self) {
        self.chain = chain::Chain::new();
        println!("Current chain reset to a single block")
    }
    fn handle_cmd_mine(&mut self, args: &str) {
        let opt_data: Option<String> =
            // Retrieve data as the next transaction (as a string) from the pool
            if args.is_empty()  {
                peek_at_pool(&mut self.txn_pool)
                .map(|txn|
                    {   // assuming we can always safely serialize a transaction (which should be the case)..
                        println!("Retrieved transaction from the pool:\n\t{}", txn);
                        serde_json::to_string(txn).unwrap()
                    }
                )
            // Use data as the provided cmd args
            } else {
                Some (args.to_string())
            };
        match opt_data {
            None => eprintln!("No transactions in the pool to mine for."),
            Some(data) => {
                self.chain.make_new_valid_block(&data);
                let last_block = self.chain.get_last_block().to_owned();
                println!("Mined and pushed new block to chain: {:?}", last_block);

                if remove_from_pool(&mut self.txn_pool, &last_block) {
                    println!("Deleted mined transaction from the local pool.")
                }

                swarm::publish_pow_msg(
                    PowMessage::NewBlock {
                        transmit_type: TransmitType::ToAll,
                        block: last_block
                    }
                , &mut self.swarm);
                println!("Broadcasted new block.");
            }
        }
    }
    fn handle_cmd_req(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `req` missing an argument, specify \"all\" or [peer_id]");
            }
            "all" => {
                let req = PowMessage::ChainRequest {
                    transmit_type: TransmitType::ToAll,
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Broadcasting request to all");
                swarm::publish_pow_msg(req, &mut self.swarm);
            }
            peer_id => {
                let req = PowMessage::ChainRequest {
                    transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Broadcasting request for \"{}\"", peer_id);
                swarm::publish_pow_msg(req, &mut self.swarm);
            }
        }
    }
    fn handle_cmd_show(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `show` missing an argument `chain`, `peers`, or `pool`")
            }
            "chain"   => {
                println!("Current chain:\n");
                self.chain.0.iter().for_each(|block| println!("{}", block))
            }
            "peers"   => {
                let (dscv_peers, conn_peers): (Vec<PeerId>, Vec<PeerId>)
                    = swarm::get_peers(&mut self.swarm);
                println!("Discovered Peers ({})", dscv_peers.len());
                dscv_peers.iter().for_each(|p| println!("{}", p));
                println!("Connected Peers ({})", conn_peers.len());
                conn_peers.iter().for_each(|p| println!("{}", p));
            }
            "pool"   => {
                println!("Current transaction pool:\n");
                self.txn_pool.iter().for_each(|txn| println!("{}", txn))
            }
            _ => {
                println!("Command error: `show` has unrecognised argument(s). Specify `chain`, `peers`, or `pool`")
            }
        }
    }
    fn handle_cmd_redial(&mut self){
        let discovered_peers : Vec<libp2p::PeerId> = swarm::get_peers(&mut self.swarm).0;
        if discovered_peers.is_empty() {
            println!("No discovered peers to dial!");
            return ()
        }
        for peer_id in discovered_peers {
            match self.swarm.dial(&peer_id){
                Ok(()) => println!("Dial for {}", peer_id),
                Err(e) => eprintln!("Dial error {}", e)
            }
        }
    }
    // (Predefined) Swarm event. For debugging purposes.
    fn handle_swarm_event<E : std::fmt::Debug>(swarm_event: SwarmEvent<(), E>) {
        match swarm_event {
            SwarmEvent::ConnectionEstablished { peer_id, .. }
                => info!("SwarmEvent: connection established with peer: {:?}", peer_id),
            SwarmEvent::ConnectionClosed { peer_id, cause: Some(err), .. }
                => info!("SwarmEvent: connection closed with peer: {:?}, cause: {:?}", peer_id, err),
            SwarmEvent::ConnectionClosed { peer_id, cause: None, .. }
                => info!("SwarmEvent: connection closed with peer: {:?}", peer_id),
            SwarmEvent::NewListenAddr { listener_id, address, .. }
                => info!("SwarmEvent: {:?} listening on {}", listener_id, address),
            SwarmEvent::Dialing(peer_id)
                => info!("SwarmEvent: dialling {:?} ", peer_id),
            SwarmEvent::IncomingConnection { local_addr, send_back_addr }
                => info!("SwarmEvent: incoming connection on addr {:?} with send-back addr {}", local_addr, send_back_addr),
            _
                => info!("Unhandled swarm event: {:?}", swarm_event)
        }
    }
}

pub async fn set_up_peer() -> Peer {
    /* Asynchronous channel, to communicate between different parts of our application.
        1. to_peer is an output channel, provided to network.rs.
            After network receieves a remote message, it forwards any requests here back to the peer (from_network)
        2. from_network is an input channel, used by peer.rs
            Receive requests forwarded by to_peer, and handles them. */
    let ( pow_sender // used to send messages to response_rcv
        , pow_receiver) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();
    let ( txn_sender // used to send messages to response_rcv
        , txn_receiver) // used to receive the messages sent by response_sender.
        = mpsc::unbounded_channel();

    // Swarm, with our network behaviour
    let swarm
        = swarm::set_up_blockchain_swarm(pow_sender, txn_sender).await;

    // Async Reader for StdIn, which reads the stream line by line.
    let from_stdin
        = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    // Load chain from local file
    let chain: Chain
        = match file::read_chain().await {
            Err(e) => {
                eprintln!("\nProblem loading chain from the local file: \"{}\" \n\
                           Instantiating a fresh chain instead. ", e);
                Chain::new()
            }
            Ok(chain) => {
                println!("\nLoaded chain from local file.\n");
                chain
            }
        };

    println!("\n## Your Peer Id: ##\n{}", swarm.local_peer_id().to_string());
    Peer { from_stdin
        , pow_receiver
        , txn_receiver
        , swarm
        , chain
        , txn_pool: HashSet::new()
    }
}

fn remove_from_pool(txn_pool : &mut HashSet<Transaction>, block: &Block) -> bool {
    if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data){
        return txn_pool.remove(&txn)
    }
    false
}

fn peek_at_pool<'a>(txn_pool : &'a mut HashSet<Transaction>) -> Option<&'a Transaction> {
    txn_pool.iter().peekable().next()
}

fn print_user_commands(){
    let commands = include_str!("../commands.md");
    println!("{}", commands);
}