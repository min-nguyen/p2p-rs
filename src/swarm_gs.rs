// https://docs.rs/gossipsub/latest/gossipsub/
// https://github.com/libp2p/rust-libp2p/tree/master/examples

use libp2p::{futures::future::Either, gossipsub::{self, GossipsubEvent, IdentTopic, MessageAuthenticity, Topic}, identity, mdns::{self, Mdns, MdnsEvent}, noise::X25519Spec, swarm::{NetworkBehaviourEventProcess, SwarmBuilder}, NetworkBehaviour, PeerId};
use libp2p::core::{identity::Keypair,transport::{Transport, MemoryTransport}, Multiaddr};
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;


// FloodSub Topic for subscribing and sending blocks
pub static BLOCK_TOPIC: Lazy<IdentTopic> = Lazy::new(|| Topic::new("blocks"));

// Messages are either (1) requests for data or (2) responses to some arbitrary peer's request.
pub type BlockchainMessage = Either<BlockRequest, BlockResponse>;
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockRequest {
    // Requests for blocks can be either ToAll or ToOne
    pub transmit_type : TransmitType,
    // The PeerID the request came from.
    pub sender_peer_id : String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockResponse {
    // Responding with our blocks will always to be ToAll
    pub transmit_type : TransmitType,
    // The PeerID to recieve the response.
    pub receiver_peer_id : String,
    // Core message payload being transmitted in the network.
    pub data : String
}
// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String)   // contains intended peer id
}

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct BlockchainBehaviour {
  pub gossipsub: gossipsub::Gossipsub,
  pub mdns: Mdns
}

impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    info!("discovered peer: {}", peer);
                    // Add to our list of peers to propagate messages to
                    // self.gossipsub.ad .add_node_to_partial_view(peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("removed peer: {}", peer);
                    if !self.mdns.has_node(&peer) {
                        // self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
            //
            // MdnsEvent
        }
    }
}
impl NetworkBehaviourEventProcess<GossipsubEvent> for BlockchainBehaviour {
  fn inject_event(&mut self, event: GossipsubEvent) {
      match event {
          // Event for a new message from a peer
          gossipsub::GossipsubEvent::Message{ propagation_source, message_id, message } => {
            println!("{:?}", message)
          }
          _ => (),
      }
  }
}

// pub fn set_up_block_behaviour(){
//   let message_id_fn = |message: &gossipsub::Message| {
//       let mut s = DefaultHasher::new();
//       message.data.hash(&mut s);
//       gossipsub::MessageId::from(s.finish().to_string())
//   };

//   // Set a custom gossipsub configuration
//   let gossipsub_config = gossipsub::ConfigBuilder::default()
//       .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
//       .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
//       .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
//       .build()
//       .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

//   // build a gossipsub network behaviour
//   let gossipsub = gossipsub::Behaviour::new(
//       gossipsub::MessageAuthenticity::Signed(key.clone()),
//       gossipsub_config,
//   )?;

//   let mdns =
//       mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
//   Ok(MyBehaviour { gossipsub, mdns })
// }

pub async fn set_up_swarm(local_peer_id : PeerId, local_keys : identity::Keypair, to_local_peer : mpsc::UnboundedSender<BlockchainMessage>){

  // Transport,
  let transp = {
    // Authentication keys, for the `Noise` crypto-protocol, used to secure traffic within the p2p network
    let local_auth_keys: libp2p::noise::AuthenticKeypair<X25519Spec>
      = libp2p::noise::Keypair::<X25519Spec>::new()
      .into_authentic(&local_keys)
      .expect("can create auth keys");

    MemoryTransport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(libp2p::noise::NoiseConfig::xx(local_auth_keys).into_authenticated())
            .multiplex(libp2p::mplex::MplexConfig::new())
            .boxed()
    };

  // Network behaviour
  let mut behaviour = {
    let gossipsub: libp2p::gossipsub::Gossipsub =
        libp2p::gossipsub::Gossipsub::new
          ( MessageAuthenticity::Signed(local_keys)
          , libp2p::gossipsub::GossipsubConfig::default()).unwrap();

    let mdns =  Mdns::new(Default::default())
          .await
          .expect("can create mdns");

    BlockchainBehaviour {gossipsub, mdns}
  };
  behaviour.gossipsub.subscribe(&BLOCK_TOPIC);

  // Create a Swarm to manage peers and events
  let mut swarm =
    SwarmBuilder::new(transp, behaviour, local_peer_id)
    .executor(Box::new(|fut| {
        tokio::spawn(fut);
    }))
    .build();


  // Listen on a memory transport.
  let memory: Multiaddr = libp2p::core::multiaddr::Protocol::Memory(10).into();
  let addr = libp2p::swarm::Swarm::listen_on(&mut swarm, memory).unwrap();
  println!("Listening on {:?}", addr);
}