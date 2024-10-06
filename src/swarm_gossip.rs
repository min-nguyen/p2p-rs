/*  A Swarm for NetworkBehaviour with a GossipSub Messaging Protocol.
    https://docs.rs/gossipsub/latest/gossipsub/

    GossipSub, unlike FloodSub, can have its max transmit message size be changed.
*/

use libp2p::{
    gossipsub::{self, Gossipsub, GossipsubConfig, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, IdentTopic, MessageAuthenticity, MessageId, Topic, ValidationMode},
    mplex, noise,
    Multiaddr, NetworkBehaviour, PeerId, Transport,
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    identity::Keypair,
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig
};

use once_cell::sync::Lazy;
use std::{collections::HashSet, hash::{DefaultHasher, Hash, Hasher}};
use tokio::sync::mpsc;
use log::{debug, error, info};
use std::time::Duration;

use crate::transaction::{self, Transaction};

use super::message::{PowMessage, TxnMessage, TransmitType};

static LOCAL_KEYS: Lazy<Keypair> = Lazy::new(|| Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

static CHAIN_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("chain"));
static TRANSACTION_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("transactions"));

const MAX_MESSAGE_SIZE : usize = 10 * 1_048_576;     // 10mb

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
  pub gossipsub: gossipsub::Gossipsub,
  pub mdns: Mdns,
  // ** Relevant only to a specific local peer that we are setting up
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
                    info!("MdnsEvent: discovered peer: {}", peer);
                    self.gossipsub.add_explicit_peer(&peer);
                }
                // let mesh_peers : Vec<libp2p::PeerId> = self.gossipsub.all_mesh_peers().cloned().collect();
                // println!("{:?}", mesh_peers);
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
        match event {
            GossipsubEvent::Message{propagation_source, message, message_id} => {
                    if let Ok(pow_msg) = serde_json::from_slice::<PowMessage>(&message.data) {
                    info!("Received {:?} from {:?}", pow_msg, propagation_source);
                    match pow_msg {
                            PowMessage::ChainResponse { ref transmit_type, .. }
                            | PowMessage::ChainRequest { ref transmit_type, .. }
                            | PowMessage::NewBlock { ref transmit_type, .. } =>
                            match transmit_type {
                                TransmitType::ToOne(target_peer_id) if *target_peer_id == LOCAL_PEER_ID.to_string()
                                => if let Err(e) = self.pow_sender.send(pow_msg){
                                        error!("Error sending message to peer via local channel: {}", e);
                                    }
                                ,
                                TransmitType::ToAll
                                => if let Err(e) = self.pow_sender.send(pow_msg){
                                        error!("Error sending message to peer via local channel: {}", e);
                                    }
                                ,
                                _ => info!("Ignoring received message -- not for us.")
                            }
                        }
                    }
                    else if let Ok(transaction) = serde_json::from_slice::<TxnMessage>(&message.data) {
                        self.txn_sender.send(transaction);
                    }
            }
            _ => (),
        }
    }
}

fn new_tcp_transport() -> Boxed<(PeerId, StreamMuxerBox)>{
    // Authentication keys, for the `Noise` crypto-protocol, used to secure traffic within the p2p network
    let local_auth_keys: noise::AuthenticKeypair<noise::X25519Spec>
        = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&LOCAL_KEYS.clone())
        .expect("can create auth keys");

    TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
}

async fn new_mdns_discovery() -> Mdns {
    let mdns_config: MdnsConfig
      = MdnsConfig::default();
    Mdns::new(mdns_config)
        .await
        .expect("can create mdns")
}

pub async fn set_up_blockchain_swarm(
        pow_sender : mpsc::UnboundedSender<PowMessage>
    ,   txn_sender : mpsc::UnboundedSender<TxnMessage>)
    -> Swarm<BlockchainBehaviour> {

    // Transport
    let transp = new_tcp_transport();

    // Network behaviour
    let mut behaviour: BlockchainBehaviour = {

        // Discovery Protocol
        let mdns = new_mdns_discovery().await;

        // Communication Protocol
        let gossipsub_config: GossipsubConfig
            = GossipsubConfigBuilder::default()
                // Custom hashing for message_ids, to filter out duplicate transactions
                .message_id_fn(message_id_fn)
                // This is set to aid debugging by not cluttering the log space
                .heartbeat_interval(Duration::from_secs(10))
                // This sets the kind of message validation. The default is Strict (enforce message signing)
                // By default, the gossipsub implementation will sign all messages with the author’s private key, and require a valid signature before accepting or propagating a message further.
                .validation_mode(ValidationMode::Strict)
                // increase max size of messages published size
                .max_transmit_size(MAX_MESSAGE_SIZE)
                // time a connection is maintained to a peer without being in the mesh and without receiving/sending a message to them
                .idle_timeout(Duration::from_secs(120))
                // number of heartbeats to keep in cache
                .history_length(12)
                .max_messages_per_rpc(Some(500))
                .build()
                .expect("valid gossipsub config");

        let gossipsub: Gossipsub
        = Gossipsub::new
            ( MessageAuthenticity::Signed(LOCAL_KEYS.clone())
            , gossipsub_config,
            )
            .expect("can create gossipsub");

        BlockchainBehaviour {mdns, gossipsub, pow_sender, txn_sender}
    };

    match behaviour.gossipsub.subscribe(&CHAIN_TOPIC)  {
        Ok(b) => info!("gossipsub.subscribe() returned {}", b),
        Err(e) => eprintln!("gossipsub.subscribe() error: {:?}", e)
    };

    // Swarm
    let mut swarm =
            SwarmBuilder::new(transp, behaviour, LOCAL_PEER_ID.clone())
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

    // Listen on a memory transport.
    let listen_addr : Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket");
    Swarm::listen_on(&mut swarm, listen_addr.clone()).expect("swarm can be started");
    println!("Listening on {:?}", listen_addr);
    swarm
}

pub fn publish_message(msg: PowMessage, swarm: &mut Swarm<BlockchainBehaviour>){
    let json = serde_json::to_string(&msg).expect("can jsonify message");
    let res = swarm
            .behaviour_mut()
            .gossipsub
            .publish(CHAIN_TOPIC.clone(), json.as_bytes());
    match res {
        Err(e)   => eprintln!("publish_message() error: {:?}", e),
        Ok (msg_id) => info!("publish_message() successful msg_id = {}", msg_id)
    }
}

pub fn get_peers(swarm: &mut Swarm<BlockchainBehaviour> ) -> (Vec<PeerId>, Vec<PeerId>) {
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
    let collect_peers
        = |peers : HashSet<&PeerId>| peers.into_iter().cloned().collect();

    (collect_peers(discovered_peers), collect_peers(connected_peers))
}

// Configuring with this message_id_fn would prevent two messages of simply the same content being sent.
fn message_id_fn(message: &gossipsub::GossipsubMessage) -> MessageId {
    let mut hasher: DefaultHasher = DefaultHasher::new();
    let GossipsubMessage{  data, topic, ..} = message;

    // Filter out duplicate transactions by hashing only on the payload (i.e. the transaction).
    if *topic == TRANSACTION_TOPIC.hash(){
      data.hash(&mut hasher);
    }
    // Allow duplicates of other messages (e.g. several requests).
    else {
      message.hash(&mut hasher);
    }
    gossipsub::MessageId::from(hasher.finish().to_string())
}