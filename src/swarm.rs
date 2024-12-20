/*
    *Swarm*: Contains the network logic using GossipSub as the communication protocol and Mdns as the peer discovery protocol.
    (GossipSub, unlike FloodSub, can have its max transmit message size be changed.)
    - Configures PeerId, Keypair, and Topic(s) for the network.
    - Sets up NetworkBehaviour (that defines how peer discovery and message events are handled).
    - Sets up Swarm (that executes the NetworkBehaviour).
*/

use super::message::{PowMessage, TxnMessage};

use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    gossipsub::{
        self, Gossipsub, GossipsubConfig, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage,
        IdentTopic, MessageAuthenticity, MessageId, Topic, ValidationMode,
    },
    identity::Keypair,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    mplex, noise,
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Multiaddr, NetworkBehaviour, PeerId, Transport,
};
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::time::Duration;
use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
};
use tokio::sync::mpsc::{self, UnboundedSender};

pub static LOCAL_KEYS: Lazy<Keypair> = Lazy::new(|| Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

static CHAIN_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("chain"));
static TXN_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("transactions"));

const MAX_MESSAGE_SIZE: usize = 10 * 1_048_576; // 10mb

// Custom network behaviour that combines Gossipsub and Mdns
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    pub gossipsub: gossipsub::Gossipsub,
    pub mdns: Mdns,

    // ** relevant only to a specific local peer that we are setting up
    #[behaviour(ignore)]
    pow_sender: mpsc::UnboundedSender<PowMessage>,
    #[behaviour(ignore)]
    txn_sender: mpsc::UnboundedSender<TxnMessage>,
}

impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    info!("MdnsEvent: discovered new peer: {}", peer);
                    self.gossipsub.add_explicit_peer(&peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    info!("MdnsEvent: removed peer: {}", peer);
                    if !self.mdns.has_node(&peer) {
                        self.gossipsub.remove_explicit_peer(&peer);
                    }
                }
            }
        }
    }
}
impl NetworkBehaviourEventProcess<GossipsubEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message {
            propagation_source,
            message,
            ..
        } = event
        {
            info!("Received {:?} from {:?}", message, propagation_source);
            if let Ok(pow_msg) = serde_json::from_slice::<PowMessage>(&message.data) {
                match pow_msg {
                    PowMessage::ChainRequest { ref target, .. }
                    | PowMessage::BlockRequest { ref target, .. } => match target {
                        Some(target) if *target == LOCAL_PEER_ID.to_string() => {
                            send_local_peer(&self.pow_sender, pow_msg)
                        }
                        None => send_local_peer(&self.pow_sender, pow_msg),
                        _ => info!("Ignoring request. Not for us."),
                    },
                    PowMessage::ChainResponse { ref target, .. }
                    | PowMessage::BlockResponse { ref target, .. } => {
                        if *target == LOCAL_PEER_ID.to_string() {
                            send_local_peer(&self.pow_sender, pow_msg)
                        } else {
                            info!("Ignoring response. Not for us.")
                        }
                    }
                    PowMessage::NewBlock { .. } => send_local_peer(&self.pow_sender, pow_msg),
                }
            } else if let Ok(txn_msg) = serde_json::from_slice::<TxnMessage>(&message.data) {
                send_local_peer(&self.txn_sender, txn_msg)
            }
        }
    }
}

fn new_tcp_transport() -> Boxed<(PeerId, StreamMuxerBox)> {
    // Authentication keys, for the `Noise` crypto-protocol, used to secure traffic within the p2p network
    let local_auth_keys: noise::AuthenticKeypair<noise::X25519Spec> =
        noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&LOCAL_KEYS.clone())
            .expect("can create auth keys");

    TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
}

async fn new_mdns_discovery() -> Mdns {
    let mdns_config: MdnsConfig = MdnsConfig::default();
    Mdns::new(mdns_config).await.expect("can create mdns")
}

