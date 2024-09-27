mod p2p {
  pub mod peer;
  pub mod local_data;
  pub mod local_network;
  pub mod local_swarm;
}

/*************************************************************************************************************
                                                                  LOCAL_IO
                                                                      ↑
 STDIN ==>         PEER    ===========>   SWARM   =========> LOCAL_NETWORKBEHAVIOUR <==>  P2P NETWORK
           { LOCAL_RECEIVER } <================================  { LOCAL_SENDER }

**************************************************************************************************************/

#[tokio::main]
async fn main() {
  let mut peer = p2p::peer::set_up_peer().await;
  peer.handle_local_events().await
}
