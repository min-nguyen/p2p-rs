/*
    *Peer*: The peer's logic on the local machine, which the entire application is architected around.
    - Manages a Swarm object (for communicating with peers in the network).
    - Manages std input events (for command-line interactions).
    - Manages a local Chain object (which it both adds new mined blocks to and synchronises with other peers' chains).
    - Manages a local Transaction pool (which it may mine new blocks for).
*/

use super::{
    block::{Block, NextBlockErr, NextBlockResult},
    chain::{self, Chain},
    file,
    message::{PowMessage, TxnMessage},
    swarm::{self as swarm, BlockchainBehaviour},
    transaction::Transaction,
    util::abbrev,
};
use libp2p::{
    futures::StreamExt,
    swarm::{Swarm, SwarmEvent},
    PeerId,
};
use log::info;
use std::collections::HashSet;
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{self, UnboundedReceiver},
};

const DEFAULT_FILE_PATH: &str = "blocks.json";

/* Events for the peer to handle, either:
    (1) Local inputs from the terminal
    (2) Remote chain messages from miners in the network
    (3) Remote transaction messages from peers in the network
*/
enum EventType {
    Std(String),
    Pow(PowMessage),
    Txn(TxnMessage),
}

/* A Peer consists of:
(1) A channel to handle commands from standard input
(2) A channel to receive blockchain requests/responses forwarded from the network behaviour
(3) A channel to receive transaction messages forwarded from the network behaviour
(4) A local blockchain
(5) A map of disconnected forks. New entries are created when receiving blocks further ahead than the main chain.
(6) A local transaction pool */
pub struct Peer {
    from_stdin: tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    pow_receiver: UnboundedReceiver<PowMessage>,
    txn_receiver: UnboundedReceiver<TxnMessage>,
    swarm: Swarm<BlockchainBehaviour>,
    chain: Chain,
    txns: HashSet<Transaction>,
}

impl Peer {
    /* Main loop -- Defines the logic for how the peer:
    1. Handles remote requests/responses from the network
    2. Handles local commands from the standard input   */
    pub async fn run(&mut self) {
        println!("Enter `help` to see the command menu.");
        loop {
            let evt: Option<EventType> = {
                tokio::select! {
                    pow_event = self.pow_receiver.recv()
                        => Some(EventType::Pow(pow_event.expect("pow event exists"))),
                    txn_event = self.txn_receiver.recv()
                        => Some(EventType::Txn(txn_event.expect("txn event exists"))),
                    std_event = self.from_stdin.next_line()
                        => Some(EventType::Std(std_event.expect("can get line").expect("can read line from stdin"))),
                    swarm_event = self.swarm.select_next_some()
                        => { Self::handle_swarm_event(swarm_event); None }
                }
            };
            if let Some(event) = evt {
                println!("{} New Event {}", "-".repeat(40), "-".repeat(40));
                match event {
                    EventType::Pow(msg) => self.handle_pow_event(msg),
                    EventType::Txn(msg) => self.handle_txn_event(msg),
                    EventType::Std(cmd) => self.handle_std_event(&cmd).await,
                }
            }
        }
    }
    // Blockchain event.
    fn handle_pow_event(&mut self, msg: PowMessage) {
        received!("\"{}\" from PeerId({})", msg, abbrev(msg.source()));
        match msg.clone() {
            // PowMessage::ChainRequest { .. } => {
            //     let resp: PowMessage = PowMessage::ChainResponse {
            //         target: msg.source().to_string(),
            //         source: self.swarm.local_peer_id().to_string(),
            //         chain: self.chain.clone(),
            //     };
            //     swarm::publish_pow_msg(resp.clone(), &mut self.swarm);
            //     responded!("\"{}\" to PeerId({})", resp, abbrev(msg.source()));
            // }
            // PowMessage::ChainResponse { chain, .. } => match self.chain.choose_chain(chain) {
            //     Ok(res) => update!("{}", res),
            //     Err(e) => update!("Remote chain couldn't be validated due to \"{}\"", e),
            // },
            PowMessage::BlockRequest { hash, .. } => {
                if let Some(block) = self.chain.find(&|b| b.hash == hash) {
                    let resp: PowMessage = PowMessage::BlockResponse {
                        target: msg.source().clone(),
                        source: self.swarm.local_peer_id().to_string(),
                        block: (*block).clone(),
                    };
                    swarm::publish_pow_msg(resp.clone(), &mut self.swarm);
                    responded!("\"{}\" to PeerId({}):", resp, abbrev(msg.source()));
                } else {
                    update!("Block not found on the main chain.");
                }
            }
            PowMessage::BlockResponse { block, .. } => {
                self.handle_block(block, Chain::store_orphan_block)
            }
            PowMessage::NewBlock { block, .. } => self.handle_block(block, Chain::store_new_block),
        }
    }

