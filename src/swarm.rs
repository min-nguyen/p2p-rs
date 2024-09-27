use libp2p::{core::transport::Boxed, swarm::{NetworkBehaviour, SwarmBuilder}, PeerId, Swarm};
use log::info;
use super::network::{RECIPE_TOPIC, RecipeBehaviour, RecipeResponse, RecipeRequest, TransmitType};


/*
    *Swarm*:
    -- Configured to a specific NetworkBehaviour, and used to broadcast messages to other peers' NetworkBehaviours.
    -- Each peer has a local Swarm.

    More generally,
    -- Manages connections created with the Transport and executes our NetworkBehaviour
    -- Used to trigger and receive events from the network
*/

/*  Create a swarm with our Transport, NetworkBehaviour, and PeerID.
    Start to listen to a local IP (port decided by the OS) using our set up. */
pub fn set_up_swarm(transp : Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>, behaviour : RecipeBehaviour, local_peer_id : PeerId)
  -> Swarm<RecipeBehaviour> {

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

pub async fn publish_response(resp: RecipeResponse, swarm: &mut Swarm<RecipeBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await;
  info!("local_swarm: Published response.")
}
pub async fn publish_request(resp: RecipeRequest, swarm: &mut Swarm<RecipeBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await;
  info!("local_swarm: Published request.")
}
async fn publish(json : String,  swarm: &mut Swarm<RecipeBehaviour> ) {
  swarm
      .behaviour_mut()
      .floodsub
      .publish(RECIPE_TOPIC.clone(), json.as_bytes());
}