pub async fn set_up_blockchain_swarm(
    pow_sender: UnboundedSender<PowMessage>,
    txn_sender: UnboundedSender<TxnMessage>,
) -> Swarm<BlockchainBehaviour> {
    // Transport
    let transp = new_tcp_transport();

    // Network behaviour
    let mut behaviour: BlockchainBehaviour = {
        // Discovery Protocol
        let mdns = new_mdns_discovery().await;

        // Communication Protocol
        let gossipsub_config: GossipsubConfig = GossipsubConfigBuilder::default()
            // custom hashing for message_ids, to filter out duplicate transactions
            .message_id_fn(filter_dup_transactions)
            // aid debugging by not cluttering the log space
            .heartbeat_interval(Duration::from_secs(10))
            // by default, the gossipsub implementation will sign all messages with the author’s private key, and require a valid signature before accepting or propagating a message further.
            .validation_mode(ValidationMode::Strict)
            // increase max size of messages published size
            .max_transmit_size(MAX_MESSAGE_SIZE)
            // time a connection is maintained to a peer without being in the mesh and without receiving/sending a message to them
            .idle_timeout(Duration::from_secs(600))
            // number of heartbeats to keep in cache
            .history_length(12)
            .max_messages_per_rpc(Some(500))
            .build()
            .expect("valid gossipsub config");

        let gossipsub: Gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(LOCAL_KEYS.clone()),
            gossipsub_config,
        )
        .expect("can create gossipsub");

        BlockchainBehaviour {
            mdns,
            gossipsub,
            pow_sender,
            txn_sender,
        }
    };
    match behaviour.gossipsub.subscribe(&CHAIN_TOPIC) {
        Ok(b) => info!("gossipsub.subscribe() returned {}", b),
        Err(e) => warn!("gossipsub.subscribe() error: {:?}", e),
    };
    match behaviour.gossipsub.subscribe(&TXN_TOPIC) {
        Ok(b) => info!("gossipsub.subscribe() returned {}", b),
        Err(e) => warn!("gossipsub.subscribe() error: {:?}", e),
    };

    // Swarm
    let mut swarm = SwarmBuilder::new(transp, behaviour, *LOCAL_PEER_ID)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    // Listen on a memory transport.
    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .expect("can get a local socket");
    Swarm::listen_on(&mut swarm, listen_addr.clone()).expect("swarm can be started");
    println!("Listening on {:?}", listen_addr);
    swarm
}

fn filter_dup_transactions(message: &gossipsub::GossipsubMessage) -> MessageId {
    let mut hasher: DefaultHasher = DefaultHasher::new();
    let GossipsubMessage { data, topic, .. } = message;

    // filter out duplicate transactions by hashing only on the payload (i.e. the transaction).
    if *topic == TXN_TOPIC.hash() {
        data.hash(&mut hasher);
    }
    // allow duplicates of other message payloads (e.g. several requests).
    else {
        message.hash(&mut hasher);
    }
    gossipsub::MessageId::from(hasher.finish().to_string())
}

fn send_local_peer<T>(sender: &UnboundedSender<T>, msg: T) {
    if let Err(e) = sender.send(msg) {
        error!("Error sending message to peer via local channel: {}", e);
    }
}

pub fn publish_pow_msg(msg: PowMessage, swarm: &mut Swarm<BlockchainBehaviour>) {
    publish_msg(msg, CHAIN_TOPIC.clone(), swarm)
}

pub fn publish_txn_msg(msg: TxnMessage, swarm: &mut Swarm<BlockchainBehaviour>) {
    publish_msg(msg, TXN_TOPIC.clone(), swarm)
}

fn publish_msg<T: Serialize>(msg: T, topic: IdentTopic, swarm: &mut Swarm<BlockchainBehaviour>) {
    let s: String = match serde_json::to_string(&msg) {
        Ok(json) => json,
        Err(e) => {
            error!("Couldn't jsonify message, {}", e);
            return;
        }
    };
    let res = swarm.behaviour_mut().gossipsub.publish(topic, s.as_bytes());
    match res {
        Err(e) => info!("Publish message error: {:?}", e),
        Ok(_) => info!("Publish message successful."),
    }
}

pub fn get_peers(swarm: &mut Swarm<BlockchainBehaviour>) -> (Vec<PeerId>, Vec<PeerId>) {
    debug!("get_peers()");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut discovered_peers: HashSet<&PeerId> = HashSet::new();
    let mut connected_peers: HashSet<&PeerId> = HashSet::new();
    for peer in nodes {
        discovered_peers.insert(peer);
        if swarm.is_connected(peer) {
            connected_peers.insert(peer);
        }
    }
    let collect_peers = |peers: HashSet<&PeerId>| peers.into_iter().cloned().collect();

    (
        collect_peers(discovered_peers),
        collect_peers(connected_peers),
    )
}

pub fn connected_peers(swarm: &mut Swarm<BlockchainBehaviour>) -> Vec<PeerId> {
    get_peers(swarm).1
}
pub fn discovered_peers(swarm: &mut Swarm<BlockchainBehaviour>) -> Vec<PeerId> {
    get_peers(swarm).0
}