    fn handle_block<F>(&mut self, block: Block, store_block: F)
    where
        F: FnOnce(&mut Chain, Block) -> Result<NextBlockResult, NextBlockErr>,
    {
        if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data) {
            match Transaction::validate_transaction(&txn) {
                Ok(()) => {
                    update!("Processed transaction in block as valid.")
                }
                Err(e) => {
                    update!(
                        "Processed transaction in block as invalid due to\n\t\"{}\"",
                        e
                    );
                    return;
                }
            }
        }

        match store_block(&mut self.chain, block.clone()) {
            Ok(res) => {
                update!("Block resulted in update:\n\t\"{}\"", res);
                if remove_from_pool(&mut self.txns, &block) {
                    update!("Deleted mined transaction from the local pool.");
                }
                // Update the state of the main chain
                if let Ok(res) = self.chain.choose_fork() {
                    update!("{}", res);
                }
            }
            Err(e) => {
                update!(
                    "Block resulted in no update to chain or forks:\n\t\"{}\"",
                    e
                );
                if let NextBlockErr::MissingParent {
                    parent_hash,
                    parent_idx,
                } = e
                {
                    let req = PowMessage::BlockRequest {
                        target: None,
                        source: self.swarm.local_peer_id().to_string(),
                        idx: parent_idx,
                        hash: parent_hash.clone(),
                    };
                    swarm::publish_pow_msg(req.clone(), &mut self.swarm);
                    responded!("\"{}\" to all connected peers.", req);
                }
            }
        }
    }

    // Transaction event.
    fn handle_txn_event(&mut self, msg: TxnMessage) {
        received!("\"{}\" from PeerId({})", msg, abbrev(msg.source()));
        match msg {
            TxnMessage::NewTransaction { txn, .. } => {
                match Transaction::validate_transaction(&txn) {
                    Ok(()) => {
                        self.txns.insert(txn);
                        update!("Added new transaction to pool.");
                    }
                    Err(e) => {
                        update!("Processed transaction as invalid:\n\t\"{}\"", e);
                    }
                }
            }
        }
    }
    // Stdin event for a local user command.
    async fn handle_std_event(&mut self, cmd: &str) {
        match cmd {
            // `reset`, deletes the current local chain and writes a new one with a single block.
            cmd if cmd.starts_with("reset") => self.handle_cmd_reset(),
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
            cmd if cmd.starts_with("redial") => self.handle_cmd_redial(),
            cmd if cmd.starts_with("help") => {
                print_user_commands();
            }
            // //`req <all | [peer_id]>`, requiring us to publish a ChainRequest to the network.
            // cmd if cmd.starts_with("req") => {
            //     let arg = cmd.strip_prefix("req").expect("can strip `req`").trim();
            //     self.handle_cmd_req(arg)
            // }
            // `mine [data]` makes and writes a new block with the given data (and an incrementing id)
            cmd if cmd.starts_with("mine") => {
                let arg = cmd.strip_prefix("mine").expect("can strip `mine`").trim();
                self.handle_cmd_mine(arg)
            }
            // `show <chain | forks | orphans | peers | txns >` lists the main chain, forks, orphans, discovered & connected peers, or transaction pool
            cmd if cmd.starts_with("show") => {
                let arg = cmd.strip_prefix("show").expect("can strip `show`").trim();
                self.handle_cmd_show(arg);
            }
            // `txn [data]`, broadcasts a random transaction with the "amount" set to [data]
            cmd if cmd.starts_with("txn") => {
                let arg = cmd.strip_prefix("txn").expect("can strip `txn`").trim();
                self.handle_cmd_txn(arg);
            }
            _ => {
                println!(
                    "Unknown command: \"{}\" \nWrite `help` to show available commands.",
                    cmd
                );
            }
        }
    }
    fn handle_cmd_txn(&mut self, arg: &str) {
        if arg.is_empty() {
            println!("Command error: `req` missing an argument.\nUsage: req <all | [peer_id]>");
        } else {
            let txn: Transaction =
                Transaction::random_transaction(arg.to_string(), swarm::LOCAL_KEYS.clone());
            self.txns.insert(txn.clone());
            update!("Added a new transaction to pool:\n{}", txn);
            let txn_msg: TxnMessage = TxnMessage::NewTransaction {
                txn,
                source: self.swarm.local_peer_id().to_string(),
            };
            swarm::publish_txn_msg(txn_msg.clone(), &mut self.swarm);
            responded!("Broadcasted \"{}\" to all connected peers.", txn_msg);
        }
    }
    async fn handle_cmd_load(&mut self, file_name: &str) {
        let file_name = if file_name.is_empty() {
            DEFAULT_FILE_PATH
        } else {
            file_name
        };
        match file::read_chain(file_name).await {
            Ok(chain) => {
                self.chain = chain;
                update!("Loaded chain from local file \"{}\"", file_name)
            }
            Err(e) => eprintln!(
                "Error loading chain from local file:\n\
                                                \"{}\"",
                e
            ),
        }
    }
    async fn handle_cmd_save(&mut self, file_name: &str) {
        let file_name = if file_name.is_empty() {
            DEFAULT_FILE_PATH
        } else {
            file_name
        };
        match file::write_chain(&self.chain, file_name).await {
            Ok(()) => update!("Saved chain to local file \"{}\"", file_name),
            Err(e) => update!("Error saving chain to local file:\"{}\"", e),
        }
    }
    fn handle_cmd_reset(&mut self) {
        self.chain = chain::Chain::genesis();
        update!("Main chain reset to a single genesis block. Forks emptied.")
    }
    fn handle_cmd_mine(&mut self, args: &str) {
        let opt_data: Option<String> =
            // Retrieve data as the next transaction (as a string) from the pool
            if args.is_empty()  {
                extract_from_pool(&mut self.txns)
                .map(|txn|
                    {   // assuming we can always safely serialize a transaction (which should be the case)..
                        update!("Retrieved and transaction with hash {} from the pool.", txn.hash);
                        serde_json::to_string(&txn).unwrap()
                    }
                )
            }
            else {
                Some (args.to_string())
            };
        match opt_data {
            None => {
                update!("No transactions in the pool to mine for.")
            }
            Some(data) => {
                self.chain.mine_block(&data);
                update!(
                    "Mined and pushed a new block to main chain:\n{}",
                    self.chain.last()
                );
                let msg: PowMessage = PowMessage::NewBlock {
                    source: self.swarm.local_peer_id().to_string(),
                    block: self.chain.last().clone(),
                };
                swarm::publish_pow_msg(msg.clone(), &mut self.swarm);
                responded!("\"{}\" to all connected peers", msg);
            }
        }
    }
    // fn handle_cmd_req(&mut self, args: &str) {
    //     match args {
    //         _ if args.is_empty() => {
    //             println!("Command error: `req` missing an argument.\nUsage: <all | [peer_id]>");
    //         }
    //         "all" => {
    //             let req: PowMessage = PowMessage::ChainRequest {
    //                 target: None,
    //                 source: self.swarm.local_peer_id().to_string(),
    //             };
    //             responded!("\"{}\" to all connected peers.", req);
    //             swarm::publish_pow_msg(req, &mut self.swarm);
    //         }
    //         target => {
    //             let req = PowMessage::ChainRequest {
    //                 target: Some(target.to_string()),
    //                 source: self.swarm.local_peer_id().to_string(),
    //             };
    //             responded!("\"{}\" to PeerId({}).", req, abbrev(target));
    //             swarm::publish_pow_msg(req, &mut self.swarm);
    //         }
    //     }
    // }
    fn handle_cmd_show(&mut self, args: &str) {
        match args {
            _ if args.is_empty() => {
                println!("Command error: `show` missing an argument.\nUsage: show <chain | forks | peers | txns>")
            }
            "chain" => {
                println!(
                    "Current chain:\n\
                            {}",
                    self.chain
                );
            }
            "forks" => {
                println!("Current forks:\n");
                self.chain.print_forks();
            }
            "orphans" => {
                println!("Current orphans:\n");
                self.chain.print_orphans();
            }
            "peers" => {
                let (dscv_peers, conn_peers): (Vec<PeerId>, Vec<PeerId>) =
                    swarm::get_peers(&mut self.swarm);
                println!("Discovered Peers ({})", dscv_peers.len());
                dscv_peers.iter().for_each(|p| println!("{}", p));
                println!("Connected Peers ({})", conn_peers.len());
                conn_peers.iter().for_each(|p| println!("{}", p));
            }
            "pool" => {
                println!("Current transaction pool:\n");
                self.txns.iter().for_each(|txn| println!("{}", txn))
            }
            _ => {
                println!("Command error: `show` has unrecognised argument(s).\nUsage: show <chain | forks | peers | txns>")
            }
        }
    }
    fn handle_cmd_redial(&mut self) {
        let discovered_peers: Vec<libp2p::PeerId> = swarm::discovered_peers(&mut self.swarm);
        if discovered_peers.is_empty() {
            println!("No discovered peers to dial!");
            return;
        }
        for peer_id in discovered_peers {
            match self.swarm.dial(&peer_id) {
                Ok(()) => println!("Dial for {}", peer_id),
                Err(e) => eprintln!("Dial error {}", e),
            }
        }
    }
    // (Predefined) Swarm event. For debugging purposes.
    fn handle_swarm_event<E: std::fmt::Debug>(swarm_event: SwarmEvent<(), E>) {
        match swarm_event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => update!(
                "Connection established with PeerId({})",
                abbrev(&peer_id.to_string())
            ),
            SwarmEvent::ConnectionClosed { peer_id, .. } => update!(
                "Connection closed with PeerId({})",
                abbrev(&peer_id.to_string())
            ),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
                ..
            } => info!("SwarmEvent: {:?} listening on {}", listener_id, address),
            SwarmEvent::Dialing(peer_id) => info!(
                "SwarmEvent: dialling PeerId({})",
                abbrev(&peer_id.to_string())
            ),
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => info!(
                "SwarmEvent: incoming connection on addr {:?} with send-back addr {}",
                local_addr, send_back_addr
            ),
            _ => info!("Unhandled swarm event: {:?}", swarm_event),
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
    let swarm = swarm::set_up_blockchain_swarm(pow_sender, txn_sender).await;

    // Async Reader for StdIn, which reads the stream line by line.
    let from_stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    // Load chain from local file
    let chain: Chain = match file::read_chain(DEFAULT_FILE_PATH).await {
        Err(e) => {
            eprintln!(
                "\nProblem loading chain from the default file: \"{}\" \n\
                           Instantiating a fresh chain instead. ",
                e
            );
            Chain::genesis()
        }
        Ok(chain) => {
            println!(
                "\nLoaded chain from default file \"{}\".",
                DEFAULT_FILE_PATH
            );
            chain
        }
    };

    println!("\n## Your Peer Id ##\n{}", swarm.local_peer_id());
    Peer {
        from_stdin,
        pow_receiver,
        txn_receiver,
        swarm,
        chain,
        txns: HashSet::new(),
    }
}

fn remove_from_pool(txns: &mut HashSet<Transaction>, block: &Block) -> bool {
    if let Ok(txn) = serde_json::from_str::<Transaction>(&block.data) {
        return txns.remove(&txn);
    }
    false
}
fn extract_from_pool(txns: &mut HashSet<Transaction>) -> Option<Transaction> {
    if let Some(txn) = txns.iter().next() {
        // txns.remove(&txn); //  doesn't work: we immutably borrowing txns, via &txn, while mutably borrowing it, via txns.remove(..)
        let txn_to_return = txn.clone(); // clone the value, so that we stop immutably borrowing txns
        txns.remove(&txn_to_return);
        Some(txn_to_return)
    } else {
        None // If the pool is empty
    }
}

fn print_user_commands() {
    let commands = include_str!("../commands.md");
    println!("{}", commands);
}
