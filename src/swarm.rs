/*
    *Swarm*:  Wraps around a specific NetworkBehaviour and drives the execution of the network's defined behaviours.
    -- Used to broadcast messages to other peers' NetworkBehaviours.
    -- Each peer has a local Swarm object.

    More generally,
    -- Manages connections created with the Transport and executes our NetworkBehaviour
    -- Used to trigger and receive events from the network
*/

use std::collections::HashSet;

use libp2p::{core::transport::Boxed, swarm::SwarmBuilder, PeerId, Swarm};
use log::{debug, info};

use super::network::{BLOCK_TOPIC, BlockchainBehaviour, BlockResponse, BlockRequest};

/*  Create a swarm with our Transport, NetworkBehaviour, and PeerID.
    Start to listen to a local IP (port decided by the OS) using our set up. */
pub fn set_up_swarm(transp : Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>, behaviour : BlockchainBehaviour, local_peer_id : PeerId)
  -> Swarm<BlockchainBehaviour> {

  let mut swarm
  =   // Create a swarm with our Transport, NetworkBehaviour, and PeerID.
      SwarmBuilder::new(transp, behaviour, local_peer_id)
      .executor(Box::new(|fut| {
          tokio::spawn(fut);
      }))
      .build();

  Swarm::listen_on(
      &mut swarm,
      "/ip4/0.0.0.0/tcp/0"
          .parse()
          .expect("can get a local socket"),
  )
  .expect("swarm can be started");

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
