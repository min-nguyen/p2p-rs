/*  A Swarm for NetworkBehaviour with a FloodSub Messaging Protocol

    *Swarm*:
    Wraps around a specific NetworkBehaviour and drives the execution of the network's defined behaviours.
    -- Used to broadcast messages to other peers' NetworkBehaviours.
    -- Manages connections created with the Transport and executes our NetworkBehaviour
    -- Used to trigger and receive events from the network

    Each peer has a local Swarm object.
*/

use libp2p::{
  floodsub::{Floodsub, FloodsubConfig, FloodsubEvent, Topic},
  mplex, noise, core::upgrade, futures::future::Either, identity::Keypair, mdns::{Mdns, MdnsEvent}, swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder}, tcp::TokioTcpConfig, Multiaddr, NetworkBehaviour, PeerId, Transport
  };

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;
use log::{debug, error, info};

use super::block;


/*  (Key Pair, Peer ID) are libp2p's intrinsics for identifying a client on the network.
    Below initialises these as global values that identify the current application (i.e. client) running.

    (1) Key Pair: Public & private key for secure communication with the rest of the network
    (2) PeerId: Unique hash of public key, used to identify the peer within the whole p2p network.
*/
static LOCAL_KEYS: Lazy<Keypair> = Lazy::new(|| Keypair::generate_ed25519());
static LOCAL_PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(LOCAL_KEYS.public()));

// FloodSub Topic for subscribing and sending blocks
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

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
    pub data : block::Block
}
// Messages can be intended for (1) all peers or (2) a specific peer.
#[derive(Debug, Serialize, Deserialize)]
pub enum TransmitType {
    ToAll,
    ToOne(String)   // contains intended peer id
}

// Base NetworkBehaviour, simply specifying the 2 Protocol types
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    // ** Relevant to the global P2P Network Behaviour that all peers must share:
    pub floodsub: Floodsub,
    pub mdns: Mdns,

    // ** Relevant only to a specific local peer that we are setting up
    #[behaviour(ignore)]
    to_local_peer: mpsc::UnboundedSender<BlockchainMessage>,
    #[behaviour(ignore)]
    local_peer_id: PeerId
}

/*
Defining the Sub-Behaviours for handling events, `inject_event()`, from each Protocol Type.
    1. Sub-Behaviour for the mDNS Discovery Protocol.
    2. Concrete Sub-Behaviour for the FloodSub Communication Protocol.
*/
impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    info!("discovered peer: {}", peer);
                    // Add to our list of peers to propagate messages to
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("removed peer: {}", peer);
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
impl NetworkBehaviourEventProcess<FloodsubEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            // Event for a new message from a peer
            FloodsubEvent::Message(msg) => {
                // Match on the deserialized payload as a BlockResponse, which we print to console.
                if let Ok(resp) = serde_json::from_slice::<BlockResponse>(&msg.data) {
                    if resp.receiver_peer_id == self.local_peer_id.to_string() {
                        info!("response from {}:", msg.source);
                        if let Err(e) = self.to_local_peer.send(Either::Right(resp)){
                            error!("error sending request via channel, {}", e);
                        }
                    }
                }
                // Match on the deserialized payload as a BlockRequest, which we may forward to our local peer
                else if let Ok(req) = serde_json::from_slice::<BlockRequest>(&msg.data) {
                    match req.transmit_type {
                        // Forward any ToAll requests to local peer
                        TransmitType::ToAll => {
                            info!("received req {:?} from {:?}", req, msg.source);
                            info!("forwarding req from {:?}", msg.source);
                            if let Err(e) = self.to_local_peer.send(Either::Left(req)) {
                                error!("error sending response via channel, {}", e);
                            };
                        }
                        // Filter any ToOne requests if not intended for us, otherwise forwarding to local peer
                        TransmitType::ToOne(ref peer_id) => {
                            info!("received req {:?} from {:?}", req, msg.source);
                            if peer_id == &self.local_peer_id.to_string() {
                                info!("forwarding req from {:?}", msg.source);
                                if let Err(e) = self.to_local_peer.send(Either::Left(req)) {
                                    error!("error sending response via channel, {}", e);
                                };
                            }
                        }
                    }
                }
                else {println!("{:?}", msg);}
            }
            _ => (),
        }
    }
}

/*  Create a swarm with our Transport, NetworkBehaviour, and PeerID.
    Start to listen to a local IP (port decided by the OS) using our set up. */
pub async fn set_up_swarm(to_local_peer : mpsc::UnboundedSender<BlockchainMessage>)
  -> Swarm<BlockchainBehaviour> {

  // Transport, which we multiplex to enable multiple streams of data over one communication link.
  let transp = {
      // Authentication keys, for the `Noise` crypto-protocol, used to secure traffic within the p2p network
      let local_auth_keys: noise::AuthenticKeypair<noise::X25519Spec>
          = noise::Keypair::<noise::X25519Spec>::new()
          .into_authentic(&LOCAL_KEYS)
          .expect("can create auth keys");

      TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(local_auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
    };

  // Network Behaviour, subscribed to block topic
  let mut behaviour =  {

      let floodsubconfig: FloodsubConfig
        = FloodsubConfig::new(LOCAL_PEER_ID.clone());
      let floodsub: Floodsub
        = Floodsub::from_config(floodsubconfig);

      let mdns
        = Mdns::new(Default::default())
            .await
            .expect("can create mdns");

      BlockchainBehaviour {floodsub, mdns, to_local_peer, local_peer_id : LOCAL_PEER_ID.clone()}
  };
  behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

  // Create a swarm with our Transport, NetworkBehaviour, and PeerID.
  let mut swarm
    =  SwarmBuilder::new(transp, behaviour,  LOCAL_PEER_ID.clone())
      .executor(Box::new(|fut| {
          tokio::spawn(fut);
      }))
      .build();

  let listen_addr : Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket");
  Swarm::listen_on(&mut swarm, listen_addr).expect("swarm can be started");
  swarm
}


pub async fn publish_response(resp: BlockResponse, swarm: &mut Swarm<BlockchainBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await;
  info!("publish_response() successful")
}
pub async fn publish_request(resp: BlockRequest, swarm: &mut Swarm<BlockchainBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await;
  info!("publish_request() successful")
}
async fn publish(json : String,  swarm: &mut Swarm<BlockchainBehaviour> ) {
  swarm
      .behaviour_mut()
      .floodsub
      .publish(BLOCK_TOPIC.clone(), json.as_bytes());
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
