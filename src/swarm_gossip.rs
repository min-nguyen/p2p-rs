/*  A Swarm for NetworkBehaviour with a GossipSub Messaging Protocol.
    https://docs.rs/gossipsub/latest/gossipsub/

    GossipSub, unlike FloodSub, can have its max transmit message size be changed.
*/

use libp2p::{
  gossipsub::{self, Gossipsub, GossipsubConfigBuilder, GossipsubEvent, IdentTopic, MessageAuthenticity, Topic, ValidationMode},
  mplex, noise, core::upgrade, identity::Keypair, mdns::{Mdns, MdnsEvent}, swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder}, tcp::TokioTcpConfig, Multiaddr, NetworkBehaviour, PeerId, Transport
};

use once_cell::sync::Lazy;
use std::collections::HashSet;
use tokio::sync::mpsc;
use log::{debug, error, info};
use std::time::Duration;

use super::message::{BlockMessage, TransmitType};

static LOCAL_KEYS: Lazy<Keypair> = Lazy::new(|| Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));
static BLOCK_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("blocks"));

const MAX_MESSAGE_SIZE : usize = 10 * 1_048_576;     // 10mb

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
  pub gossipsub: gossipsub::Gossipsub,
  pub mdns: Mdns,
  // ** Relevant only to a specific local peer that we are setting up
  #[behaviour(ignore)]
  to_local_peer: mpsc::UnboundedSender<BlockMessage>,
}

impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    info!("discovered peer: {}", peer);
                    //  self.gossipsub.all_mesh_peers().for_each(| x |  {println!("{:?}", x)} );
                    // Add to our list of peers to propagate messages to
                    // Swarm::dial(&mut self, peer_id)
                    // self.inject_event(event);
                    self.gossipsub.add_explicit_peer(&peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("removed peer: {}", peer);
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
          GossipsubEvent::Message{
            propagation_source,
            message_id,
            message } => {

                if let Ok(block_msg) = serde_json::from_slice::<BlockMessage>(&message.data) {
                  info!("received {:?} from {:?}", block_msg, propagation_source);
                  match block_msg {
                         BlockMessage::BlockResponse { ref transmit_type, .. }
                       | BlockMessage::BlockRequest { ref transmit_type, .. } =>
                          match transmit_type {
                              TransmitType::ToOne(target_peer_id) if *target_peer_id == LOCAL_PEER_ID.to_string()
                              => if let Err(e) = self.to_local_peer.send(block_msg){
                                      error!("error sending request via channel, {}", e);
                                 }
                              ,
                              TransmitType::ToAll
                              => if let Err(e) = self.to_local_peer.send(block_msg){
                                      error!("error sending request via channel, {}", e);
                                  }
                              ,
                              _ => info!("message unintended for us. ignoring.")
                          }
                  }
              }
          }
          _ => (),
      }
  }
}

pub async fn set_up_swarm(to_local_peer : mpsc::UnboundedSender<BlockMessage>)
  -> Swarm<BlockchainBehaviour> {

  // Transport
  let transp = {
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
  };

  // Network behaviour
  let mut behaviour = {

    // GossipSub
    /*  Configuring with this message_id_fn would prevent two messages of simply the same content being sent.
        This is not useful for us if we want to send `req all` twice.
        let message_id_fn = |message: &gossipsub::GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };  */

    let gossipsub_config
      = GossipsubConfigBuilder::default()
        // This is set to aid debugging by not cluttering the log space
        .heartbeat_interval(Duration::from_secs(10))
        // This sets the kind of message validation. The default is Strict (enforce message signing)
        .validation_mode(ValidationMode::Strict)
        // increase max size of messages published size
        .max_transmit_size(MAX_MESSAGE_SIZE)
        // time a connection is maintained to a peer without receiving/sending a message to them
        .idle_timeout(Duration::from_secs(120))
        // number of
        .history_length(12)
        .max_messages_per_rpc(Some(500))
        .build()
        .expect("valid gossipsub config");

    let gossipsub
      = Gossipsub::new
          ( MessageAuthenticity::Signed(LOCAL_KEYS.clone())
          , gossipsub_config,
        )
        .expect("can create gossipsub");

    // MDNS Discovery
    let mdns
      = Mdns::new(Default::default())
        .await
        .expect("can create mdns");

    BlockchainBehaviour {gossipsub, mdns, to_local_peer}
  };

  match behaviour.gossipsub.subscribe(&BLOCK_TOPIC)  {
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

pub async fn publish_block_message(resp: BlockMessage, swarm: &mut Swarm<BlockchainBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  if let Err(e) = publish(json, swarm).await {
    eprintln!("publish_block_message() error: {:?}", e)
  }
  else {
    info!("publish_block_message() successful")
  }
}
async fn publish(json : String,  swarm: &mut Swarm<BlockchainBehaviour> ) -> Result<gossipsub::MessageId, gossipsub::error::PublishError>{
  swarm
      .behaviour_mut()
      .gossipsub
      .publish(BLOCK_TOPIC.clone(), json.as_bytes())
}
pub fn get_peers(swarm: &mut Swarm<BlockchainBehaviour> ) -> (Vec<String>, Vec<String>) {
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

  let peers_to_strs
     = |peer_id : HashSet<&PeerId>| peer_id.iter().map(|p: &&PeerId| p.to_string()).collect();

  (peers_to_strs(discovered_peers), peers_to_strs(connected_peers))
}