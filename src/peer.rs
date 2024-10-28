/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    - Manages a Swarm object (for communicating with peers in the network).
    - Manages std input events (for command-line interactions).
    - Manages a local Chain object (which it both adds new mined blocks to and synchronises with other peers' chains).
    - Manages a local Transaction pool (which it may mine new blocks for).
*/


use libp2p::{
    PeerId,
    futures::StreamExt,
    swarm::{Swarm, SwarmEvent},
};
use log::info;
use tokio::{io::AsyncBufReadExt, sync::mpsc::{self, UnboundedReceiver}};
use std::{collections::{HashMap, HashSet}, hash::Hash};

use crate::{block::NextBlockResult, chain::ChooseChainResult, swarm::connected_peers};

use super::file;
use super::block::{Block, NextBlockErr};
use super::chain::{self, Chain};
use super::transaction::Transaction;
use super::message::{PowMessage, TxnMessage, TransmitType};
use super::swarm::{self as swarm, BlockchainBehaviour};


const DEFAULT_FILE_PATH: &str = "blocks.json";

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
    (5) A map of disconnected forks. New entries are created when receiving blocks further ahead than the main chain.
    (6) A local transaction pool */
pub struct Peer {
    from_stdin : tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    pow_receiver : UnboundedReceiver<PowMessage>,
    txn_receiver : UnboundedReceiver<TxnMessage>,
    swarm : Swarm<BlockchainBehaviour>,
    chain : Chain,
    txn_pool : HashSet<Transaction>
}

impl Peer {
    /* Main loop -- Defines the logic for how the peer:
        1. Handles remote requests/responses from the network
        2. Handles local commands from the standard input   */
    pub async fn run(&mut self){
        println!("Enter `help` to see the command menu.");
        loop {
            let evt: Option<EventType> = {
                tokio::select! {
                    pow_event = self.pow_receiver.recv()
                        => Some(EventType::PowEvent(pow_event.expect("pow event exists"))),
                    txn_event = self.txn_receiver.recv()
                        => Some(EventType::TxnEvent(txn_event.expect("txn event exists"))),
                    std_event = self.from_stdin.next_line()
                        => Some(EventType::StdEvent(std_event.expect("can get line").expect("can read line from stdin"))),
                    swarm_event = self.swarm.select_next_some()
                        => { Self::handle_swarm_event(swarm_event); None }
                }
            };
            if let Some(event) = evt {
                println!("{} New Event {}", "-".repeat(40),  "-".repeat(40));
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
        println!("Received the following message:\n\
                  {}", msg);
        match msg {
            PowMessage::ChainRequest { sender_peer_id, .. } => {
                let resp = PowMessage::ChainResponse {
                    transmit_type: TransmitType::ToOne(sender_peer_id.clone()),
                    chain: self.chain.clone(),
                };
                swarm::publish_pow_msg(resp, &mut  self.swarm);
                println!("Broadcasted a ChainResponse with intended target:\n\
                         \t{}\nto connected peers:\n\t{:?}", sender_peer_id, swarm::connected_peers(&mut self.swarm));
            },
            PowMessage::ChainResponse{ chain , ..} => {
                match self.chain.sync_to_chain(chain){
                    Ok(res@ChooseChainResult::KeepMain{..}) => {
                        println!("Keeping main chain over remote peer's chain: \n\
                                    \t\"{}\"", res)
                    }
                    Ok(res@ChooseChainResult::ChooseOther{..}) => {
                        println!("Updated main chain to be remote peer's chain: \n\
                                    \t\"{}\"", res)
                    }
                    Err(e) => {
                        println!("Remote chain couldn't be validated:\n\
                                    \t\"{}\"", e)
                    }
                }
            },
            PowMessage::BlockRequest { sender_peer_id, block_hash, .. } => {
                if let Some(b)= self.chain.lookup_block_hash(&block_hash){
                        let resp = PowMessage::BlockResponse {
                            transmit_type: TransmitType::ToOne(sender_peer_id.clone()),
                            block: b.clone()
                        };
                        swarm::publish_pow_msg(resp, &mut self.swarm);
                        println!("Sent BlockResponse with target:\n\
                                 \t{}\n\
                                 broadcasted to connected peers:\n\
                                \t{:?}", sender_peer_id, swarm::connected_peers(&mut self.swarm));
                    }
                else {
                    println!("Couldn't lookup a BlockRequest in the main chain for the following hash:\n\
                                 \t\"{}\"", block_hash);
                }
            }
            PowMessage::BlockResponse { block, .. } => {
                // validate transaction inside the block, *if any*, and return early if invalid
                if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data){
                    match Transaction::validate_transaction(&txn) {
                        Ok (()) => {
                            println!("Successfully validated transaction inside the remote peer's block.")
                        }
                        Err (e) => {
                            println!("Couldn't validate transaction inside the remote peer's block, due to:\n\
                                    \t\"{}\"\n\
                                    Ignoring new block.", e);
                            return
                        }
                    }
                }
                match self.chain.store_orphan_block(block.clone()){
                    Ok(res) => {
                        println!("Orphan Block handled with update to chain or forks:\n\t\"{}\"", res);
                        if remove_from_pool(&mut self.txn_pool, &block){
                            println!("Deleted mined transaction from the local pool.");
                        }
                        // update the state of the main chain
                        match self.chain.handle_block_result(res) {
                            Ok(ChooseChainResult::ChooseOther { .. }) => {
                                    println!("Updated main chain to a longer Orphan fork.")
                            }
                            e => { println!("{:?}", e)
                            }
                        }
                    }
                    Err(e) => {
                        println!("Block handled with no update to chain or forks\n\t\"{}\"", e);
                        match e {
                            NextBlockErr::MissingParent { block_parent_hash,.. } =>
                                {
                                    let req = PowMessage::BlockRequest {
                                        transmit_type: TransmitType::ToAll,
                                        block_hash: block_parent_hash,
                                        sender_peer_id: self.swarm.local_peer_id().to_string()
                                    };
                                    swarm::publish_pow_msg(req, &mut self.swarm);
                                    println!("Sent BlockRequest for missing block:\n\
                                            \t{}\n\
                                            to:\n\
                                            \t{:?}", block.prev_hash, connected_peers(&mut self.swarm));
                                },
                            _ => {

                            }
                        }
                    }
                }
            }
            PowMessage::NewBlock { block, .. } => {
                // validate transaction inside the block, *if any*, and return early if invalid
                if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data){
                    match Transaction::validate_transaction(&txn) {
                        Ok (()) => {
                            println!("Successfully validated transaction inside the remote peer's block.")
                        }
                        Err (e) => {
                            println!("Couldn't validate transaction inside the remote peer's block, due to:\n\
                                    \t\"{}\"\n\
                                    Ignoring new block.", e);
                            return
                        }
                    }
                }
                // validate the block itself and store it
                match self.chain.store_new_block(block.clone()){
                    Ok(res) => {
                        println!("Block handled with update:\n\t\"{}\"", res);
                        if remove_from_pool(&mut self.txn_pool, &block){
                            println!("Deleted mined transaction from the local pool.");
                        }
                        match self.chain.handle_block_result(res) {
                            Ok(ChooseChainResult::ChooseOther { .. }) => {
                                    println!("Updated main chain to a longer fork.")
                            }
                            _ => {
                            }
                        }
                    }
                    Err(e) => {
                        println!("Block handled with no update to chain or forks, due to:\n\t\"{}\"", e);
                        match e {
                            NextBlockErr::MissingParent { block_parent_hash,.. } => {
                                let req = PowMessage::BlockRequest {
                                    transmit_type: TransmitType::ToAll,
                                    block_hash: block_parent_hash,
                                    sender_peer_id: self.swarm.local_peer_id().to_string()
                                };
                                swarm::publish_pow_msg(req, &mut self.swarm);
                                println!("Sent BlockRequest for missing block:\n\
                                        \t{}\n\
                                        to:\n\
                                        \t{:?}", block.prev_hash, connected_peers(&mut self.swarm));

                            },
                            _ => {}
                        }

                    }
                }
            }
        }
    }
    // Transaction event.
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
                        println!("Couldn't validate new transaction, due to:\n\
                                \t\"{}\"\n\
                                Ignoring new transaction.", e);
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
                let file_name = cmd.strip_prefix("load").expect("can strip `load`").trim();
                self.handle_cmd_load(file_name).await
            }
            // `save`, saves a chain from a local file.
            cmd if cmd.starts_with("save") => {
                let file_name = cmd.strip_prefix("save").expect("can strip `save`").trim();
                self.handle_cmd_save(file_name).await
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
                let arg = cmd.strip_prefix("req").expect("can strip `req`").trim();
                self.handle_cmd_req(arg)
            }
            // `mine [data]` makes and writes a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("mine") => {
                let arg = cmd.strip_prefix("mine").expect("can strip `mine`").trim();
                self.handle_cmd_mine(arg)
            }
            // `show <chain | peers | pool >` lists the local chain, discovered & connected peers, or transaction pool
            cmd if cmd.starts_with("show") => {
                let arg = cmd.strip_prefix("show").expect("can strip `show`").trim() ;
                self.handle_cmd_show(arg);
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
        if arg.is_empty() {
            println!("Command error: `req` missing an argument. Specify \"all\" or [peer_id]");
        }
        else {
            let txn: Transaction = Transaction::random_transaction(arg.to_string(), swarm::LOCAL_KEYS.clone());
            self.txn_pool.insert(txn.clone());
            println!("Added the new following new transaction to pool: \n{}\t", txn);
            let txn_msg: TxnMessage = TxnMessage::NewTransaction { txn };
            swarm::publish_txn_msg(txn_msg, &mut self.swarm);
            let connected_peers = swarm::connected_peers(&mut self.swarm);
            println!("Broadcasted new transaction to:\n\
                     \t{:?}", connected_peers);
        }
    }
    async fn handle_cmd_load(&mut self, file_name: &str){
        let file_name = if file_name.is_empty() { DEFAULT_FILE_PATH } else { &file_name };
        match file::read_chain(file_name).await {
            Ok(chain) => {
                self.chain = chain;
                println!("Loaded chain from local file \"{}\"", file_name)
            }
            Err(e) => eprintln!("Error loading chain from local file:\n\
                                                \"{}\"", e),
        }
    }
    async fn handle_cmd_save(&mut self, file_name: &str){
        let file_name = if file_name.is_empty() { DEFAULT_FILE_PATH } else { &file_name };
        match file::write_chain(&self.chain, file_name).await {
            Ok(()) => println!("Saved chain to local file \"{}\"", file_name),
            Err(e) => eprintln!("Error saving chain to local file:\n\
                                                \"{}\"", e),
        }
    }
    fn handle_cmd_reset(&mut self) {
        self.chain = chain::Chain::genesis();
        println!("Main chain reset to a single genesis block. Forks emptied.")
    }
    fn handle_cmd_mine(&mut self, args: &str) {
        let opt_data: Option<String> =
            // Retrieve data as the next transaction (as a string) from the pool
            if args.is_empty()  {
                extract_from_pool(&mut self.txn_pool)
                .map(|txn|
                    {   // assuming we can always safely serialize a transaction (which should be the case)..
                        println!("Retrieved and removed the following transaction from the pool:\n\
                                {}", txn);
                        serde_json::to_string(&txn).unwrap()
                    }
                )
            }
            // Use data as the provided cmd args
            else {
                Some (args.to_string())
            };
        match opt_data {
            None => eprintln!("No transactions in the pool to mine for."),
            Some(data) => {
                self.chain.mine_block(&data);
                println!("Mined and pushed the following new block to chain:\n\
                         {}", self.chain.last());

                swarm::publish_pow_msg(
                    PowMessage::NewBlock {
                        transmit_type: TransmitType::ToAll,
                        block: self.chain.last().clone()
                    }
                , &mut self.swarm);

                let connected_peers = swarm::connected_peers(&mut self.swarm);
                println!("Broadcasted new block to:\n\
                        \t{:?}", connected_peers);
            }
        }
    }
    fn handle_cmd_req(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `req` missing an argument. Specify \"all\" or [peer_id]");
            }
            "all" => {
                let req = PowMessage::ChainRequest {
                    transmit_type: TransmitType::ToAll,
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Sending ChainRequest for:\n\
                        \t<All Peers>\n\
                        broadcasting to:\n\
                        \t{:?}", swarm::connected_peers(&mut self.swarm));
                swarm::publish_pow_msg(req, &mut self.swarm);
            }
            peer_id => {
                let req = PowMessage::ChainRequest {
                    transmit_type: TransmitType::ToOne(peer_id.to_owned()),
                    sender_peer_id: self.swarm.local_peer_id().to_string(),
                };
                println!("Sending ChainRequest for \"{}\"\n\
                         broadcasting to:\n\
                         \t{:?}", peer_id, swarm::connected_peers(&mut self.swarm));
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
                println!("Current chain:\n\
                            {}", self.chain);
            }
            "forks"   => {
                println!("Current forks:\n");
                chain::show_forks(&self.chain);
            }
            "orphans"   => {
                println!("Current orphans:\n");
                chain::show_orphans(&self.chain);
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
                println!("Command error: `show` has unrecognised argument(s). Specify `chain`, `forks`, `peers`, or `pool`")
            }
        }
    }
    fn handle_cmd_redial(&mut self){
        let discovered_peers : Vec<libp2p::PeerId> = swarm::discovered_peers(&mut self.swarm);
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
                => println!("Connection established with: \n\
                            \t{:?}", peer_id),
            SwarmEvent::ConnectionClosed { peer_id, .. }
                => println!("Connection closed with: \n\
                            \t{:?}", peer_id),
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
        = match file::read_chain(DEFAULT_FILE_PATH).await {
            Err(e) => {
                eprintln!("\nProblem loading chain from the default file: \"{}\" \n\
                           Instantiating a fresh chain instead. ", e);
                Chain::genesis()
            }
            Ok(chain) => {
                println!("\nLoaded chain from default file \"{}\".", DEFAULT_FILE_PATH);
                chain
            }
        };

    println!("\n## Your Peer Id ##\n{}", swarm.local_peer_id().to_string());
    Peer { from_stdin
        , pow_receiver
        , txn_receiver
        , swarm
        , chain
        // , orphans: HashMap::new()
        , txn_pool: HashSet::new()
    }
}

fn remove_from_pool(txn_pool : &mut HashSet<Transaction>, block: &Block) -> bool {
    if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data){
        return txn_pool.remove(&txn)
    }
    false
}
fn extract_from_pool(txn_pool: &mut HashSet<Transaction>) -> Option<Transaction> {
    if let Some(txn) = txn_pool.iter().next() {
        // txn_pool.remove(&txn); //  doesn't work: we immutably borrowing txn_pool, via &txn, while mutably borrowing it, via txn_pool.remove(..)
        let txn_to_return = txn.clone(); // clone the value, so that we stop immutably borrowing txn_pool
        txn_pool.remove(&txn_to_return);
        Some(txn_to_return)
    } else {
        None // If the pool is empty
    }
}
fn peek_at_pool<'a>(txn_pool : &'a mut HashSet<Transaction>) -> Option<&'a Transaction> {
    txn_pool.iter().peekable().next()
}

fn print_user_commands(){
    let commands = include_str!("../commands.md");
    println!("{}", commands);
}