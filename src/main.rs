mod p2p {
  pub mod peer;
  pub mod local_data;
  pub mod local_network;
  pub mod local_swarm;
}

#[tokio::main]
async fn main() {
  pretty_env_logger::init();

  let mut peer = p2p::peer::set_up_peer().await;
  peer.handle_local_events().await
}
